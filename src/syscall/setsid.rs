use crate::err::*;
use crate::types::pid_t;
use syscalls::{Sysno, syscall};

/// Create a new session and set the process group ID
///
/// Returns the new session ID (which equals the calling process's PID)
pub unsafe fn setsid() -> Result<pid_t, Errno> {
    syscall!(Sysno::setsid).map(|ret| ret as pid_t)
}
