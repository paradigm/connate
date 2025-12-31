use crate::err::*;
use crate::types::c_int;
use core::ops::BitOr;
use syscalls::{Sysno, syscall};

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct PollEvents(u16);

impl PollEvents {
    pub const POLLIN: Self = Self(0x0001);

    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn bits(self) -> u16 {
        self.0
    }

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

impl BitOr for PollEvents {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PollFd {
    pub fd: c_int,
    pub events: PollEvents,
    pub revents: PollEvents,
}
const _: () = assert!(core::mem::size_of::<PollFd>() == 8);

// `man 2 poll`:
//
// SYNOPSIS
//        int poll(struct pollfd *fds, nfds_t nfds, int timeout);
//
// RETURN VALUE
//        On success, poll() returns a nonnegative value which is the number of elements in the
//        pollfds whose revents fields have been set to a nonzero value (indicating an event or an
//        error).  A return value of zero indicates that the system call timed out before any file
//        descriptors became ready.
//
//        On error, -1 is returned, and errno is set to indicate the error.
pub unsafe fn poll(fds: &mut [PollFd], timeout: i32) -> Result<i32, Errno> {
    syscall!(Sysno::poll, fds.as_mut_ptr(), fds.len() as u64, timeout).map(|n| n as i32)
}
