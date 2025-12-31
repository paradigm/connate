use crate::err::*;
use crate::syscall::AT_FDCWD;
use crate::types::*;

pub use crate::syscall::Stat;

/// File type mask for st_mode
pub const S_IFMT: mode_t = 0o170000;

/// Directory file type bit
pub const S_IFDIR: mode_t = 0o040000;

/// User execute permission bit
pub const S_IXUSR: mode_t = 0o100;

/// Group execute permission bit
pub const S_IXGRP: mode_t = 0o010;

/// Other execute permission bit
pub const S_IXOTH: mode_t = 0o001;

#[inline]
pub fn stat(path: &CStr) -> Result<Stat, Errno> {
    let mut statbuf = Stat::default();
    // SAFETY: We pass a valid mutable reference to statbuf, and AT_FDCWD with pathname
    // behaves like classic stat(). The kernel will populate statbuf on success.
    unsafe { crate::syscall::fstatat(AT_FDCWD, path, &mut statbuf, 0) }?;
    Ok(statbuf)
}

#[inline]
pub fn is_dir(path: &CStr) -> Result<bool, Errno> {
    let statbuf = stat(path)?;
    Ok((statbuf.st_mode & S_IFMT) == S_IFDIR)
}

#[inline]
pub fn is_executable(path: &CStr) -> Result<bool, Errno> {
    let statbuf = stat(path)?;
    Ok((statbuf.st_mode & (S_IXUSR | S_IXGRP | S_IXOTH)) != 0)
}
