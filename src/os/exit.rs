use crate::types::c_int;

#[inline]
pub fn exit(status: c_int) -> ! {
    unsafe { crate::syscall::exit(status) }
}
