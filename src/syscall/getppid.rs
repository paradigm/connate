use crate::types::pid_t;
use syscalls::{Sysno, syscall};

// `man 2 getppid`:
//
// SYNOPSIS
//        pid_t getppid(void);
//
// ERRORS
//        These functions are always successful.
//
pub unsafe fn getppid() -> pid_t {
    unsafe { syscall!(Sysno::getppid).unwrap_unchecked() as pid_t }
}
