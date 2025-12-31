use crate::err::*;
use crate::types::c_int;
use syscalls::{Sysno, syscall};

// `man 2 getdents64`:
//
// SYNOPSIS
//        long getdents64(unsigned int fd, struct linux_dirent64 *dirp, unsigned int count);
//
// RETURN VALUE
//        On success, the number of bytes read is returned. On end of directory, 0 is returned.
//        On error, -1 is returned, and errno is set to indicate the error.
pub unsafe fn getdents64(fd: c_int, buf: &mut [u8]) -> Result<usize, Errno> {
    syscall!(Sysno::getdents64, fd, buf.as_mut_ptr(), buf.len())
}
