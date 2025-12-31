use crate::err::*;
use crate::types::{c_int, off_t};
use syscalls::{Sysno, syscall};

// `man 2 ftruncate`:
//
// SYNOPSIS
//        int ftruncate(int fd, off_t length);
//
// DESCRIPTION
//        The ftruncate() function causes the regular file referenced by fd to be truncated to a
//        size of precisely length bytes.
//
//        If the file previously was larger than this size, the extra data is lost. If the file
//        previously was shorter, it is extended, and the extended part reads as null bytes ('\0').
//
// RETURN VALUE
//        On success, zero is returned. On error, -1 is returned, and errno is set to indicate
//        the error.
pub unsafe fn ftruncate(fd: c_int, length: off_t) -> Result<(), Errno> {
    syscall!(Sysno::ftruncate, fd, length).map(|_| ())
}
