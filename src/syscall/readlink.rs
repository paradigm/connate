use crate::err::*;
use core::ffi::CStr;
use syscalls::{Sysno, syscall};

// `man 2 readlink`:
//
// SYNOPSIS
//        ssize_t readlink(const char *restrict pathname, char *restrict buf,
//                        size_t bufsiz);
//
// RETURN VALUE
//        On success, these calls return the number of bytes placed in buf.  On error, -1 is
//        returned, and errno is set to indicate the error.
pub unsafe fn readlink(pathname: &CStr, buf: &mut [u8]) -> Result<usize, Errno> {
    syscall!(
        Sysno::readlink,
        pathname.as_ptr() as usize,
        buf.as_mut_ptr() as usize,
        buf.len()
    )
}
