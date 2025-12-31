use crate::types::pid_t;
use syscalls::{Sysno, syscall};

// `man 2 getpid`:
//
// SYNOPSIS
//        pid_t getpid(void);
//
// ERRORS
//        These functions are always successful.
//
pub unsafe fn getpid() -> pid_t {
    unsafe { syscall!(Sysno::getpid).unwrap_unchecked() as pid_t }
}
