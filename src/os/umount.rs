use crate::err::*;
use crate::types::*;

pub use crate::syscall::UmountFlags;

/// Unmount a filesystem
///
/// # Arguments
/// * `target` - The mount point directory to unmount
/// * `flags` - Unmount flags (UmountFlags::empty() for normal unmount, MNT_FORCE, MNT_DETACH, etc.)
///
/// # Errors
/// Returns the errno value on failure
#[inline]
pub fn umount(target: &CStr, flags: UmountFlags) -> Result<(), Errno> {
    unsafe { crate::syscall::umount2(target, flags) }
}
