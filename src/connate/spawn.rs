use connate::constants::*;
use connate::err::*;
use connate::internal_api::*;
use connate::ipc::*;
use connate::os::*;
use connate::syscall::{PollEvents, PollFd, poll};
use connate::types::*;
use connate::util::BufWriter;
use itoa::Integer;

pub trait Spawn {
    fn spawn_setting_up(&mut self, logger_write_fd: Option<Fd>) -> Result<(), Errno>;
    fn spawn_run(&mut self, logger_write_fd: Option<Fd>) -> Result<(), Errno>;
    fn spawn_cleaning_up(&mut self, logger_write_fd: Option<Fd>) -> Result<(), Errno>;
}

impl Spawn for Service {
    fn spawn_setting_up(&mut self, logger_write_fd: Option<Fd>) -> Result<(), Errno> {
        if matches!(self.cfg.setup, Run::None) {
            return Ok(());
        }
        if self.cfg.stop_all_children {
            spawn_supervised(self, &self.cfg.setup, logger_write_fd, false)
        } else {
            spawn_direct(self, &self.cfg.setup, logger_write_fd)
        }
    }

    fn spawn_run(&mut self, logger_write_fd: Option<Fd>) -> Result<(), Errno> {
        if matches!(self.cfg.run, Run::None) {
            return Ok(());
        }
        let notify_daemonize = matches!(self.cfg.ready, Ready::Daemonize);
        if self.cfg.stop_all_children || notify_daemonize {
            spawn_supervised(self, &self.cfg.run, logger_write_fd, notify_daemonize)
        } else {
            spawn_direct(self, &self.cfg.run, logger_write_fd)
        }
    }

    fn spawn_cleaning_up(&mut self, logger_write_fd: Option<Fd>) -> Result<(), Errno> {
        if matches!(self.cfg.cleanup, Run::None) {
            return Ok(());
        }
        if self.cfg.stop_all_children {
            spawn_supervised(self, &self.cfg.cleanup, logger_write_fd, false)
        } else {
            spawn_direct(self, &self.cfg.cleanup, logger_write_fd)
        }
    }
}

/// Direct spawn: fork and exec without supervisor
fn spawn_direct(svc: &mut Service, run: &Run, logger_write_fd: Option<Fd>) -> Result<(), Errno> {
    let log_overwrite = match run {
        Run::Exec { log_overwrite, .. } | Run::Fn { log_overwrite, .. } => *log_overwrite,
        Run::None => false,
    };

    let pid = match fork()? {
        ForkResult::Parent(pid) => pid,
        ForkResult::Child => {
            // Child process
            if setup_process(svc, logger_write_fd, log_overwrite).is_err() {
                exit(1);
            }
            execute_run(run);
            // execute_run never returns on success (exec or exit)
        }
    };

    svc.pid = Some(pid);
    Ok(())
}

/// Supervised spawn: fork supervisor which manages service process
fn spawn_supervised(
    svc: &mut Service,
    run: &Run,
    logger_write_fd: Option<Fd>,
    notify_daemonize: bool,
) -> Result<(), Errno> {
    let pid = match fork()? {
        ForkResult::Parent(pid) => pid,
        ForkResult::Child => {
            // Supervisor process
            run_supervisor(svc, run, logger_write_fd, notify_daemonize);
            // run_supervisor never returns
        }
    };

    svc.supervisor_pid = Some(pid);
    Ok(())
}

