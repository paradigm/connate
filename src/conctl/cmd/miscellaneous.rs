use connate::constants::*;
use connate::ipc::*;
use connate::os::*;
use connate::types::*;

pub fn cmd_help(mut envp: Envp, config_lock_file: Option<&CStr>) -> ! {
    print(
        r#"Usage: conctl [PID | CONNATE_LOCK_PATH] COMMAND [ARGS]

conctl finds the connate daemon by checking in order:
- If optional first arg starts with digit, indicates PID
- If optional first arg starts with '.' or '/', indicates lock path
- $CONNATE_PID"#,
    );
    if let Some((_, pid)) = envp.clone().find(|(var, _)| var == &PID_ENVVAR) {
        print(" (");
        print(pid);
        println(")");
    } else {
        println(" (currently unset)");
    }

    print("- $CONNATE_LOCK_FILE");
    if let Some((_, path)) = envp.find(|(var, _)| var == &LOCK_FILE_ENVVAR) {
        print(" (");
        print(path);
        println(")");
    } else {
        println(" (currently unset)");
    }

    print("- Compiled-in config lock file");
    if let Some(path) = config_lock_file {
        print(" (");
        print(path);
        println(")");
    } else {
        println(" (currently unset)");
    }

    println(
        r#"- Assumes PID=1

For all commands which take `[services]` argument, if no services are specified,
implicitly applies to all services.  For commands which take `<services>`, one
or more services must be specified.

GENERAL QUERY COMMANDs:
s, status  [services]  Prints status information
l, list                List all services
   state   [services]  Print the current state
   target  [services]  Print the target state
p, pid     [services]  Print the Process IDs
   code    [services]  Print the last exit code
   attempt [services]  Print the number of attempts to start and stay up
   time    [services]  Print the time in the current state

DEPENDENCY QUERY COMMANDS:
needs      [services]  Print hard dependencies
wants      [services]  Print soft dependencies
conflicts  [services]  Print anti dependencies
groups     [services]  Print group members
log        [services]  Print log configuration

SET TARGET COMMANDs:
u, up      <services>  Bring up service(s) and dependencies
d, down    <services>  Bring down the service(s) and dependents
r, restart <services>  Restart the service(s)
o, once    <services>  Bring the service(s) up once (no retry)

SET TARGET AND WAIT FOR SETTLE COMMANDS:
U, UP      <services>  Bring up service(s) and dependencies
                       then wait for service state to settle
D, DOWN    <services>  Bring down the service(s) and dependents
                       then wait for service state to settle
R, RESTART <services>  Restart the service(s)
                       then wait for service state to settle
O, ONCE    <services>  Bring the service(s) up once (no retry)
                       then wait for service state to settle

MISCELLANEOUS COMMANDs:
-h, --help, help      Print this help message
P, PID                Print the Connate Process ID
x, exec [path]        Instructs Connate to re‑execute itself (usually to
                      change configuration).  Optionally give it a new
                      executable path; otherwise, it re‑uses the file path that
                      was previously used to execute it.
ready                 Notify connate that this service is ready. Called from
                      within a service process with `run = Run::Notify` to
                      signal that initialization is complete and dependencies
                      can now be fulfilled.

Output formats are intended to be both human and machine readable, allowing for
feeding one command's output back in as input.  For example:
    conctl needs sshd | xargs conctl status
or
    conctl status $(conctl needs sshd)

Additionally, one can create a process tree for Connate with the aid of pstree:
    pstree $(conctl PID)

or a specific service:
    pstree $(conctl pid sshd)
"#,
    );

    exit(0);
}

pub fn cmd_connate_pid(pid: pid_t) -> ! {
    println(pid);

    exit(0);
}

pub fn cmd_exec(mut ipc_client: IpcClient, mut argv: Argv) -> ! {
    // IPC doesn't have an explicit Some/None.
    // Empty path implies None.
    let path = argv.pop().unwrap_or(c"");
    let response = ipc_client.send_and_receive(Request::Exec(path));

    if response.cmd_return_failed() {
        println(response);
        exit(1);
    } else {
        print_color(Color::Okay, "Successfully ");
        print("updated with ");
        if path.is_empty() {
            print_color(Color::Path, "/proc/self/exe");
        } else {
            print_color(Color::Path, path);
        }
        print("\n");
        exit(0);
    }
}
