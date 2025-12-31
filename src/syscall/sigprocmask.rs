use crate::err::*;
use crate::types::*;
use syscalls::{Sysno, syscall};

#[allow(non_camel_case_types)]
pub enum SigprocmaskHow {
    SIG_BLOCK = 0,
    SIG_UNBLOCK = 1,
    SIG_SETMASK = 2,
}

// `man 2 sigprocmask`:
//
// SYNOPSIS
//       int sigprocmask(int how, const sigset_t *set, sigset_t *oldset);
//
// RETURN VALUE
//      On success, sigprocmask() returns 0; on error, -1 is returned, and errno is set to indicate
//      the error.
pub unsafe fn sigprocmask(how: SigprocmaskHow, set: &sigset_t) -> Result<(), Errno> {
    // Clippy is confused here; the cast is necessary in the macro.
    #[allow(clippy::unnecessary_cast)]
    syscall!(
        Sysno::rt_sigprocmask,
        how as c_int,
        set as *const sigset_t,
        core::ptr::null_mut() as *mut sigset_t,
        core::mem::size_of::<sigset_t>()
    )
    .map(|_| ())
}
