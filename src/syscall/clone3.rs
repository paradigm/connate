use crate::err::*;
use crate::types::pid_t;
use syscalls::{syscall, Sysno};

pub use crate::syscall::clone::{CloneFlags, CloneResult};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Clone3Args {
    pub flags: u64,
    pub pidfd: u64,
    pub child_tid: u64,
    pub parent_tid: u64,
    pub exit_signal: u64,
    pub stack: u64,
    pub stack_size: u64,
    pub tls: u64,
    pub set_tid: u64,
    pub set_tid_size: u64,
    pub cgroup: u64,
}
const _: () = assert!(core::mem::size_of::<Clone3Args>() == 88);

impl Clone3Args {
    pub const fn new() -> Self {
        Self {
            flags: 0,
            pidfd: 0,
            child_tid: 0,
            parent_tid: 0,
            exit_signal: 0,
            stack: 0,
            stack_size: 0,
            tls: 0,
            set_tid: 0,
            set_tid_size: 0,
            cgroup: 0,
        }
    }

    pub const fn with_flags(mut self, flags: CloneFlags) -> Self {
        self.flags = flags.bits();
        self
    }

    pub const fn with_exit_signal(mut self, signal: u64) -> Self {
        self.exit_signal = signal;
        self
    }

    pub fn with_stack(mut self, stack: *mut u8, stack_size: usize) -> Self {
        self.stack = stack as u64;
        self.stack_size = stack_size as u64;
        self
    }

    pub const fn with_tls(mut self, tls: usize) -> Self {
        self.tls = tls as u64;
        self
    }

    pub fn with_pidfd(mut self, pidfd: *mut i32) -> Self {
        self.pidfd = pidfd as u64;
        self
    }

    pub fn with_parent_tid(mut self, parent_tid: *mut i32) -> Self {
        self.parent_tid = parent_tid as u64;
        self
    }

    pub fn with_child_tid(mut self, child_tid: *mut i32) -> Self {
        self.child_tid = child_tid as u64;
        self
    }
}

impl Default for Clone3Args {
    fn default() -> Self {
        Self::new()
    }
}

pub unsafe fn clone3(args: &Clone3Args) -> Result<CloneResult, Errno> {
    let pid = syscall!(
        Sysno::clone3,
        args as *const Clone3Args,
        core::mem::size_of::<Clone3Args>()
    )? as pid_t;

    if pid == 0 {
        Ok(CloneResult::Child)
    } else {
        Ok(CloneResult::Parent(pid))
    }
}
