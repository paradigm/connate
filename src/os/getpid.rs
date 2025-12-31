use crate::types::pid_t;

#[inline]
pub fn getpid() -> pid_t {
    unsafe { crate::syscall::getpid() }
}
