//! System call types that are strongly associated with specific system calls are defined in the
//! corresponding system call file.  Those shared across many are defined here.

/// When serializing strings, a prefix used to indicate their length
pub type StrLen = u16;

#[allow(non_camel_case_types)]
pub type pid_t = i32;

#[allow(non_camel_case_types)]
pub type uid_t = u32;

#[allow(non_camel_case_types)]
pub type gid_t = u32;

#[allow(non_camel_case_types)]
pub type mode_t = u32;

#[allow(non_camel_case_types)]
pub type off_t = i64;

#[allow(non_camel_case_types)]
pub type c_int = core::ffi::c_int;

#[allow(non_camel_case_types)]
pub type c_short = core::ffi::c_short;

#[allow(non_camel_case_types)]
pub type c_char = core::ffi::c_char;

pub type CStr = core::ffi::CStr;

#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct timespec {
    pub tv_sec: i64,
    pub tv_nsec: i64,
}
const _: () = assert!(core::mem::size_of::<timespec>() == 16);

impl timespec {
    pub fn millis_since(self, earlier: timespec) -> i64 {
        self.tv_sec
            .wrapping_sub(earlier.tv_sec)
            .saturating_mul(1000)
            .saturating_add(self.tv_nsec.wrapping_sub(earlier.tv_nsec) / 1_000_000)
    }
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Copy, PartialEq)]
#[repr(u32)]
// Only implementing the ones we care about to make matches simpler
pub enum Signal {
    SIGHUP = 1,
    SIGINT = 2,
    // SIGQUIT = 3,
    // SIGILL = 4,
    // SIGTRAP = 5,
    // SIGABRT = 6,
    // SIGBUS = 7,
    // SIGFPE = 8,
    SIGKILL = 9,
    // SIGUSR1 = 10,
    // SIGSEGV = 11,
    // SIGUSR2 = 12,
    // SIGPIPE = 13,
    // SIGALRM = 14,
    SIGTERM = 15,
    // SIGSTKFLT = 16,
    SIGCHLD = 17,
    UNRECOGNIZED = u32::MAX,
}

impl Signal {
    pub fn as_bitmask(self) -> usize {
        debug_assert!(self != Self::UNRECOGNIZED);

        1 << (self as usize - 1)
    }
}

/// Documentation on this type has inconsistent descriptions of its size and format.
/// Possibilities appear to be:
/// - It it should be 1024 bits, one for each 1024 signals
/// - It it should be more than 1024 bits to track information other than signals, but only pass
///   the first 1024 bits to the kernel.
/// - It it should be the machine pointer size.
///
/// In testing, the last was the only one that didn't get an EINVAL from the Linux kernel.  However,
/// more research should be done here to clarify the matter.
#[allow(non_camel_case_types)]
#[repr(C)]
pub struct sigset_t(usize);

impl sigset_t {
    pub fn new_empty_set() -> Self {
        Self(0)
    }

    pub fn new_full_set() -> Self {
        Self(!0)
    }
}

impl core::ops::BitOr<Signal> for sigset_t {
    type Output = Self;

    fn bitor(self, signal: Signal) -> Self::Output {
        Self(self.0 | signal.as_bitmask())
    }
}

impl core::ops::BitOrAssign<Signal> for sigset_t {
    fn bitor_assign(&mut self, signal: Signal) {
        self.0 |= signal.as_bitmask();
    }
}

impl core::ops::BitAnd<Signal> for sigset_t {
    type Output = Self;

    fn bitand(self, signal: Signal) -> Self::Output {
        Self(self.0 & signal.as_bitmask())
    }
}

pub trait PidParse {
    fn parse_pid(&self) -> Result<pid_t, crate::err::Errno>;
}

impl PidParse for &[u8] {
    fn parse_pid(&self) -> Result<pid_t, crate::err::Errno> {
        if self.is_empty() {
            return Err(crate::err::Errno::EINVAL);
        }

        let mut result: pid_t = 0;
        for &b in *self {
            if !b.is_ascii_digit() {
                return Err(crate::err::Errno::EINVAL);
            }
            result = result
                .checked_mul(10)
                .and_then(|r| r.checked_add((b - b'0') as pid_t))
                .ok_or(crate::err::Errno::EINVAL)?;
        }
        Ok(result)
    }
}

impl PidParse for &CStr {
    fn parse_pid(&self) -> Result<pid_t, crate::err::Errno> {
        self.to_bytes().parse_pid()
    }
}

/// Atomic Option<pid_t>
pub struct AtomicOptionPid(core::sync::atomic::AtomicI32);
impl AtomicOptionPid {
    pub const fn new_none() -> Self {
        Self(core::sync::atomic::AtomicI32::new(0))
    }

    pub fn new(pid: Option<pid_t>) -> Self {
        let pid = pid.unwrap_or(0);
        Self(core::sync::atomic::AtomicI32::new(pid))
    }

    pub fn get(&self) -> Option<pid_t> {
        let pid = self.0.load(core::sync::atomic::Ordering::Acquire);
        if pid == 0 { None } else { Some(pid) }
    }

    pub fn clear(&self) {
        self.0.store(0, core::sync::atomic::Ordering::Release);
    }

    pub fn set(&self, pid: Option<pid_t>) {
        let pid = pid.unwrap_or(0);
        self.0.store(pid, core::sync::atomic::Ordering::Release);
    }

    pub fn swap(&self, new_pid: Option<pid_t>) -> Option<pid_t> {
        let new_val = new_pid.unwrap_or(0);
        let old_val = self.0.swap(new_val, core::sync::atomic::Ordering::AcqRel);
        if old_val == 0 { None } else { Some(old_val) }
    }

    pub fn take(&self) -> Option<pid_t> {
        self.swap(None)
    }
}

/// Atomic bool
pub struct AtomicBool(core::sync::atomic::AtomicBool);
impl AtomicBool {
    pub const fn new(val: bool) -> Self {
        Self(core::sync::atomic::AtomicBool::new(val))
    }

    pub fn get(&self) -> bool {
        self.0.load(core::sync::atomic::Ordering::Acquire)
    }

    pub fn set(&self, val: bool) {
        self.0.store(val, core::sync::atomic::Ordering::Release);
    }

    pub fn swap(&self, new_val: bool) -> bool {
        self.0.swap(new_val, core::sync::atomic::Ordering::AcqRel)
    }
}
