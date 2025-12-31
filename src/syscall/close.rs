use crate::err::*;
use crate::types::c_int;
use syscalls::{Sysno, syscall};

// `man 2 close`:
//
// SYNOPSIS
//        int close(int fd);
//
// RETURN VALUE
//        Returns zero on success.  On error, -1 is returned, and errno is set to indicate the
//        error.
pub unsafe fn close(fd: c_int) -> Result<(), Errno> {
    syscall!(Sysno::close, fd).map(|_| ())
}
