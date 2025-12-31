use crate::session::*;
use connate::err::*;
use connate::internal_api::*;
use connate::os::*;
use connate::types::*;

pub fn handle_signal<const N: usize>(
    signalfd: &mut SignalFd,
    svcs: &mut [Service; N],
    shutting_down: &mut bool,
    session_fd: &mut SessionFd,
) {
    match signalfd.read_signal() {
        // Shutdown request
        Ok(Signal::SIGINT) | Ok(Signal::SIGTERM) => {
            if getpid() == 1 {
                return; // Ignore if we're PID=1
            }
            for svc in svcs.iter_mut() {
                svc.target = Target::Down;
                svc.dirty = true;
            }
            *shutting_down = true;
        }
        // Config reload request
        Ok(Signal::SIGHUP) => {
            if session_fd.save(svcs).is_ok() {
                let _ = exec_self();
            }
        }
        // Child process died
        Ok(Signal::SIGCHLD) => handle_sigchld(svcs),
        // SIGKILL cannot be caught/handled. If we receive it, process just dies.
        Ok(Signal::SIGKILL) => unsafe { core::hint::unreachable_unchecked() },
        // Ignore unknown signals
        Ok(Signal::UNRECOGNIZED) | Err(_) => {}
    }
}

fn handle_sigchld<const N: usize>(mut svcs: &mut [Service; N]) {
    // Loop over all children that died:
    // - If we recognize the child as a service, tag service as died for state transition logic
    // - If we don't recognize it, just reap
    loop {
        // Wait for any child (-1) with WNOHANG
        match waitpid(-1, WaitPidOptions::WNOHANG) {
            Err(Errno::ECHILD) => break, // No (more) dead children
            Err(e) => Err(e).or_abort("Unable to waitpid()"),
            Ok((0, _)) => break, // No (more) dead children
            Ok((pid, status)) => {
                let exit_code = if wifexited(status) {
                    wexitstatus(status)
                } else if wifsignaled(status) {
                    128 + wtermsig(status)
                } else {
                    // Stopped or continued, *not* killed
                    // Ignore
                    continue;
                };

                if let Some(svc) = svcs.find_by_pid_mut(pid) {
                    svc.pid = None;
                    svc.exit_code = Some(exit_code);
                    svc.dirty = true;
                    if let Some((fd_read, fd_write)) = svc.stdin_pipe.take() {
                        let _ = fd_read.close();
                        let _ = fd_write.close();
                    }
                } else if let Some(svc) = svcs.find_by_supervisor_pid_mut(pid) {
                    svc.supervisor_pid = None;
                    // If supervisor died, we can't reliably track the service's process.
                    // Don't try to.  Assume it died.
                    svc.pid = None;
                    svc.supervisor_pid = None;
                    svc.exit_code = Some(exit_code);
                    svc.dirty = true;
                    if let Some((fd_read, fd_write)) = svc.stdin_pipe.take() {
                        let _ = fd_read.close();
                        let _ = fd_write.close();
                    }
                }
                // Other else branch is an unexpected child.  We just reaped it; nothing else to
                // do.
            }
        }
    }
}
