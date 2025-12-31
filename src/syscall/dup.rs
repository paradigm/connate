use crate::err::*;
use crate::syscall::OpenFlags;
use crate::types::c_int;
use syscalls::{Sysno, syscall};

// `man 2 dup3`:
//
// SYNOPSIS
//       int dup3(int oldfd, int newfd, int flags);
//
// RETURN VALUE
//        On success, these system calls return the new file descriptor.  On error, -1 is returned,
//        and errno is set to indicate the error.
pub unsafe fn dup3(oldfd: c_int, newfd: c_int, flags: OpenFlags) -> Result<c_int, Errno> {
    syscall!(Sysno::dup3, oldfd, newfd, flags.bits() as usize).map(|fd| fd as c_int)
}
