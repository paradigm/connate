use crate::err::Errno;
use crate::syscall::PrctlOption;

/// Enable this process to become a subreaper for its descendants
///
/// When a process dies, its children are reparented. Normally they're reparented to init (PID 1).
/// A subreaper process will instead become the parent of any descendant processes whose
/// immediate parent dies. This allows connate to reap service processes even if they
/// fork children that outlive the service.
#[inline]
pub fn set_child_subreaper() -> Result<(), Errno> {
    // SAFETY: prctl with PR_SET_CHILD_SUBREAPER is always safe
    unsafe { crate::syscall::prctl(PrctlOption::PR_SET_CHILD_SUBREAPER, 1) }
}
