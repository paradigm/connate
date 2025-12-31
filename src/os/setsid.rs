use crate::err::*;
use crate::types::pid_t;

/// Create a new session for the calling process
///
/// This makes the process a session leader and process group leader.
/// The process will have no controlling terminal.
#[inline]
pub fn setsid() -> Result<pid_t, Errno> {
    // SAFETY: setsid is always safe to call
    unsafe { crate::syscall::setsid() }
}
