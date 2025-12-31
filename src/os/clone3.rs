use crate::err::Errno;

pub use crate::syscall::{Clone3Args, CloneResult};

pub fn clone3(args: &Clone3Args) -> Result<CloneResult, Errno> {
    // SAFETY: connate is single-threaded, so clone3 is safe
    unsafe { crate::syscall::clone3(args) }
}
