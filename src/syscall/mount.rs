use crate::err::*;
use core::ffi::CStr;
use core::ops::BitOr;
use syscalls::{Sysno, syscall};

#[derive(Clone, Copy)]
pub struct MountFlags(u64);

impl MountFlags {
    pub const MS_RDONLY: Self = Self(0o1);
    pub const MS_NOSUID: Self = Self(0o2);
    pub const MS_NODEV: Self = Self(0o4);
    pub const MS_NOEXEC: Self = Self(0o10);
    pub const MS_SYNCHRONOUS: Self = Self(0o20);
    pub const MS_REMOUNT: Self = Self(0o40);
    pub const MS_MANDLOCK: Self = Self(0o100);
    pub const MS_DIRSYNC: Self = Self(0o200);
    pub const MS_NOATIME: Self = Self(0o2000);
    pub const MS_NODIRATIME: Self = Self(0o4000);
    pub const MS_BIND: Self = Self(0o10000);
    pub const MS_MOVE: Self = Self(0o20000);
    pub const MS_REC: Self = Self(0o40000);
    pub const MS_SILENT: Self = Self(0o100000);
    pub const MS_POSIXACL: Self = Self(0o200000);
    pub const MS_UNBINDABLE: Self = Self(0o400000);
    pub const MS_PRIVATE: Self = Self(0o1000000);
    pub const MS_SLAVE: Self = Self(0o2000000);
    pub const MS_SHARED: Self = Self(0o4000000);
    pub const MS_RELATIME: Self = Self(0o10000000);
    pub const MS_KERNMOUNT: Self = Self(0o20000000);
    pub const MS_I_VERSION: Self = Self(0o40000000);
    pub const MS_STRICTATIME: Self = Self(0o100000000);
    pub const MS_LAZYTIME: Self = Self(0o200000000);

    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn bits(self) -> u64 {
        self.0
    }

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

impl BitOr for MountFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

// `man 2 mount`:
//
// SYNOPSIS
//        int mount(const char *source, const char *target,
//                  const char *filesystemtype, unsigned long mountflags,
//                  const void *data);
//
// RETURN VALUE
//        On success, zero is returned. On error, -1 is returned, and errno is set appropriately.
pub unsafe fn mount(
    source: Option<&CStr>,
    target: &CStr,
    filesystemtype: Option<&CStr>,
    mountflags: MountFlags,
    data: Option<&CStr>,
) -> Result<(), Errno> {
    syscall!(
        Sysno::mount,
        source.map_or(core::ptr::null(), |s| s.as_ptr()),
        target.as_ptr(),
        filesystemtype.map_or(core::ptr::null(), |s| s.as_ptr()),
        mountflags.bits(),
        data.map_or(core::ptr::null(), |s| s.as_ptr())
    )
    .map(|_| ())
}
