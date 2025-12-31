use crate::err::*;
use crate::types::*;
use syscalls::{Sysno, syscall};

#[cfg(not(any(
    all(target_os = "linux", target_arch = "aarch64"),
    all(target_os = "linux", target_arch = "x86_64"),
)))]
compile_error!("src/syscall/fstatat.rs only supports Linux x86_64 and Linux AArch64.");

/// File status structure returned by stat/fstatat
#[repr(C)]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[derive(Clone, Copy, Default)]
pub struct Stat {
    pub st_dev: u64,
    pub st_ino: u64,
    pub st_nlink: u64,
    pub st_mode: mode_t,
    pub st_uid: uid_t,
    pub st_gid: gid_t,
    __pad0: u32,
    pub st_rdev: u64,
    pub st_size: i64,
    pub st_blksize: i64,
    pub st_blocks: i64,
    pub st_atime: i64,
    pub st_atime_nsec: i64,
    pub st_mtime: i64,
    pub st_mtime_nsec: i64,
    pub st_ctime: i64,
    pub st_ctime_nsec: i64,
    __unused: [i64; 3],
}
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const _: () = assert!(core::mem::size_of::<Stat>() == 144);

/// File status structure returned by stat/fstatat
#[repr(C)]
#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
#[derive(Clone, Copy, Default)]
pub struct Stat {
    pub st_dev: u64,
    pub st_ino: u64,
    pub st_mode: mode_t,
    pub st_nlink: u32,
    pub st_uid: uid_t,
    pub st_gid: gid_t,
    pub st_rdev: u64,
    __pad1: u64,
    pub st_size: i64,
    pub st_blksize: i32,
    __pad2: i32,
    pub st_blocks: i64,
    pub st_atime: i64,
    pub st_atime_nsec: i64,
    pub st_mtime: i64,
    pub st_mtime_nsec: i64,
    pub st_ctime: i64,
    pub st_ctime_nsec: i64,
    __unused4: u32,
    __unused5: u32,
}
#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
const _: () = assert!(core::mem::size_of::<Stat>() == 128);

// `man 2 fstatat`:
//
// SYNOPSIS
//        int fstatat(int dirfd, const char *restrict pathname,
//                    struct stat *restrict statbuf, int flags);
//
// RETURN VALUE
//        On success, zero is returned.  On error, -1 is returned, and errno is set to
//        indicate the error.
pub unsafe fn fstatat(
    dirfd: c_int,
    pathname: &CStr,
    statbuf: &mut Stat,
    flags: c_int,
) -> Result<(), Errno> {
    syscall!(
        Sysno::newfstatat,
        dirfd,
        pathname.as_ptr() as usize,
        statbuf as *mut Stat as usize,
        flags
    )
    .map(|_| ())
}
