use core::ffi::CStr;

use crate::err::Errno;
use crate::syscall::PrctlOption;

/// Set the process name visible in /proc/[pid]/comm
///
/// The name is truncated to 15 characters (16 bytes including null terminator).
/// This name appears in tools like htop, ps, and pstree.
#[inline]
pub fn set_process_name(name: &CStr) -> Result<(), Errno> {
    // SAFETY: prctl with PR_SET_NAME and a valid CStr pointer is always safe
    unsafe { crate::syscall::prctl(PrctlOption::PR_SET_NAME, name.as_ptr() as usize) }
}
