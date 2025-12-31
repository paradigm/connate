use crate::err::*;
use crate::types::timespec;
use syscalls::{Sysno, syscall};

#[allow(non_camel_case_types)]
#[repr(C)]
pub enum ClockId {
    CLOCK_REALTIME = 0,
    CLOCK_MONOTONIC = 1,
    CLOCK_MONOTONIC_COARSE = 6,
}

/// # Safety
///
/// The caller must ensure `tp` is a valid pointer.
pub unsafe fn clock_gettime(clock_id: ClockId, tp: *mut timespec) -> Result<(), Errno> {
    syscall!(Sysno::clock_gettime, clock_id as usize, tp as usize).map(|_| ())
}
