use crate::err::*;
use crate::types::{c_int, pid_t};
use core::ops::BitOr;
use syscalls::{syscall, Sysno};

#[derive(Clone, Copy)]
pub struct CloneFlags(u64);

impl CloneFlags {
    pub const CSIGNAL: Self = Self(0x000000ff);
    pub const CLONE_VM: Self = Self(0x00000100);
    pub const CLONE_FS: Self = Self(0x00000200);
    pub const CLONE_FILES: Self = Self(0x00000400);
    pub const CLONE_SIGHAND: Self = Self(0x00000800);
    pub const CLONE_PIDFD: Self = Self(0x00001000);
    pub const CLONE_PTRACE: Self = Self(0x00002000);
    pub const CLONE_VFORK: Self = Self(0x00004000);
    pub const CLONE_PARENT: Self = Self(0x00008000);
    pub const CLONE_THREAD: Self = Self(0x00010000);
    pub const CLONE_NEWNS: Self = Self(0x00020000);
    pub const CLONE_SYSVSEM: Self = Self(0x00040000);
    pub const CLONE_SETTLS: Self = Self(0x00080000);
    pub const CLONE_PARENT_SETTID: Self = Self(0x00100000);
    pub const CLONE_CHILD_CLEARTID: Self = Self(0x00200000);
    pub const CLONE_DETACHED: Self = Self(0x00400000);
    pub const CLONE_UNTRACED: Self = Self(0x00800000);
    pub const CLONE_CHILD_SETTID: Self = Self(0x01000000);
    pub const CLONE_NEWCGROUP: Self = Self(0x02000000);
    pub const CLONE_NEWUTS: Self = Self(0x04000000);
    pub const CLONE_NEWIPC: Self = Self(0x08000000);
    pub const CLONE_NEWUSER: Self = Self(0x10000000);
    pub const CLONE_NEWPID: Self = Self(0x20000000);
    pub const CLONE_NEWNET: Self = Self(0x40000000);
    pub const CLONE_IO: Self = Self(0x80000000);
    pub const CLONE_CLEAR_SIGHAND: Self = Self(0x100000000);
    pub const CLONE_INTO_CGROUP: Self = Self(0x200000000);

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

impl BitOr for CloneFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloneResult {
    Parent(pid_t),
    Child,
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn clone(
    flags: CloneFlags,
    stack: *mut u8,
    parent_tid: *mut c_int,
    child_tid: *mut c_int,
    tls: usize,
) -> Result<CloneResult, Errno> {
    let pid = syscall!(
        Sysno::clone,
        flags.bits(),
        stack,
        parent_tid,
        child_tid,
        tls
    )? as pid_t;

    if pid == 0 {
        Ok(CloneResult::Child)
    } else {
        Ok(CloneResult::Parent(pid))
    }
}

#[cfg(target_arch = "aarch64")]
pub unsafe fn clone(
    flags: CloneFlags,
    stack: *mut u8,
    parent_tid: *mut c_int,
    child_tid: *mut c_int,
    tls: usize,
) -> Result<CloneResult, Errno> {
    let pid = syscall!(
        Sysno::clone,
        flags.bits(),
        stack,
        parent_tid,
        tls,
        child_tid
    )? as pid_t;

    if pid == 0 {
        Ok(CloneResult::Child)
    } else {
        Ok(CloneResult::Parent(pid))
    }
}
