use connate::err::*;
use connate::ipc::*;
use connate::os::*;

/// Generic helper for commands that set the target of one or more services
fn set_target_generic<'a, F>(
    mut ipc_client: IpcClient,
    argv: Argv<'a>,
    request_fn: F,
    target: &str,
) -> !
where
    F: Fn(&'a [u8]) -> Request<'a>,
{
    let mut failed = false;

    if argv.is_empty() {
        abort_with_msg("No service specified");
    }

    // Calculate max name length for padding
    let mut max_name_len: usize = 0;
    for name in argv.iter() {
        max_name_len = core::cmp::max(max_name_len, name.to_bytes().len());
    }

    for name in argv.iter() {
        let name = name.to_bytes();
        let request = request_fn(name);
        let response = ipc_client.send_and_receive(request);

        print_color(Color::Service, name);
        print_color(Color::Glue, ":");
        name.print_padding(max_name_len + 1);

        if response.cmd_return_failed() {
            failed = true;
            println(response);
        } else {
            print("set target ");
            print(target);
            print("\n");
        }
    }

    exit(if failed { 1 } else { 0 });
}

#[inline]
pub fn cmd_up(ipc_client: IpcClient, argv: Argv) -> ! {
    set_target_generic(ipc_client, argv, Request::SetTargetUp, "up")
}

#[inline]
pub fn cmd_down(ipc_client: IpcClient, argv: Argv) -> ! {
    set_target_generic(ipc_client, argv, Request::SetTargetDown, "down")
}

#[inline]
pub fn cmd_restart(ipc_client: IpcClient, argv: Argv) -> ! {
    set_target_generic(ipc_client, argv, Request::SetTargetRestart, "restart")
}

#[inline]
pub fn cmd_once(ipc_client: IpcClient, argv: Argv) -> ! {
    set_target_generic(ipc_client, argv, Request::SetTargetOnce, "once")
}
