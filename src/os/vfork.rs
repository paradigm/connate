use crate::err::Errno;
use crate::syscall::ForkResult;

/// Fork the current process with vfork semantics
///
/// Like fork(), but the parent is suspended until the child calls exec or _exit.
/// The child shares memory with the parent until exec or _exit.
///
/// # Safety
///
/// This function is unsafe because even in single-threaded contexts:
/// - The child must not return from the function that called vfork
/// - The child must not modify any data other than a variable of type pid_t used to store the return value
/// - The child must call exec or _exit before doing anything else
///
/// The caller must ensure these constraints are met.
pub unsafe fn vfork() -> Result<ForkResult, Errno> {
    unsafe { crate::syscall::vfork() }
}
