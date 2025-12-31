use crate::err::Errno;
use crate::types::*;

pub use crate::syscall::{IdType, WaitIdInfo, WaitIdOptions};

/// Wait for a child process to change state
///
/// idtype specifies which children to wait for (P_ALL, P_PID, P_PGID)
/// id specifies the specific pid/pgid if idtype is P_PID or P_PGID (ignored for P_ALL)
/// infop is filled with information about the child
/// options specifies wait options (WEXITED, WSTOPPED, WCONTINUED, WNOHANG, WNOWAIT)
///
/// Returns Ok(()) on success with infop filled, or Err(errno) on failure
#[inline]
pub fn waitid(
    idtype: IdType,
    id: pid_t,
    infop: &mut WaitIdInfo,
    options: WaitIdOptions,
) -> Result<(), Errno> {
    // SAFETY: waitid is safe to call with valid pointers and we're passing a valid mutable reference
    unsafe { crate::syscall::waitid(idtype, id, infop, options) }
}
