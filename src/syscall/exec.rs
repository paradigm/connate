use crate::err::*;
use core::ffi::{CStr, c_char, c_int};
use syscalls::{Sysno, syscall};

// `man 2 execve`:
//
// SYNOPSIS
//       int execve(const char *pathname, char *const _Nullable argv[],
//                  char *const _Nullable envp[]);
//
// RETURN VALUE
//        On success, execve() does not return, on error -1 is returned, and errno is set to indicate the error.
pub unsafe fn execve(
    pathname: &CStr,
    argv: *const *const c_char,
    envp: *const *const c_char,
) -> Result<c_int, Errno> {
    syscall!(
        Sysno::execve,
        pathname.as_ptr() as usize,
        argv as usize,
        envp as usize
    )
    .map(|ret| ret as c_int)
}
