use connate::ipc::*;
use connate::os::*;

pub fn cmd_status(mut ipc_client: IpcClient, mut argv: Argv) -> ! {
    use Color::*;
    let mut failed = false;

    match argv.pop() {
        None => {
            // - Query all services
            // - By index, since we don't have the names up-front
            // - Print service name to associate data with service
            let mut max_name_len: usize = 0;
            let mut status_widths = StatusWidths::default();
            let mut service_count: usize = 0;

            // First pass: find field widths for padding
            for i in 0.. {
                match ipc_client.send_and_receive(Request::QueryByIndexName(i)) {
                    Response::Name(name) => {
                        max_name_len = core::cmp::max(max_name_len, name.len());
                        let response = ipc_client.send_and_receive(Request::QueryByIndexStatus(i));
                        if let Some((s, t, p, r)) = response.status_field_lens() {
                            status_widths.update(s, t, p, r);
                        }
                        service_count += 1;
                    }
                    Response::ServiceNotFound => break,
                    response => {
                        failed |= response.cmd_return_failed();
                        break;
                    }
                }
            }

            // Second pass: print with padding
            for i in 0..service_count {
                match ipc_client.send_and_receive(Request::QueryByIndexName(i)) {
                    Response::Name(name) => {
                        print_color(Service, name);
                        print_color(Glue, ":");
                        name.print_padding(max_name_len + 1);
                        let response = ipc_client.send_and_receive(Request::QueryByIndexStatus(i));
                        failed |= response.cmd_return_failed();
                        response.print_status_padded(&status_widths);
                        print("\n");
                    }
                    Response::ServiceNotFound => break,
                    response => {
                        failed |= response.cmd_return_failed();
                        break;
                    }
                }
            }
        }
        Some(name) if argv.is_empty() => {
            // - Query single service
            // - By name, since we have service name
            // - Don't print service name, since it's obvious from context and simplifies scripting
            let response = ipc_client.send_and_receive(Request::QueryByNameStatus(name.to_bytes()));
            failed |= response.cmd_return_failed();
            println(response);
        }
        Some(first) => {
            // - Query all services
            // - By name, since we have the service names
            // - Print service name to associate data with service

            // Calculate max name length from argv
            let mut max_name_len = first.to_bytes().len();
            for name in argv.iter() {
                max_name_len = core::cmp::max(max_name_len, name.to_bytes().len());
            }

            // First pass: find field widths for padding
            let mut status_widths = StatusWidths::default();
            let first_response =
                ipc_client.send_and_receive(Request::QueryByNameStatus(first.to_bytes()));
            if let Some((s, t, p, r)) = first_response.status_field_lens() {
                status_widths.update(s, t, p, r);
            }
            for name in argv.iter() {
                let response =
                    ipc_client.send_and_receive(Request::QueryByNameStatus(name.to_bytes()));
                if let Some((s, t, p, r)) = response.status_field_lens() {
                    status_widths.update(s, t, p, r);
                }
            }

            // Second pass: print with padding
            let first_response =
                ipc_client.send_and_receive(Request::QueryByNameStatus(first.to_bytes()));
            failed |= first_response.cmd_return_failed();
            print_color(Service, first.to_bytes());
            print_color(Glue, ":");
            first.to_bytes().print_padding(max_name_len + 1);
            first_response.print_status_padded(&status_widths);
            print("\n");

            for name in argv.iter() {
                let response =
                    ipc_client.send_and_receive(Request::QueryByNameStatus(name.to_bytes()));
                failed |= response.cmd_return_failed();
                print_color(Service, name.to_bytes());
                print_color(Glue, ":");
                name.to_bytes().print_padding(max_name_len + 1);
                response.print_status_padded(&status_widths);
                print("\n");
            }
        }
    }

    exit(if failed { 1 } else { 0 });
}

pub fn cmd_list(mut ipc_client: IpcClient) -> ! {
    let mut failed = false;
    for i in 0.. {
        match ipc_client.send_and_receive(Request::QueryByIndexName(i)) {
            Response::Name(name) => {
                print_color(Color::Service, name);
                print("\n");
            }
            Response::ServiceNotFound => break,
            response => {
                failed |= response.cmd_return_failed();
                break;
            }
        }
    }

    exit(if failed { 1 } else { 0 });
}

