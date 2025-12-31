use crate::err::*;
use core::ffi::CStr;

/// Read the target of a symbolic link into `buf`, returning the number of bytes written.
#[inline]
pub fn readlink(pathname: &CStr, buf: &mut [u8]) -> Result<usize, Errno> {
    // SAFETY: Fully type checked, this is syscall is always safe
    unsafe { crate::syscall::readlink(pathname, buf) }
}
