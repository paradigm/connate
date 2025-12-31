use crate::err::*;
use crate::types::c_int;
use syscalls::{Sysno, syscall};

// `man 2 read`:
//
// SYNOPSIS
//        ssize_t read(int fd, void *buf, size_t count);
//
// RETURN VALUE
//        On success, the number of bytes read is returned.  On error, -1 is returned, and errno is
//        set to indicate the error.
pub unsafe fn read(fd: c_int, buf: &mut [u8]) -> Result<usize, Errno> {
    syscall!(Sysno::read, fd, buf.as_mut_ptr(), buf.len())
}
