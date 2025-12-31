use crate::err::*;
use crate::types::*;
use syscalls::{Sysno, syscall};

// `man 2 nanosleep`:
//
// SYNOPSIS
//        int nanosleep(const struct timespec *req, struct timespec *rem);
//
// RETURN VALUE
//        On successfully sleeping for the requested interval, nanosleep() returns 0.  If the call
//        is interrupted by a signal handler, nanosleep() returns -1, sets errno to EINTR, and loads
//        the remaining time into `rem`.
pub unsafe fn nanosleep(request: &timespec, remain: Option<&mut timespec>) -> Result<(), Errno> {
    let remain_ptr = remain
        .map(|remain| remain as *mut timespec)
        .unwrap_or(core::ptr::null_mut());

    syscall!(Sysno::nanosleep, request as *const timespec, remain_ptr).map(|_| ())
}
