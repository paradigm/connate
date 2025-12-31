use crate::err::*;
use crate::types::{CStr, c_int};
use core::ops::BitOr;
use syscalls::{Sysno, syscall};

/// memfd_create flags
#[derive(Clone, Copy)]
pub struct MemfdFlags(c_int);

impl MemfdFlags {
    pub const MFD_CLOEXEC: Self = Self(0x0001);
    pub const MFD_ALLOW_SEALING: Self = Self(0x0002);

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

impl BitOr for MemfdFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

// `man 2 memfd_create`:
//
// SYNOPSIS
//        int memfd_create(const char *name, unsigned int flags);
//
// DESCRIPTION
//        memfd_create() creates an anonymous file and returns a file descriptor that refers to it.
//        The file behaves like a regular file, and so can be modified, truncated, memory-mapped, and
//        so on. However, unlike a regular file, it lives in RAM and has a volatile backing storage.
//
// RETURN VALUE
//        On success, memfd_create() returns a new file descriptor. On error, -1 is returned and
//        errno is set to indicate the error.
pub unsafe fn memfd_create(name: &CStr, flags: MemfdFlags) -> Result<c_int, Errno> {
    syscall!(Sysno::memfd_create, name.as_ptr(), flags.bits()).map(|fd| fd as c_int)
}
