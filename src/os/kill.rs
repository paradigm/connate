use crate::err::Errno;
use crate::types::{Signal, pid_t};

#[inline]
pub fn kill(pid: pid_t, sig: Signal) -> Result<(), Errno> {
    unsafe { crate::syscall::kill(pid, sig as i32) }
}
