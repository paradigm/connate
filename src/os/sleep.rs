use crate::err::*;
use crate::syscall::nanosleep;
use crate::types::*;

/// Sleep for the provided number of seconds.
pub fn sleep(seconds: i64) -> Result<(), Errno> {
    if seconds < 0 {
        return Err(Errno::EINVAL);
    }

    let request = timespec {
        tv_sec: seconds,
        tv_nsec: 0,
    };

    let mut remain = timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };

    unsafe { nanosleep(&request, Some(&mut remain)) }
}
