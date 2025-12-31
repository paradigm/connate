use crate::err::*;
use crate::types::mode_t;
use core::ffi::CStr;
use syscalls::{Sysno, syscall};

// `man 2 mkdir`:
//
// SYNOPSIS
//        int mkdir(const char *pathname, mode_t mode);
//
// DESCRIPTION
//        mkdir() attempts to create a directory named pathname.
//
// RETURN VALUE
//        mkdir() and mkdirat() return zero on success, or -1 if an error occurred
//        (in which case, errno is set appropriately).
pub unsafe fn mkdir(pathname: &CStr, mode: mode_t) -> Result<(), Errno> {
    syscall!(Sysno::mkdir, pathname.as_ptr(), mode).map(|_| ())
}
