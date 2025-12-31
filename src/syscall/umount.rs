use crate::err::*;
use crate::types::*;
use core::ffi::CStr;
use core::ops::BitOr;
use syscalls::{Sysno, syscall};

#[derive(Clone, Copy)]
pub struct UmountFlags(c_int);

impl UmountFlags {
    pub const MNT_FORCE: Self = Self(0o1);
    pub const MNT_DETACH: Self = Self(0o2);
    pub const MNT_EXPIRE: Self = Self(0o4);
    pub const UMOUNT_NOFOLLOW: Self = Self(0o10);

    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn bits(self) -> c_int {
        self.0
    }

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

impl BitOr for UmountFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

// `man 2 umount2`:
//
// SYNOPSIS
//        int umount2(const char *target, int flags);
//
// RETURN VALUE
//        On success, zero is returned. On error, -1 is returned, and errno is set appropriately.
pub unsafe fn umount2(target: &CStr, flags: UmountFlags) -> Result<(), Errno> {
    syscall!(Sysno::umount2, target.as_ptr(), flags.bits()).map(|_| ())
}
