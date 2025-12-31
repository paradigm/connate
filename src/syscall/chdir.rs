use crate::err::*;
use core::ffi::CStr;
use syscalls::{Sysno, syscall};

// `man 2 chdir`:
//
// SYNOPSIS
//        int chdir(const char *path);
//
// DESCRIPTION
//        chdir() changes the current working directory of the calling process
//        to the directory specified in path.
//
// RETURN VALUE
//        On success, zero is returned.  On error, -1 is returned, and errno is
//        set appropriately.
pub unsafe fn chdir(path: &CStr) -> Result<(), Errno> {
    syscall!(Sysno::chdir, path.as_ptr()).map(|_| ())
}
