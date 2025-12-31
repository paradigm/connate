use crate::err::*;
use crate::types::*;

/// Returns the errno value on failure (e.g., EEXIST if directory already exists)
#[inline]
pub fn mkdir(pathname: &CStr, mode: mode_t) -> Result<(), Errno> {
    unsafe { crate::syscall::mkdir(pathname, mode) }
}
