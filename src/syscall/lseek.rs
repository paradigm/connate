use crate::err::*;
use crate::types::{c_int, off_t};
use syscalls::{Sysno, syscall};

/// Whence values for lseek
#[allow(non_camel_case_types)]
#[repr(C)]
pub enum SeekWhence {
    SEEK_SET = 0,
    SEEK_CUR = 1,
    SEEK_END = 2,
}

// `man 2 lseek`:
//
// SYNOPSIS
//        off_t lseek(int fd, off_t offset, int whence);
//
// DESCRIPTION
//        lseek() repositions the file offset of the open file description associated with the file
//        descriptor fd to the argument offset according to the directive whence.
//
//        whence can be:
//        SEEK_SET - The file offset is set to offset bytes.
//        SEEK_CUR - The file offset is set to its current location plus offset bytes.
//        SEEK_END - The file offset is set to the size of the file plus offset bytes.
//
// RETURN VALUE
//        Upon successful completion, lseek() returns the resulting offset location as measured in
//        bytes from the beginning of the file. On error, the value (off_t) -1 is returned and
//        errno is set to indicate the error.
pub unsafe fn lseek(fd: c_int, offset: off_t, whence: SeekWhence) -> Result<off_t, Errno> {
    syscall!(Sysno::lseek, fd, offset, whence as c_int).map(|pos| pos as off_t)
}
