use crate::err::*;
use crate::types::{CStr, c_int};
use core::ops::BitOr;
use syscalls::{Sysno, syscall};

#[derive(Clone, Copy)]
pub struct OpenFlags(c_int);

impl OpenFlags {
    pub const O_CLOEXEC: Self = Self(0o2000000);
    pub const O_RDONLY: Self = Self(0o0000000);
    pub const O_WRONLY: Self = Self(0o0000001);
    pub const O_RDWR: Self = Self(0o0000002);
    pub const O_CREAT: Self = Self(0o0000100);
    pub const O_TRUNC: Self = Self(0o0001000);
    pub const O_APPEND: Self = Self(0o0002000);
    pub const O_NONBLOCK: Self = Self(0o0004000);

    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn bits(self) -> c_int {
        self.0
    }

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    pub const fn or(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl BitOr for OpenFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

pub const AT_FDCWD: c_int = -100;

// `man 2 openat`:
//
// SYNOPSIS
//        int openat(int dirfd, const char *pathname, int flags, ...
//                   /* mode_t mode */ );
//
// RETURN VALUE
//        On success, open(), openat(), and creat() return the new file descriptor (a nonnegative
//        integer).  On error, -1 is returned and errno is set to indicate the error.
pub unsafe fn openat(
    dirfd: c_int,
    path: &CStr,
    flags: OpenFlags,
    mode: c_int,
) -> Result<c_int, Errno> {
    syscall!(Sysno::openat, dirfd, path.as_ptr(), flags.bits(), mode).map(|fd| fd as c_int)
}
