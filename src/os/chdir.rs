use crate::err::*;
use crate::types::*;

#[inline]
pub fn chdir(path: &CStr) -> Result<(), Errno> {
    unsafe { crate::syscall::chdir(path) }
}
