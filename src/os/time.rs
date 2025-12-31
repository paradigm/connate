use crate::err::*;
use crate::syscall::{ClockId, clock_gettime};
use crate::types::timespec;

/// Get monotonically increasing time
///
/// Uses CLOCK_MONOTONIC_COARSE.  Given the second-level accuracy, overhead for non-coarse isn't
/// beneficial.
///
/// Returns a timespec with tv_sec (seconds) and tv_nsec (nanoseconds).
#[inline]
pub fn get_time_monotonic() -> Result<timespec, Errno> {
    let mut tp = timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // Safety: Only concern is that `tp` is a valid pointer, which we've just created.
    unsafe { clock_gettime(ClockId::CLOCK_MONOTONIC_COARSE, &mut tp) }?;
    Ok(tp)
}
