use crate::err::*;
use crate::types::pid_t;
use syscalls::{Sysno, syscall};

use crate::syscall::ForkResult;

/// Fork the current process with vfork semantics
///
/// Like fork(), but the parent is suspended until the child calls exec or _exit.
/// The child shares memory with the parent until exec or _exit.
///
/// Returns ForkResult::Parent(pid) in the parent process, ForkResult::Child in the child
///
/// # Safety
///
/// This function is unsafe because:
/// - The child must not return from the function that called vfork
/// - The child must not modify any data other than a variable of type pid_t used to store the return value
/// - The child must call exec or _exit before doing anything else
pub unsafe fn vfork() -> Result<ForkResult, Errno> {
    let pid = syscall!(Sysno::vfork)? as pid_t;
    if pid == 0 {
        Ok(ForkResult::Child)
    } else {
        Ok(ForkResult::Parent(pid))
    }
}
