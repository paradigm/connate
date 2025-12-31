use connate::err::*;
use connate::ipc::*;
use connate::os::*;
use connate::syscall::{PollEvents, PollFd, poll};
use connate::types::*;
use connate::util::BufWriter;
use itoa::Integer; // ::MAX_STR_LEN

/// Generic helper for settle commands that set target and wait for stable states
///
/// Sets the target for all services, then blocks until each reaches a stable state.
/// Exits with error if any service reaches Failed or CannotStop states.
fn settle_generic<'a, F>(
    mut ipc_client: IpcClient,
    argv: Argv<'a>,
    connate_pid: pid_t,
    request_fn: F,
) -> !
where
    F: Fn(&'a [u8]) -> Request<'a>,
{
    if argv.is_empty() {
        abort_with_msg("No service specified");
    }

    // Calculate max name length for padding
    let mut max_name_len: usize = 0;
    for name in argv.iter() {
        max_name_len = core::cmp::max(max_name_len, name.to_bytes().len());
    }

    // Set target for all services first
    for name in argv.iter() {
        let request = request_fn(name.to_bytes());
        let response = ipc_client.send_and_receive(request);
        if response.cmd_return_failed() {
            print_color(Color::Service, name.to_bytes());
            print_color(Color::Glue, ":");
            name.to_bytes().print_padding(max_name_len + 1);
            println(response);
            exit(1);
        }
    }

    // Wait for each service to reach a stable state, sequentially
    let mut any_bad = false;

    for name in argv.iter() {
        // Print "<service>: " (state will follow)
        print_color(Color::Service, name.to_bytes());
        print_color(Color::Glue, ":");
        name.to_bytes().print_padding(max_name_len + 1);

        // Query current state
        let mut state =
            match ipc_client.send_and_receive(Request::QueryByNameState(name.to_bytes())) {
                Response::State(state) => state,
                Response::ServiceNotFound => {
                    print_color(Color::NotFound, "not-found");
                    print("\n");
                    exit(1);
                }
                _ => abort_with_msg("Unexpected response to QueryByNameState"),
            };

        // If not already stable, wait for stabilization
        //
        // TODO: How should we handle the state stabilizing to something other than the requested
        // state? It's possible another `conctl` changes the target while we're waiting for
        // stabilization.
        if !state.stable() {
            // Get settle pipe FD for this service
            let settle_fd =
                match ipc_client.send_and_receive(Request::QuerySettleFd(name.to_bytes())) {
                    Response::SettleFd(fd) => fd,
                    Response::SettleDisabled => {
                        print_color(Color::Error, "settle-disabled");
                        print("\n");
                        abort_with_msg("Settle feature is disabled in this build of connate");
                    }
                    Response::ServiceNotFound => {
                        print_color(Color::NotFound, "not-found");
                        print("\n");
                        exit(1);
                    }
                    _ => abort_with_msg("Unexpected response to QuerySettleFd"),
                };

            // Build path to /proc/<pid>/fd/<settle_fd>
            const PATH_SIZE: usize = b"/proc/".len()
                + pid_t::MAX_STR_LEN
                + "/fd/".len()
                + c_int::MAX_STR_LEN
                + b"\0".len();
            let mut buf = [0u8; PATH_SIZE];
            let mut writer = BufWriter::new(&mut buf);
            let mut itoa_buf = itoa::Buffer::new();

            writer
                .push(b"/proc/")
                .and_then(|_| writer.push(itoa_buf.format(connate_pid).as_bytes()))
                .and_then(|_| writer.push(b"/fd/"))
                .and_then(|_| writer.push(itoa_buf.format(settle_fd).as_bytes()))
                .and_then(|_| writer.push(b"\0"))
                .or_abort("buffer overflow building settle FD path");

            // Safety: We just built this buffer including the trailing null
            let settle_path: &CStr =
                unsafe { CStr::from_bytes_with_nul_unchecked(writer.as_slice()) };
            let settle_pipe_fd =
                Fd::open(settle_path, OpenFlags::O_RDONLY, 0).or_fs_abort("open", settle_path);

            // Poll until readable, then re-check state
            loop {
                let mut pollfd = PollFd {
                    fd: settle_pipe_fd.as_raw(),
                    events: PollEvents::POLLIN,
                    revents: PollEvents::empty(),
                };

                // Release IPC lock before blocking poll to allow supervisors
                // and other conctl instances to work while we're blocked
                ipc_client.unlock();

                // Poll with no timeout (-1)
                if unsafe { poll(core::slice::from_mut(&mut pollfd), -1) }
                    .is_err_and(|e| e != Errno::EINTR)
                {
                    abort_with_msg("Unable to poll() on service settle fd");
                }

                // Re-acquire lock and query state
                ipc_client.lock_quiet();

                state =
                    match ipc_client.send_and_receive(Request::QueryByNameState(name.to_bytes())) {
                        Response::State(s) => s,
                        _ => abort_with_msg("Unexpected response to QueryByNameState"),
                    };

                if state.stable() {
                    break;
                }
            }

            let _ = settle_pipe_fd.close();
        }

        // Print state
        println(state);

        // Track bad states
        if state.bad() {
            any_bad = true;
        }
    }

    exit(if any_bad { 1 } else { 0 });
}

#[inline]
pub fn cmd_settle_up(ipc_client: IpcClient, argv: Argv, connate_pid: pid_t) -> ! {
    settle_generic(ipc_client, argv, connate_pid, Request::SetTargetUp)
}

#[inline]
pub fn cmd_settle_down(ipc_client: IpcClient, argv: Argv, connate_pid: pid_t) -> ! {
    settle_generic(ipc_client, argv, connate_pid, Request::SetTargetDown)
}

#[inline]
pub fn cmd_settle_restart(ipc_client: IpcClient, argv: Argv, connate_pid: pid_t) -> ! {
    settle_generic(ipc_client, argv, connate_pid, Request::SetTargetRestart)
}

#[inline]
pub fn cmd_settle_once(ipc_client: IpcClient, argv: Argv, connate_pid: pid_t) -> ! {
    settle_generic(ipc_client, argv, connate_pid, Request::SetTargetOnce)
}
