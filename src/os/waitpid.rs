use crate::err::*;
use crate::types::*;

pub use crate::syscall::WaitPidOptions;

/// Wait for a child process to change state
///
/// Returns (pid, status) of the child that changed state
#[inline]
pub fn waitpid(pid: pid_t, options: WaitPidOptions) -> Result<(pid_t, c_int), Errno> {
    // SAFETY: waitpid is always safe to call
    unsafe { crate::syscall::waitpid(pid, options) }
}

/// Extract exit status from wait status
pub const fn wexitstatus(status: c_int) -> c_int {
    crate::syscall::wexitstatus(status)
}

/// Check if process exited normally
pub const fn wifexited(status: c_int) -> bool {
    crate::syscall::wifexited(status)
}

/// Check if process was terminated by signal
pub const fn wifsignaled(status: c_int) -> bool {
    crate::syscall::wifsignaled(status)
}

/// Extract termination signal from wait status
pub const fn wtermsig(status: c_int) -> c_int {
    crate::syscall::wtermsig(status)
}

/// Check if process was stopped
pub const fn wifstopped(status: c_int) -> bool {
    crate::syscall::wifstopped(status)
}

/// Extract stop signal from wait status
pub const fn wstopsig(status: c_int) -> c_int {
    crate::syscall::wstopsig(status)
}
