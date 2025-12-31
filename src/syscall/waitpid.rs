use crate::err::*;
use crate::types::*;
use core::ffi::c_int;
use core::ops::BitOr;
use syscalls::{Sysno, syscall};

/// Options for waitpid
#[derive(Clone, Copy)]
pub struct WaitPidOptions(c_int);

impl WaitPidOptions {
    pub const WNOHANG: Self = Self(1);
    pub const WUNTRACED: Self = Self(2);
    pub const WCONTINUED: Self = Self(8);

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

impl BitOr for WaitPidOptions {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

/// Wait for a child process to change state
///
/// If pid == -1, wait for any child process
/// If pid > 0, wait for the child whose process ID equals pid
///
/// Returns (pid, status) of the child that changed state
pub unsafe fn waitpid(pid: pid_t, options: WaitPidOptions) -> Result<(pid_t, c_int), Errno> {
    let mut status: c_int = 0;
    let status_ptr = &mut status as *mut c_int;

    syscall!(
        Sysno::wait4,
        pid,
        status_ptr,
        options.bits(),
        0 // rusage pointer (NULL)
    )
    .map(|ret| (ret as pid_t, status))
}

/// Extract exit status from wait status
pub const fn wexitstatus(status: c_int) -> c_int {
    (status >> 8) & 0xff
}

/// Check if process exited normally
pub const fn wifexited(status: c_int) -> bool {
    (status & 0x7f) == 0
}

/// Check if process was terminated by signal
pub const fn wifsignaled(status: c_int) -> bool {
    ((status & 0x7f) + 1) as i8 >= 2
}

/// Extract termination signal from wait status
pub const fn wtermsig(status: c_int) -> c_int {
    status & 0x7f
}

/// Check if process was stopped
pub const fn wifstopped(status: c_int) -> bool {
    (status & 0xff) == 0x7f
}

/// Extract stop signal from wait status
pub const fn wstopsig(status: c_int) -> c_int {
    wexitstatus(status)
}
