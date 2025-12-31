mod dependency_query;
mod general_query;
mod miscellaneous;
mod ready;
mod set_target;
mod settle;

pub use dependency_query::*;
pub use general_query::*;
pub use miscellaneous::*;
pub use ready::*;
pub use set_target::*;
pub use settle::*;

use connate::constants::*;
use connate::err::*;
use connate::ipc::IpcClient;
use connate::os::*;
use connate::types::*;

pub enum Cmd<'a> {
    Help(Envp<'a>, Option<&'a CStr>),
    ConnatePid(pid_t),
    Exec(IpcClient, Argv<'a>),
    Status(IpcClient, Argv<'a>),
    List(IpcClient),
    State(IpcClient, Argv<'a>),
    Target(IpcClient, Argv<'a>),
    Code(IpcClient, Argv<'a>),
    Pid(IpcClient, Argv<'a>),
    Attempt(IpcClient, Argv<'a>),
    Time(IpcClient, Argv<'a>),
    Needs(IpcClient, Argv<'a>),
    Wants(IpcClient, Argv<'a>),
    Conflicts(IpcClient, Argv<'a>),
    Groups(IpcClient, Argv<'a>),
    Log(IpcClient, Argv<'a>),
    Up(IpcClient, Argv<'a>),
    Down(IpcClient, Argv<'a>),
    Restart(IpcClient, Argv<'a>),
    Once(IpcClient, Argv<'a>),
    SettleUp(IpcClient, Argv<'a>, pid_t),
    SettleDown(IpcClient, Argv<'a>, pid_t),
    SettleRestart(IpcClient, Argv<'a>, pid_t),
    SettleOnce(IpcClient, Argv<'a>, pid_t),
    Ready(IpcClient, pid_t),
}

impl<'a> Cmd<'a> {
    pub fn new(mut argv: Argv<'a>, mut envp: Envp<'a>, config_lock_path: Option<&'a CStr>) -> Self {
        // The CLI format is:
        //
        // conctl [PID | CONNATE_LOCK_PATH] cmd [ARGS]
        //
        // The optional first argument determines how to locate the connate daemon:
        // - If it starts with a digit (0-9), it is interpreted as the connate PID
        // - If it starts with '.' or '/', it is interpreted as the lock file path
        //
        // If neither PID nor path is provided, conctl resolves the PID with this priority:
        // 1. $CONNATE_PID environment variable
        // 2. $CONNATE_LOCK_FILE environment variable (read PID from lock)
        // 3. Compiled-in config lock file (read PID from lock)
        // 4. Default to PID 1 (init mode)

        // Ignore argv[0]
        let _ = argv.pop();

        // Find cmd:
        // - If first arg starts with digit, treat as PID
        // - If first arg starts with '.' or '/', treat as lock path
        // - Otherwise treat as cmd
        let arg = argv.pop().or_abort("No cmd specified.  See `--help`");

        let (cli_pid, lock_path, cmd) = match arg.to_bytes().first() {
            Some(b'0'..=b'9') => (Some(arg), None, argv.pop()),
            Some(&b'.') | Some(&b'/') => (None, Some(arg), argv.pop()),
            _ => (None, None, Some(arg)),
        };
        let cmd_str = cmd.or_abort("No cmd specified.  See `--help`");

        // Handle any cmds that don't require any resources at this point
        match cmd_str.to_bytes() {
            b"-" | b"-h" | b"--help" | b"help" => return Self::Help(envp, config_lock_path),
            _ => {}
        }

        // Determine connate's PID:
        // 1. CLI PID argument (if digit)
        // 2. CLI lock path (if starts with . or /)
        // 3. $CONNATE_PID env var
        // 4. $CONNATE_LOCK_FILE env var
        // 5. config lock file
        // 6. default to PID 1
        let pid: pid_t = if let Some(pid_arg) = cli_pid {
            pid_arg.parse_pid().or_abort("invalid PID argument")
        } else if let Some(path) = lock_path {
            get_pid_from_lock(path)
        } else if let Some((_, pid_str)) = envp.clone().find(|(var, _)| var == &PID_ENVVAR) {
            pid_str.parse_pid().or_abort("$CONNATE_PID value invalid")
        } else if let Some((_, env_path)) = envp.find(|(var, _)| var == &LOCK_FILE_ENVVAR) {
            get_pid_from_lock(env_path)
        } else if let Some(config_path) = config_lock_path {
            get_pid_from_lock(config_path)
        } else {
            // No lock file configured, default to PID 1 (init mode)
            1
        };

        // Handle any cmds that don't require IPC at this point
        match cmd_str.to_bytes() {
            b"PID" | b"P" => return Self::ConnatePid(pid),
            _ => {}
        }

        // All remaining valid cmds require an IPC connection to connate
        let mut ipc_client = IpcClient::from_pid(pid);
        ipc_client.lock_with_warning();

        match cmd_str.to_bytes() {
            b"exec" | b"x" => Self::Exec(ipc_client, argv),
            b"status" | b"s" => Self::Status(ipc_client, argv),
            b"list" | b"l" => Self::List(ipc_client),
            b"state" => Self::State(ipc_client, argv),
            b"target" => Self::Target(ipc_client, argv),
            b"pid" | b"p" => Self::Pid(ipc_client, argv),
            b"code" => Self::Code(ipc_client, argv),
            b"attempt" => Self::Attempt(ipc_client, argv),
            b"time" => Self::Time(ipc_client, argv),
            b"needs" => Self::Needs(ipc_client, argv),
            b"wants" => Self::Wants(ipc_client, argv),
            b"conflicts" => Self::Conflicts(ipc_client, argv),
            b"groups" => Self::Groups(ipc_client, argv),
            b"log" => Self::Log(ipc_client, argv),
            b"up" | b"u" => Self::Up(ipc_client, argv),
            b"down" | b"d" => Self::Down(ipc_client, argv),
            b"restart" | b"r" => Self::Restart(ipc_client, argv),
            b"once" | b"o" => Self::Once(ipc_client, argv),
            b"UP" | b"U" => Self::SettleUp(ipc_client, argv, pid),
            b"DOWN" | b"D" => Self::SettleDown(ipc_client, argv, pid),
            b"RESTART" | b"R" => Self::SettleRestart(ipc_client, argv, pid),
            b"ONCE" | b"O" => Self::SettleOnce(ipc_client, argv, pid),
            b"ready" => Self::Ready(ipc_client, pid),
            _ => abort_with_msg("Invalid cmd.  See `--help`"),
        }
    }

    pub fn run(self) -> ! {
        match self {
            Cmd::Help(envp, config_lock_file) => cmd_help(envp, config_lock_file),
            Cmd::ConnatePid(pid) => cmd_connate_pid(pid),
            Cmd::Exec(pid, argv) => cmd_exec(pid, argv),
            Cmd::Status(ipc_client, argv) => cmd_status(ipc_client, argv),
            Cmd::List(ipc_client) => cmd_list(ipc_client),
            Cmd::State(ipc_client, argv) => cmd_state(ipc_client, argv),
            Cmd::Target(ipc_client, argv) => cmd_target(ipc_client, argv),
            Cmd::Pid(ipc_client, argv) => cmd_pid(ipc_client, argv),
            Cmd::Code(ipc_client, argv) => cmd_code(ipc_client, argv),
            Cmd::Attempt(ipc_client, argv) => cmd_attempt(ipc_client, argv),
            Cmd::Time(ipc_client, argv) => cmd_time(ipc_client, argv),
            Cmd::Needs(ipc_client, argv) => cmd_needs(ipc_client, argv),
            Cmd::Wants(ipc_client, argv) => cmd_wants(ipc_client, argv),
            Cmd::Conflicts(ipc_client, argv) => cmd_conflicts(ipc_client, argv),
            Cmd::Groups(ipc_client, argv) => cmd_groups(ipc_client, argv),
            Cmd::Log(ipc_client, argv) => cmd_log(ipc_client, argv),
            Cmd::Up(ipc_client, argv) => cmd_up(ipc_client, argv),
            Cmd::Down(ipc_client, argv) => cmd_down(ipc_client, argv),
            Cmd::Restart(ipc_client, argv) => cmd_restart(ipc_client, argv),
            Cmd::Once(ipc_client, argv) => cmd_once(ipc_client, argv),
            Cmd::SettleUp(ipc_client, argv, pid) => cmd_settle_up(ipc_client, argv, pid),
            Cmd::SettleDown(ipc_client, argv, pid) => cmd_settle_down(ipc_client, argv, pid),
            Cmd::SettleRestart(ipc_client, argv, pid) => cmd_settle_restart(ipc_client, argv, pid),
            Cmd::SettleOnce(ipc_client, argv, pid) => cmd_settle_once(ipc_client, argv, pid),
            Cmd::Ready(ipc_client, pid) => cmd_ready(ipc_client, pid),
        }
    }
}

fn get_pid_from_lock(lock_path: &CStr) -> pid_t {
    Fd::open(lock_path, OpenFlags::O_RDONLY, 0)
        .or_fs_abort("open", lock_path)
        .get_locking_pid()
        .or_fs_abort("get PID locking", lock_path)
        .or_fs_abort("find PID locking", lock_path)
}
