use crate::err::*;
use crate::types::{gid_t, uid_t};
use syscalls::{Sysno, syscall};

/// Set the user ID of the calling process
pub unsafe fn setuid(uid: uid_t) -> Result<(), Errno> {
    syscall!(Sysno::setuid, uid).map(|_| ())
}

/// Set the group ID of the calling process
pub unsafe fn setgid(gid: gid_t) -> Result<(), Errno> {
    syscall!(Sysno::setgid, gid).map(|_| ())
}

/// Set real, effective, and saved user IDs
pub unsafe fn setresuid(ruid: uid_t, euid: uid_t, suid: uid_t) -> Result<(), Errno> {
    syscall!(Sysno::setresuid, ruid, euid, suid).map(|_| ())
}

/// Set real, effective, and saved group IDs
pub unsafe fn setresgid(rgid: gid_t, egid: gid_t, sgid: gid_t) -> Result<(), Errno> {
    syscall!(Sysno::setresgid, rgid, egid, sgid).map(|_| ())
}
