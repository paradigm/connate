use crate::err::Errno;
use crate::types::*;
use core::ops::BitOr;
use syscalls::{Sysno, syscall};

#[repr(C)]
pub struct WaitIdInfo {
    si_signo: i32, // offset 0
    si_errno: i32, // offset 4
    si_code: i32,  // offset 8
    _pad0: i32,    // offset 12 (padding for union alignment)
    // SIGCHLD fields in the union (starts at offset 16):
    si_pid: i32,    // offset 16
    si_uid: u32,    // offset 20
    si_status: i32, // offset 24
    _pad1: i32,     // offset 28 (padding)
    si_utime: i64,  // offset 32
    si_stime: i64,  // offset 40
    // Padding to reach 128 bytes total
    _pad2: [u8; 80], // offset 48, size 80 (128 - 48 = 80)
}
const _: () = assert!(core::mem::size_of::<WaitIdInfo>() == 128);

impl Default for WaitIdInfo {
    fn default() -> Self {
        Self::new()
    }
}

impl WaitIdInfo {
    pub fn new() -> Self {
        Self {
            si_signo: 0,
            si_errno: 0,
            si_code: 0,
            _pad0: 0,
            si_pid: 0,
            si_uid: 0,
            si_status: 0,
            _pad1: 0,
            si_utime: 0,
            si_stime: 0,
            _pad2: [0; 80],
        }
    }

    pub fn pid(&self) -> pid_t {
        self.si_pid
    }

    pub fn status(&self) -> i32 {
        self.si_status
    }
}

/// IdType for waitid()
#[allow(non_camel_case_types)]
#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum IdType {
    /// Wait for any child
    P_ALL = 0,
    /// Wait for child with specific PID
    P_PID = 1,
    /// Wait for children with specific PGID
    P_PGID = 2,
}

/// Options for waitid
#[derive(Clone, Copy)]
pub struct WaitIdOptions(c_int);

impl WaitIdOptions {
    /// Wait for children that have exited
    pub const WEXITED: Self = Self(4);
    /// Wait for children that have stopped
    pub const WSTOPPED: Self = Self(2);
    /// Wait for children that have continued
    pub const WCONTINUED: Self = Self(8);
    /// Return immediately if no child has exited
    pub const WNOHANG: Self = Self(1);
    /// Don't reap the child (leave it in a waitable state)
    pub const WNOWAIT: Self = Self(0x0100_0000);

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

impl BitOr for WaitIdOptions {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

/// Wait for a child process to change state
///
/// idtype specifies which children to wait for (P_ALL, P_PID, P_PGID)
/// id specifies the specific pid/pgid if idtype is P_PID or P_PGID (ignored for P_ALL)
/// infop is filled with information about the child
/// options specifies wait options (WEXITED, WSTOPPED, WCONTINUED, WNOHANG, WNOWAIT)
///
/// Returns Ok(()) on success with infop filled, or Err(errno) on failure
pub unsafe fn waitid(
    idtype: IdType,
    id: pid_t,
    infop: &mut WaitIdInfo,
    options: WaitIdOptions,
) -> Result<(), Errno> {
    let infop_ptr = infop as *mut WaitIdInfo;

    // The raw waitid syscall takes 5 arguments; the 5th is a rusage pointer (NULL if not needed)
    syscall!(
        Sysno::waitid,
        idtype as isize,
        id as isize,
        infop_ptr,
        options.bits() as isize,
        0isize // rusage pointer = NULL
    )
    .map(|_| ())
}
