use crate::types::c_int;
use syscalls::{Sysno, syscall};

// `man 2 exit`:
//
// SYNOPSIS
//       void _exit(int status);
//
// RETURN VALUE
//      These functions do not return.
pub unsafe fn exit(status: c_int) -> ! {
    let _ = syscall!(Sysno::exit, status);
    // Inform the compiler that this function does not return
    unsafe { core::hint::unreachable_unchecked() };
}
