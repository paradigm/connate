use crate::constants::FD_SIGNAL;
use crate::err::*;
use crate::os::{Fd, OpenFlags};
pub use crate::syscall::SigInfo;
use crate::syscall::{SignalFdFlags, read, signalfd};
use crate::types::*;

pub struct SignalFd(c_int);

impl SignalFd {
    pub fn new() -> Result<Self, Errno> {
        let mut signals = sigset_t::new_empty_set();
        signals |= Signal::SIGHUP;
        signals |= Signal::SIGINT;
        signals |= Signal::SIGTERM;
        signals |= Signal::SIGCHLD;

        // We do not SFD_CLOEXEC here to ensure the signalfd survives a re-exec.
        let flags = SignalFdFlags::empty();
        let fd = unsafe { signalfd(-1, &signals, flags)? };
        Ok(Self(fd))
    }

    pub fn read_siginfo(&mut self) -> Result<SigInfo, Errno> {
        let mut siginfo = SigInfo::new();
        let raw_buf = unsafe {
            core::slice::from_raw_parts_mut(
                &mut siginfo as *mut SigInfo as *mut u8,
                core::mem::size_of::<SigInfo>(),
            )
        };
        let bytes = unsafe { read(self.as_raw(), raw_buf)? };

        if bytes == core::mem::size_of::<SigInfo>() {
            Ok(siginfo)
        } else {
            Err(Errno::EINVAL)
        }
    }

    pub fn read_signal(&mut self) -> Result<Signal, Errno> {
        self.read_siginfo().map(|info| info.signal())
    }

    pub fn as_raw(&self) -> c_int {
        self.0
    }

    pub fn from_raw(fd: c_int) -> Self {
        Self(fd)
    }

    pub fn move_to(self, new_fd: c_int) -> Result<Self, Errno> {
        let old_fd = Fd::from_raw(self.0);
        let new_fd = old_fd.dup(new_fd, OpenFlags::empty())?;
        old_fd.close()?;
        Ok(Self(new_fd.into_raw()))
    }

    pub fn try_resume() -> Option<Self> {
        let fd = Fd::from_raw(FD_SIGNAL);
        if fd.is_valid() {
            Some(Self(FD_SIGNAL))
        } else {
            None
        }
    }
}
