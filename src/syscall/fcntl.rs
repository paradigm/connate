use crate::err::*;
use crate::types::{c_int, c_short, off_t, pid_t};
use syscalls::{Sysno, syscall};

// FlockType and FlockWhence are defined in C documentation as `short`.
//
// Rust doesn't let us directly use #[repr(c_short)].
// Assert the type we are using is the same size as c_short.
const _: () = assert!(core::mem::size_of::<c_short>() == core::mem::size_of::<u16>());

#[allow(non_camel_case_types)]
#[repr(C)]
pub enum FcntlCmd {
    F_GETFL = 3,
    F_SETFL = 4,
    F_GETLK = 5,
    F_SETLK = 6,
    F_SETLKW = 7,
}

#[allow(non_camel_case_types)]
#[repr(u16)]
pub enum FlockType {
    F_RDLCK = 0,
    F_WRLCK = 1,
    F_UNLCK = 2,
}

#[allow(non_camel_case_types)]
#[repr(u16)]
pub enum FlockWhence {
    SEEK_SET = 0,
    SEEK_CUR = 1,
    SEEK_END = 2,
}

#[repr(C)]
pub struct Flock {
    pub l_type: FlockType,
    pub l_whence: FlockWhence,
    pub l_start: off_t,
    pub l_len: off_t,
    pub l_pid: pid_t,
}
const _: () = assert!(core::mem::size_of::<Flock>() == 32);

// `man 2 fcntl`:
//
// SYNOPSIS
//        int fcntl(int fd, int cmd, ... /* arg */ );
//
// RETURN VALUE
//        For a successful call, the return value depends on the operation:
//
//        [...non-lock operations...]
//
//        All other commands
//               Zero.
//
//        On error, -1 is returned, and errno is set to indicate the error.
pub unsafe fn fcntl(fd: c_int, cmd: FcntlCmd, flock: &mut Flock) -> Result<c_int, Errno> {
    syscall!(Sysno::fcntl, fd, cmd, flock as *mut Flock).map(|ret| ret as c_int)
}

// fcntl for F_GETFL/F_SETFL which take/return integer flags
pub unsafe fn fcntl_flags(fd: c_int, cmd: FcntlCmd, flags: c_int) -> Result<c_int, Errno> {
    syscall!(Sysno::fcntl, fd, cmd, flags).map(|ret| ret as c_int)
}
