use crate::err::*;
use crate::types::*;
use core::ops::BitOr;
use syscalls::{Sysno, syscall};

#[repr(C)]
#[derive(Default)]
pub struct SigInfo {
    si_signo: u32,
    si_errno: i32,
    si_code: i32,
    si_pid: u32,
    si_uid: u32,
    si_fd: i32,
    si_tid: u32,
    si_band: u32,
    si_overrun: u32,
    si_trapno: u32,
    si_status: i32,
    si_int: i32,
    si_ptr: u64,
    si_utime: u64,
    si_stime: u64,
    si_addr: u64,
    si_addr_lsb: u16,
    _pad: u16,
    si_syscall: i32,
    si_call_addr: u64,
    si_arch: u32,
    _pad2: [u8; 28], // pad to 128 bytes
}
const _: () = assert!(core::mem::size_of::<SigInfo>() == 128);

impl SigInfo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn signal(&self) -> Signal {
        // TODO: once inline const patterns stabilize, use them here
        // https://github.com/rust-lang/rust/issues/76001
        const SIGHUP: u32 = Signal::SIGHUP as u32;
        const SIGINT: u32 = Signal::SIGINT as u32;
        const SIGTERM: u32 = Signal::SIGTERM as u32;
        const SIGCHLD: u32 = Signal::SIGCHLD as u32;
        match self.si_signo {
            SIGHUP => Signal::SIGHUP,
            SIGINT => Signal::SIGINT,
            SIGTERM => Signal::SIGTERM,
            SIGCHLD => Signal::SIGCHLD,
            _ => Signal::UNRECOGNIZED,
        }
    }

    pub fn pid(&self) -> pid_t {
        self.si_pid as pid_t
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy)]
pub struct SignalFdFlags(c_int);

impl SignalFdFlags {
    pub const SFD_CLOEXEC: Self = Self(0x0008_0000);
    pub const SFD_NONBLOCK: Self = Self(0x0000_0800);

    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn bits(self) -> c_int {
        self.0
    }

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

impl BitOr for SignalFdFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

// `man 2 signalfd`:
//
// SYNOPSIS
//       int signalfd(int fd, const sigset_t *mask, int flags);
//
// RETURN VALUE
//       On success, signalfd() returns a signalfd file descriptor; this is either a new file
//       descriptor (if fd was -1), or fd if fd was a valid signalfd file descriptor.  On error,
//       -1 is returned and errno is set to indicate the error.
pub unsafe fn signalfd(fd: c_int, mask: &sigset_t, flags: SignalFdFlags) -> Result<c_int, Errno> {
    syscall!(
        Sysno::signalfd4,
        fd,
        mask as *const sigset_t,
        core::mem::size_of::<sigset_t>(),
        flags.bits()
    )
    .map(|raw_fd| raw_fd as c_int)
}
