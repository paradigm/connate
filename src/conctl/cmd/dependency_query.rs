use connate::constants::*;
use connate::err::*;
use connate::ipc::*;
use connate::os::*;
use connate::util::BufWriter;

/// Generic helper for commands that query dependencies (needs, wants, conflicts, groups)
pub fn query_dependencies<F>(mut ipc_client: IpcClient, mut argv: Argv, request_fn: F) -> !
where
    F: for<'a> Fn(&'a [u8], usize) -> Request<'a>,
{
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
            let mut name_buf = [0u8; MSG_SVC_NAME_SIZE];
            for i in 0..service_count {
                match ipc_client.send_and_receive(Request::QueryByIndexName(i)) {
                    Response::Name(name) => {
                        // We can't re-use the provided name, as it's a buffer in `ipc_client`
                        // that's holding it as mut.
                        //
                        // Copy it out.
                        let mut writer = BufWriter::new(&mut name_buf);
                        writer
                            .push(name)
                            .or_abort("Unable to deserialize response from connate");
                        let name_len = name.len();
                        let name = name_buf
                            .get(..name_len)
                            .ok_or(Errno::EINVAL)
                            .or_abort("Unable to deserialize response from connate");

                        print_color(Color::Service, name);
                        print_color(Color::Glue, ":");
                        name.print_padding(max_name_len + 1);
                        failed |= query_deps_for_service(&mut ipc_client, name, &request_fn);
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
            failed |= query_deps_for_service(&mut ipc_client, name.to_bytes(), &request_fn);
        }
        Some(first) => {
            // - Query all services
            // - By name, since we have the service names
            // - Print service name to associate data with service
            let mut max_name_len = first.to_bytes().len();
            for name in argv.iter() {
                max_name_len = core::cmp::max(max_name_len, name.to_bytes().len());
            }

            // Print with padding
            print_color(Color::Service, first);
            print_color(Color::Glue, ":");
            first.to_bytes().print_padding(max_name_len + 1);
            failed |= query_deps_for_service(&mut ipc_client, first.to_bytes(), &request_fn);

            for name in argv.iter() {
                print_color(Color::Service, name);
                print_color(Color::Glue, ":");
                name.to_bytes().print_padding(max_name_len + 1);
                failed |= query_deps_for_service(&mut ipc_client, name.to_bytes(), &request_fn);
            }
        }
    }

    exit(if failed { 1 } else { 0 });
}

/// Helper to query and print dependencies for a single service
/// Returns "failed": true if service not found, false if service exists
fn query_deps_for_service<F>(ipc_client: &mut IpcClient, name: &[u8], request_fn: &F) -> bool
where
    F: for<'a> Fn(&'a [u8], usize) -> Request<'a>,
{
    let mut first = true;

    for dep_i in 0.. {
        match ipc_client.send_and_receive(request_fn(name, dep_i)) {
            Response::Name(dep_name) => {
                if !first {
                    print(" ");
                }
                first = false;
                print_color(Color::Service, dep_name);
            }
            Response::FieldIsNone => {
                // None means no more dependencies (or no dependency at this index)
                break;
            }
            Response::ServiceNotFound => {
                // ServiceNotFound means the service being queried doesn't exist
                print(Response::ServiceNotFound);
                print("\n");
                return true;
            }
            response => {
                // Unexpected error (Failed, InvalidRequest, etc.)
                print(response);
                print("\n");
                return true;
            }
        }
    }
    print("\n");

    // Service exists (no ServiceNotFound response)
    false
}

#[inline]
pub fn cmd_needs(ipc_client: IpcClient, argv: Argv) -> ! {
    query_dependencies(ipc_client, argv, |name, idx| Request::QueryNeeds(idx, name))
}

#[inline]
pub fn cmd_wants(ipc_client: IpcClient, argv: Argv) -> ! {
    query_dependencies(ipc_client, argv, |name, idx| Request::QueryWants(idx, name))
}

#[inline]
pub fn cmd_conflicts(ipc_client: IpcClient, argv: Argv) -> ! {
    query_dependencies(ipc_client, argv, |name, idx| {
        Request::QueryConflicts(idx, name)
    })
}

#[inline]
pub fn cmd_groups(ipc_client: IpcClient, argv: Argv) -> ! {
    query_dependencies(ipc_client, argv, |name, idx| {
        Request::QueryGroups(idx, name)
    })
}
