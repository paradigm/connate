use crate::err::*;
use crate::types::{c_int, pid_t};
use syscalls::{Sysno, syscall};

// `man 2 kill`:
//
// SYNOPSIS
//        int kill(pid_t pid, int sig);
//
// RETURN VALUE
//        On success (at least one signal was sent), zero is returned.  On error, -1 is returned,
//        and errno is set to indicate the error.
pub unsafe fn kill(pid: pid_t, sig: c_int) -> Result<(), Errno> {
    syscall!(Sysno::kill, pid, sig).map(|_| ())
}