/// Run the supervisor process
///
/// This function never returns - it either exits or aborts.
fn run_supervisor(
    svc: &Service,
    run: &Run,
    logger_write_fd: Option<Fd>,
    notify_daemonize: bool,
) -> ! {
    if set_process_name(c"supervisor").is_err() {
        exit(1);
    }

    // Close connate's internal FDs before creating IpcClient
    // (IpcClient opens its own FDs via /proc/<pid>/fd/ paths)
    close_inherited_fds();

    // Become subreaper for all descendants
    if set_child_subreaper().is_err() {
        exit(1);
    }

    // Create signalfd for signals (SIGCHLD for child reaping, SIGTERM for cleanup requests)
    let mut signalfd = match SignalFd::new() {
        Ok(fd) => fd,
        Err(_) => exit(1),
    };

    // Get connate's PID to create IPC client
    let connate_pid = getppid();
    let mut ipc_client = IpcClient::from_pid(connate_pid);

    // Fork the actual service process
    let log_overwrite = match run {
        Run::Exec { log_overwrite, .. } | Run::Fn { log_overwrite, .. } => *log_overwrite,
        Run::None => false,
    };

    let service_pid = match fork() {
        Ok(ForkResult::Parent(pid)) => pid,
        Ok(ForkResult::Child) => {
            // Service child process
            if setup_process(svc, logger_write_fd, log_overwrite).is_err() {
                exit(1);
            }
            execute_run(run);
            // execute_run never returns on success
        }
        Err(_) => exit(1),
    };

    // Notify connate that service is starting
    ipc_client.lock_quiet();
    let _ = ipc_client.send_and_receive(Request::ServiceStarting(service_pid, svc.cfg.name));
    ipc_client.unlock();

    let mut main_pid = service_pid;
    let stop_all_children = svc.cfg.stop_all_children;

    // Main supervisor loop
    let mut pollfd = PollFd {
        fd: signalfd.as_raw(),
        events: PollEvents::POLLIN,
        revents: PollEvents::empty(),
    };

    loop {
        // Poll for signals
        let poll_result = unsafe { poll(core::slice::from_mut(&mut pollfd), -1) };
        if poll_result.is_err() {
            continue; // Interrupted, retry
        }

        if !pollfd.revents.contains(PollEvents::POLLIN) {
            continue;
        }

        // Read signal
        let signal = match signalfd.read_signal() {
            Ok(sig) => sig,
            Err(_) => continue,
        };

        match signal {
            Signal::SIGCHLD => {
                reap_children(
                    &mut main_pid,
                    notify_daemonize,
                    stop_all_children,
                    svc.cfg.name,
                    &mut ipc_client,
                );
            }
            Signal::SIGTERM => {
                let _ = kill(main_pid, Signal::SIGKILL);
                let exit_code = match waitpid(main_pid, WaitPidOptions::empty()) {
                    Ok((_, status)) => exit_code_from_status(status),
                    Err(_) => 1,
                };
                if stop_all_children {
                    kill_all_children();
                }
                exit(exit_code);
            }
            _ => {}
        }
    }
}

/// Set up child process before exec
fn setup_process(
    svc: &Service,
    logger_write_fd: Option<Fd>,
    log_overwrite: bool,
) -> Result<(), Errno> {
    // Close connate's internal FDs that we inherited
    close_inherited_fds();

    // Create new session (detach from controlling terminal)
    let _ = setsid();

    // Setup logging
    setup_logging(svc, logger_write_fd, log_overwrite)?;

    // Change directory if configured
    if let Some(path) = svc.cfg.chdir {
        chdir(path)?;
    }

    // Drop privileges (setgid must come before setuid)
    if let Some(gid) = svc.cfg.gid {
        setgid(gid)?;
    }
    if let Some(uid) = svc.cfg.uid {
        setuid(uid)?;
    }

    // Prevent gaining new privileges
    if svc.cfg.no_new_privs {
        set_no_new_privs()?;
    }

    // Unblock signals so child can receive them normally
    unblock_all_signals()?;

    Ok(())
}

/// Set up logging for child process
fn setup_logging(
    svc: &Service,
    logger_write_fd: Option<Fd>,
    log_overwrite: bool,
) -> Result<(), Errno> {
    match &svc.cfg.log {
        Log::None => {
            // Redirect stdout/stderr to /dev/null
            let dev_null = Fd::open(c"/dev/null", OpenFlags::O_WRONLY, 0)?;
            dev_null.dup(STDOUT.as_raw(), OpenFlags::empty())?;
            dev_null.dup(STDERR.as_raw(), OpenFlags::empty())?;
            dev_null.close()?;
        }
        Log::Inherit => {
            // Do nothing, inherit parent's stdout/stderr
        }
        Log::File { filepath, mode } => {
            let flags = OpenFlags::O_WRONLY | OpenFlags::O_CREAT;
            let flags = if log_overwrite {
                flags | OpenFlags::O_TRUNC
            } else {
                flags | OpenFlags::O_APPEND
            };
            let log_fd = Fd::open(filepath, flags, *mode)?;
            log_fd.dup(STDOUT.as_raw(), OpenFlags::empty())?;
            log_fd.dup(STDERR.as_raw(), OpenFlags::empty())?;
            log_fd.close()?;
        }
        Log::Service(_) => {
            // Use the provided logger pipe
            if let Some(fd) = logger_write_fd {
                fd.dup(STDOUT.as_raw(), OpenFlags::empty())?;
                fd.dup(STDERR.as_raw(), OpenFlags::empty())?;
                fd.close()?;
            }
        }
    }
    Ok(())
}

