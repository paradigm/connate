use crate::err::*;

#[inline]
pub fn sync() -> Result<(), Errno> {
    unsafe { crate::syscall::sync() }
}