/// Generic helper for commands that query a single field per service
fn query_field<'a, FIdx, FName>(
    mut ipc_client: IpcClient,
    mut argv: Argv<'a>,
    by_index: FIdx,
    by_name: FName,
) -> !
where
    FIdx: Fn(usize) -> Request<'static>,
    FName: Fn(&'a [u8]) -> Request<'a>,
{
    use Color::*;
    let mut failed = false;

    match argv.pop() {
        None => {
            // - Query all services
            // - By index, since we don't have the names up-front
            // - Print service name to associate data with service
            let mut max_name_len: usize = 0;
            let mut service_count: usize = 0;

            // First pass: find max name length
            for i in 0.. {
                match ipc_client.send_and_receive(Request::QueryByIndexName(i)) {
                    Response::Name(name) => {
                        max_name_len = core::cmp::max(max_name_len, name.len());
                        service_count = i + 1;
                    }
                    Response::ServiceNotFound => break,
                    response => {
                        failed |= response.cmd_return_failed();
                        break;
                    }
                }
            }

            // Second pass: print with padding
            for i in 0..service_count {
                match ipc_client.send_and_receive(Request::QueryByIndexName(i)) {
                    Response::Name(name) => {
                        print_color(Service, name);
                        print_color(Glue, ":");
                        name.print_padding(max_name_len + 1);
                        let response = ipc_client.send_and_receive(by_index(i));
                        failed |= response.cmd_return_failed();
                        println(response);
                    }
                    Response::ServiceNotFound => break,
                    response => {
                        failed |= response.cmd_return_failed();
                        break;
                    }
                }
            }
        }
        Some(name) if argv.is_empty() => {
            // - Query single service
            // - By name, since we have service name
            // - Don't print service name, since it's obvious from context and simplifies scripting
            let response = ipc_client.send_and_receive(by_name(name.to_bytes()));
            failed |= response.cmd_return_failed();
            println(response);
        }
        Some(first) => {
            // - Query all services
            // - By name, since we have the service names
            // - Print service name to associate data with service

            // Query multiple services by name (with name prefix)
            // Calculate max name length from argv
            let mut max_name_len = first.to_bytes().len();
            for name in argv.iter() {
                max_name_len = core::cmp::max(max_name_len, name.to_bytes().len());
            }

            // Print with padding
            let first_response = ipc_client.send_and_receive(by_name(first.to_bytes()));
            failed |= first_response.cmd_return_failed();
            print_color(Service, first.to_bytes());
            print_color(Glue, ":");
            first.to_bytes().print_padding(max_name_len + 1);
            println(first_response);

            for name in argv.iter() {
                print_color(Service, name.to_bytes());
                print_color(Glue, ":");
                name.to_bytes().print_padding(max_name_len + 1);
                let response = ipc_client.send_and_receive(by_name(name.to_bytes()));
                failed |= response.cmd_return_failed();
                println(response);
            }
        }
    }

    exit(if failed { 1 } else { 0 });
}

#[inline]
pub fn cmd_state(ipc_client: IpcClient, argv: Argv) -> ! {
    query_field(
        ipc_client,
        argv,
        Request::QueryByIndexState,
        Request::QueryByNameState,
    )
}

#[inline]
pub fn cmd_target(ipc_client: IpcClient, argv: Argv) -> ! {
    query_field(
        ipc_client,
        argv,
        Request::QueryByIndexTarget,
        Request::QueryByNameTarget,
    )
}

#[inline]
pub fn cmd_pid(ipc_client: IpcClient, argv: Argv) -> ! {
    query_field(
        ipc_client,
        argv,
        Request::QueryByIndexPid,
        Request::QueryByNamePid,
    )
}

#[inline]
pub fn cmd_code(ipc_client: IpcClient, argv: Argv) -> ! {
    query_field(
        ipc_client,
        argv,
        Request::QueryByIndexExitCode,
        Request::QueryByNameExitCode,
    )
}

#[inline]
pub fn cmd_attempt(ipc_client: IpcClient, argv: Argv) -> ! {
    query_field(
        ipc_client,
        argv,
        Request::QueryByIndexAttemptCount,
        Request::QueryByNameAttemptCount,
    )
}

#[inline]
pub fn cmd_time(ipc_client: IpcClient, argv: Argv) -> ! {
    query_field(
        ipc_client,
        argv,
        Request::QueryByIndexTime,
        Request::QueryByNameTime,
    )
}

#[inline]
pub fn cmd_log(ipc_client: IpcClient, argv: Argv) -> ! {
    query_field(
        ipc_client,
        argv,
        Request::QueryByIndexLog,
        Request::QueryByNameLog,
    )
}