/// Execute a Run variant
///
/// This function never returns on success (exec replaces the process or exit is called).
fn execute_run(run: &Run) -> ! {
    match run {
        Run::None => exit(0),
        Run::Exec {
            pathname,
            argv,
            envp,
            ..
        } => {
            // execve never returns on success
            let _ = unsafe { connate::syscall::execve(pathname, *argv, *envp) };
            exit(1);
        }
        Run::Fn { f, .. } => match f() {
            Ok(()) => exit(0),
            Err(errno) => exit(errno.into_raw() as c_int),
        },
    }
}

/// Read first child PID from /proc/self/task/{pid}/children
fn read_first_child_pid() -> Option<pid_t> {
    let pid = getpid();

    // Build path: /proc/self/task/{pid}/children
    const PATH_BUF_SIZE: usize =
        b"/proc/self/task/".len() + pid_t::MAX_STR_LEN + b"/children\0".len();
    let mut path_buf = [0u8; PATH_BUF_SIZE];
    let mut writer = BufWriter::new(&mut path_buf);

    let mut itoa_buf = itoa::Buffer::new();
    let pid_str = itoa_buf.format(pid);

    writer.push(b"/proc/self/task/").ok()?;
    writer.push(pid_str.as_bytes()).ok()?;
    writer.push(b"/children\0").ok()?;

    // Safety: We just built this buffer with a null terminator
    let path = unsafe { CStr::from_bytes_with_nul_unchecked(writer.as_slice()) };

    let fd = Fd::open(path, OpenFlags::O_RDONLY, 0).ok()?;
    let mut buf = [0u8; pid_t::MAX_STR_LEN + 1];
    let bytes_read = fd.read(&mut buf).ok()?;
    fd.close().ok()?;

    // Parse first space-separated PID
    let data = buf.get(..bytes_read)?;

    // Find end of first PID (space or newline)
    let end = data
        .iter()
        .position(|&b| b == b' ' || b == b'\n')
        .unwrap_or(data.len());

    parse_pid(data.get(..end)?)
}

/// Parse a PID from ASCII bytes
fn parse_pid(bytes: &[u8]) -> Option<pid_t> {
    if bytes.is_empty() {
        return None;
    }

    let mut result: pid_t = 0;
    for &byte in bytes {
        if !byte.is_ascii_digit() {
            return None;
        }
        result = result.checked_mul(10)?;
        result = result.checked_add((byte - b'0') as pid_t)?;
    }
    Some(result)
}

/// Close connate's fixed FDs that are inherited by forked children
///
/// These FDs are specific to connate's operation and should not be leaked to service processes.
fn close_inherited_fds() {
    let _ = Fd::from_raw(FD_SESSION_STATE).close();
    let _ = Fd::from_raw(FD_SIGNAL).close();
    let _ = Fd::from_raw(FD_LOCK_FILE).close();
    let _ = Fd::from_raw(FD_REQ_READ).close();
    let _ = Fd::from_raw(FD_REQ_WRITE).close();
    let _ = Fd::from_raw(FD_RESP_READ).close();
    let _ = Fd::from_raw(FD_RESP_WRITE).close();
}

/// Reap children and handle main process exit
fn reap_children(
    main_pid: &mut pid_t,
    notify_daemonize: bool,
    stop_all_children: bool,
    svc_name: &'static [u8],
    ipc_client: &mut IpcClient,
) {
    loop {
        let (reaped_pid, status) = match waitpid(-1, WaitPidOptions::WNOHANG) {
            Ok((0, _)) => return,
            Ok((pid, status)) => (pid, status),
            Err(_) => return,
        };

        if reaped_pid != *main_pid {
            continue; // Some other child died, keep reaping
        }

        let exit_code = exit_code_from_status(status);

        if notify_daemonize && let Some(new_pid) = read_first_child_pid() {
            ipc_client.lock_quiet();
            let _ = ipc_client.send_and_receive(Request::DaemonReady(new_pid, svc_name));
            ipc_client.unlock();
            *main_pid = new_pid;
        } else {
            if stop_all_children {
                kill_all_children();
            }
            exit(exit_code);
        }
    }
}

/// Extract exit code from waitpid status
fn exit_code_from_status(status: c_int) -> c_int {
    if wifexited(status) {
        wexitstatus(status)
    } else if wifsignaled(status) {
        128 + wtermsig(status)
    } else {
        1
    }
}

/// Kill all children and wait for them to die
fn kill_all_children() {
    while let Some(pid) = read_first_child_pid() {
        // Kill the process group (negative pid)
        let _ = kill(-pid, Signal::SIGKILL);

        // Reap any zombies
        loop {
            match waitpid(-1, WaitPidOptions::WNOHANG) {
                Ok((0, _)) => break,
                Ok((_, _)) => continue,
                Err(Errno::ECHILD) => break,
                Err(_) => break,
            }
        }
    }
}
