use crate::err::*;
use crate::types::c_int;
use syscalls::{Sysno, syscall};

#[cfg(not(any(
    all(target_os = "linux", target_arch = "aarch64"),
    all(target_os = "linux", target_arch = "x86_64"),
)))]

compile_error!("@src/syscall/ioctl.rs only supports Linux x86_64 and Linux AArch64.");
/// ioctl request codes
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum IoctlRequest {
    // While TCGETS is the same on x86_64 and aarch64, it isn't guaranteed to be everywhere.
    // We're configuring per-ISA to make sure that, should we ever refactor this, we don't forget
    // to change for other ISAs.
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    TCGETS = 0x5401,
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    TCGETS = 0x5401,
}

// NAME
//       ioctl - control device
// RETURN VALUE
//        Usually, on success zero is returned.  A few ioctl() operations use the return value as
//        an output parameter and return a nonnegative value on success.  On error, -1 is returned,
//        and errno is set to indicate the error.
pub unsafe fn ioctl(fd: c_int, request: IoctlRequest, arg: usize) -> Result<c_int, Errno> {
    syscall!(Sysno::ioctl, fd, request as c_int, arg).map(|ret| ret as c_int)
}
