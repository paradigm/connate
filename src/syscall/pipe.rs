use crate::err::*;
use crate::syscall::OpenFlags;
use crate::types::c_int;
use syscalls::{Sysno, syscall};

// `man 2 pipe2`:
//
// SYNOPSIS
//
//        int pipe2(int pipefd[2], int flags);
//
// RETURN VALUE
//        On success, zero is returned.  On error, -1 is returned, errno is set to indicate the
//        error, and pipefd is left unchanged.
pub unsafe fn pipe2(pipefd: &mut [c_int; 2], flags: OpenFlags) -> Result<(), Errno> {
    syscall!(Sysno::pipe2, pipefd.as_mut_ptr(), flags.bits()).map(|_| ())
}
