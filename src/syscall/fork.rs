use crate::err::*;
use crate::types::pid_t;
use syscalls::{Sysno, syscall};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForkResult {
    /// In parent process with child PID
    Parent(pid_t),
    /// In child process
    Child,
}

/// Fork the current process
///
/// Returns ForkResult::Parent(pid) in the parent process, ForkResult::Child in the child
///
/// # Safety
///
/// This function can be called safely in single-threaded contexts.
pub unsafe fn fork() -> Result<ForkResult, Errno> {
    let pid = syscall!(Sysno::fork)? as pid_t;
    if pid == 0 {
        Ok(ForkResult::Child)
    } else {
        Ok(ForkResult::Parent(pid))
    }
}
