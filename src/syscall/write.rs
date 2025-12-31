use crate::err::*;
use crate::types::c_int;
use syscalls::{Sysno, syscall};

// `man 2 write`:
//
// SYNOPSIS
//        ssize_t write(int fd, const void buf[.count], size_t count);
//
// RETURN VALUE
//        On success, the number of bytes written is returned.  On error, -1 is returned, and errno
//        is set to indicate the error.
pub unsafe fn write(fd: c_int, buf: &[u8]) -> Result<usize, Errno> {
    syscall!(Sysno::write, fd, buf.as_ptr(), buf.len())
}
