use crate::err::*;
use crate::types::*;
use syscalls::{Sysno, syscall};

/// prctl operations
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum PrctlOption {
    /// Set the process name (visible in /proc/[pid]/comm)
    PR_SET_NAME = 15,
    /// Set the "child subreaper" attribute of the calling process
    PR_SET_CHILD_SUBREAPER = 36,
    /// Prevent new privileges from being granted
    PR_SET_NO_NEW_PRIVS = 38,
}

impl PrctlOption {
    pub fn bits(self) -> c_int {
        self as c_int
    }
}

/// Perform operations on a process or thread
pub unsafe fn prctl(option: PrctlOption, arg2: usize) -> Result<(), Errno> {
    syscall!(Sysno::prctl, option.bits(), arg2, 0, 0, 0).map(|_| ())
}
