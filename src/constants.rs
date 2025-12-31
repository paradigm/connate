use crate::ipc::RequestHeader;
use crate::types::{StrLen, c_int, pid_t};

// Linux standard pipe size
// Writes <= to this are guaranteed to be atomic
pub const PIPE_BUF: usize = 4096;

// Fixed FD numbers
// - Allows IPC via /proc/<connate-pid>/fd/<fd>
// - Allows accessing FDs after exec() such as memfd to load session
pub const FD_SESSION_STATE: i32 = 100;
pub const FD_SESSION_STATE_STR: &[u8] = b"100";
pub const FD_SIGNAL: i32 = 101;
pub const FD_SIGNAL_STR: &[u8] = b"101";
pub const FD_LOCK_FILE: i32 = 102;
pub const FD_REQ_READ: i32 = 110;
pub const FD_REQ_READ_STR: &[u8] = b"110";
pub const FD_REQ_WRITE: i32 = 111;
pub const FD_REQ_WRITE_STR: &[u8] = b"111";
pub const FD_RESP_READ: i32 = 112;
pub const FD_RESP_READ_STR: &[u8] = b"112";
pub const FD_RESP_WRITE: i32 = 113;
pub const FD_RESP_WRITE_STR: &[u8] = b"113";

/// IPC messages are no more than PIPE_BUF size to ensure they're atomic which allows us to
/// simplify IPC logic.
pub const MSG_SIZE: usize = PIPE_BUF;

/// Service name size constraint is determined by worst-case IPC request: dependency queries which
/// both specify the service name and the usize index of the dependency
///
/// header(1) + usize(8) + str_length(2) + name(?) <= PIPE_BUF(4096)
pub const MSG_SVC_NAME_SIZE: usize = PIPE_BUF // Message size limit
    - size_of::<RequestHeader>() // Request header byte
    - size_of::<usize>() // Dependency index
    - size_of::<StrLen>(); // String length prefix

/// Path size constraint is determined by worst-case IPC request: that which requires a null
/// termination so that it can be trivially constructed into a CStr.
///
/// While not all instances of paths require a null, applying this universally minimizes chance we
/// miss a scenario and introduce a possible runtime bug.
///
/// header(1) + str_length(2) + path(?) + null <= PIPE_BUF(4096)
pub const MSG_PATH_SIZE: usize = PIPE_BUF // Message size limit
    - size_of::<RequestHeader>() // Request header byte
    - size_of::<StrLen>() // String length prefix
    - size_of::<u8>(); // Trailing null

// IPC messages for Option values can use `-1` as a sentinel value for None if it isn't a valid
// Some() value.

/// IPC message sentinel value for a lack of a PID
///
/// - On Linux, valid PIDs are always positive.
/// - `-1` is used by various PID-returning system calls to indicate an error occurred and cannot
///   be a valid PID value.
pub const MSG_PID_NONE_SENTINEL: pid_t = -1;

/// IPC message sentinel value for a lack of an exit code
///
/// - On Linux,n exit codes are unsigned 8-bit values (0-255). Negative values are impossible.
/// - If a process calls `exit(-1)` then exit code becomes 255.
pub const MSG_EXIT_CODE_NONE_SENTINEL: c_int = -1;

// Hard-coded timeouts
pub const UP_TIME_MILLIS: i64 = 1_000;
pub const FORCED_DOWN_TIME_MILLIS: i64 = 1_000;

// Environment variables
pub const LOCK_FILE_ENVVAR: &[u8] = b"CONNATE_LOCK_FILE";
pub const PID_ENVVAR: &[u8] = b"CONNATE_PID";
