use crate::types::pid_t;

#[inline]
pub fn getppid() -> pid_t {
    unsafe { crate::syscall::getppid() }
}
