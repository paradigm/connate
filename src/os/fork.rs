use crate::err::Errno;

pub type ForkResult = crate::syscall::ForkResult;

/// Fork the current process
///
/// # Safety
///
/// fork() is unsafe in:
/// - Multithreaded environments
/// - Async environments
/// - non-signalfd Signal handling
///
/// Connate eschews all of these, and thus reasonable usage of fork() is safe.
pub fn fork() -> Result<ForkResult, Errno> {
    // SAFETY: connate is single-threaded, so fork is safe
    unsafe { crate::syscall::fork() }
}
