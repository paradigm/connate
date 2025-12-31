use crate::err::Errno;
use crate::syscall::PrctlOption;

/// Prevent the process and its children from gaining new privileges
///
/// Once set, this bit cannot be unset. All children will inherit this setting.
/// setuid/setgid bits and file capabilities will have no effect.
#[inline]
pub fn set_no_new_privs() -> Result<(), Errno> {
    // SAFETY: prctl with PR_SET_NO_NEW_PRIVS is always safe
    unsafe { crate::syscall::prctl(PrctlOption::PR_SET_NO_NEW_PRIVS, 1) }
}
