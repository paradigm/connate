use crate::err::Errno;
use crate::types::c_int;

pub use crate::syscall::{CloneFlags, CloneResult};

/// # Safety
///
/// The caller must ensure:
/// - `stack` points to a valid stack region (or is null for default behavior)
/// - `parent_tid` and `child_tid` point to valid, aligned memory if the corresponding flags are set
/// - The memory pointed to by these pointers is not accessed from other threads during the call
pub unsafe fn clone(
    flags: CloneFlags,
    stack: *mut u8,
    parent_tid: *mut c_int,
    child_tid: *mut c_int,
    tls: usize,
) -> Result<CloneResult, Errno> {
    // SAFETY: The caller has guaranteed the safety preconditions
    unsafe { crate::syscall::clone(flags, stack, parent_tid, child_tid, tls) }
}
