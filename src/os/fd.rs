use crate::err::*;
use crate::syscall::*;
use crate::types::*;

pub const STDIN: Fd = Fd(0);
pub const STDOUT: Fd = Fd(1);
pub const STDERR: Fd = Fd(2);

pub use crate::syscall::{MemfdFlags, OpenFlags, SeekWhence};

/// File descriptor
#[derive(Clone)]
pub struct Fd(c_int);

impl Fd {
    pub fn open(path: &CStr, flags: OpenFlags, mode: c_int) -> Result<Self, Errno> {
        unsafe { openat(AT_FDCWD, path, flags, mode).map(Self) }
    }

    pub fn read(&self, buf: &mut [u8]) -> Result<usize, Errno> {
        unsafe { read(self.0, buf) }
    }

    pub fn write(&self, buf: &[u8]) -> Result<usize, Errno> {
        unsafe { write(self.0, buf) }
    }

    pub fn close(self) -> Result<(), Errno> {
        unsafe { close(self.0) }
    }

    pub fn ftruncate(&self, length: off_t) -> Result<(), Errno> {
        unsafe { ftruncate(self.0, length) }
    }

    pub fn lseek(&self, offset: off_t, whence: SeekWhence) -> Result<off_t, Errno> {
        unsafe { lseek(self.0, offset, whence) }
    }

    pub fn lock_nonblocking(&self) -> Result<(), Errno> {
        let mut flock = Flock {
            l_type: FlockType::F_WRLCK,
            l_whence: FlockWhence::SEEK_SET,
            l_start: 0,
            l_len: 0,
            l_pid: 0,
        };

        unsafe { fcntl(self.0, FcntlCmd::F_SETLK, &mut flock).map(|_| ()) }
    }

    pub fn lock_blocking(&self) -> Result<(), Errno> {
        let mut flock = Flock {
            l_type: FlockType::F_WRLCK,
            l_whence: FlockWhence::SEEK_SET,
            l_start: 0,
            l_len: 0,
            l_pid: 0,
        };

        unsafe { fcntl(self.0, FcntlCmd::F_SETLKW, &mut flock).map(|_| ()) }
    }

    pub fn unlock(&self) -> Result<(), Errno> {
        let mut flock = Flock {
            l_type: FlockType::F_UNLCK,
            l_whence: FlockWhence::SEEK_SET,
            l_start: 0,
            l_len: 0,
            l_pid: 0,
        };

        unsafe { fcntl(self.0, FcntlCmd::F_SETLK, &mut flock).map(|_| ()) }
    }

    pub fn get_locking_pid(&self) -> Result<Option<pid_t>, Errno> {
        let mut flock = Flock {
            l_type: FlockType::F_WRLCK,
            l_whence: FlockWhence::SEEK_SET,
            l_start: 0,
            l_len: 0,
            l_pid: 0,
        };

        let cmd = FcntlCmd::F_GETLK;

        unsafe { fcntl(self.0, cmd, &mut flock) }?;

        match flock.l_type {
            FlockType::F_UNLCK => Ok(None),
            _ => Ok(Some(flock.l_pid)),
        }
    }

    pub fn set_blocking(&self) -> Result<(), Errno> {
        // Get current flags
        let flags = unsafe { fcntl_flags(self.0, FcntlCmd::F_GETFL, 0) }?;

        // Clear O_NONBLOCK
        let new_flags = flags & !(OpenFlags::O_NONBLOCK.bits());

        // Set new flags
        unsafe { fcntl_flags(self.0, FcntlCmd::F_SETFL, new_flags) }?;

        Ok(())
    }

    pub fn isatty(&self) -> bool {
        // If ioctl errors, it's not a terminal; otherwise, it is.
        //
        // We need to provide a valid buffer for the termios structure, even though we don't use it.
        let mut buf = [0u8; 64];
        unsafe { ioctl(self.0, IoctlRequest::TCGETS, buf.as_mut_ptr() as usize) }.is_ok()
    }

    pub fn is_valid(&self) -> bool {
        // fcntl F_GETFL returns EBADF for invalid FDs; works for all FD types including pipes
        unsafe { fcntl_flags(self.0, FcntlCmd::F_GETFL, 0) }.is_ok()
    }

    pub fn dup(&self, new_fd: c_int, flags: OpenFlags) -> Result<Self, Errno> {
        unsafe { dup3(self.0, new_fd, flags).map(Self) }
    }

    pub fn into_raw(self) -> c_int {
        self.0
    }

    pub fn from_raw(fd: c_int) -> Self {
        Self(fd)
    }

    pub fn move_to(self, new_fd: c_int) -> Result<Self, Errno> {
        let new = self.dup(new_fd, OpenFlags::empty())?;
        self.close()?;
        Ok(new)
    }

    pub fn as_raw(&self) -> c_int {
        self.0
    }

    pub fn new_pipe(flags: OpenFlags) -> Result<(Self, Self), Errno> {
        let mut fds: [c_int; 2] = [0, 0];
        unsafe { pipe2(&mut fds, flags)? };
        Ok((Self::from_raw(fds[0]), Self::from_raw(fds[1])))
    }

    pub fn new_memfd(name: &CStr, flags: MemfdFlags) -> Result<Self, Errno> {
        unsafe { memfd_create(name, flags).map(Self) }
    }
}

// Naively, one might expect us to close an Fd on drop.  However, we don't actually want this.  It
// is important for some of connate's file descriptors to remain open even if the process never
// accesses them directly.
//
// For example, connate only directly accesses one end of some pipes, with the other accessed
// externally via `/proc/<pid>/fd/<fd>`.  Thus the remote end of the pipe needs to be kept open.
//
// impl Drop for Fd {
//     fn drop(&mut self) {
//         self.close();
//     }
// }
