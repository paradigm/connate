use crate::err::*;
use crate::types::{gid_t, uid_t};

/// Set the user ID of the calling process
///
/// # Safety
///
/// Requires appropriate privileges (typically root)
#[inline]
pub fn setuid(uid: uid_t) -> Result<(), Errno> {
    // SAFETY: Caller must have appropriate privileges
    unsafe { crate::syscall::setuid(uid) }
}

/// Set the group ID of the calling process
///
/// # Safety
///
/// Requires appropriate privileges (typically root)
#[inline]
pub fn setgid(gid: gid_t) -> Result<(), Errno> {
    // SAFETY: Caller must have appropriate privileges
    unsafe { crate::syscall::setgid(gid) }
}

/// Set real, effective, and saved user IDs
///
/// # Safety
///
/// Requires appropriate privileges (typically root)
#[inline]
pub fn setresuid(ruid: uid_t, euid: uid_t, suid: uid_t) -> Result<(), Errno> {
    // SAFETY: Caller must have appropriate privileges
    unsafe { crate::syscall::setresuid(ruid, euid, suid) }
}

/// Set real, effective, and saved group IDs
///
/// # Safety
///
/// Requires appropriate privileges (typically root)
#[inline]
pub fn setresgid(rgid: gid_t, egid: gid_t, sgid: gid_t) -> Result<(), Errno> {
    // SAFETY: Caller must have appropriate privileges
    unsafe { crate::syscall::setresgid(rgid, egid, sgid) }
}
