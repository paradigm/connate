use crate::os::readlink;
use core::ffi::CStr;
use syscalls::Errno;

pub fn exec_filepath(path: &CStr) -> Result<(), Errno> {
    // Provide an argv[0] so that /proc/<pid>/cmdline shows the binary name
    let argv: [*const core::ffi::c_char; 2] = [path.as_ptr(), core::ptr::null()];

    // SAFETY: argv is a properly null-terminated array of null-terminated C strings
    unsafe { crate::syscall::execve(path, argv.as_ptr(), core::ptr::null()).map(|_| ()) }
}

/// Exec /proc/self/exe to re-initialize binary
///
/// If the file was deleted or overwritten, the symlink will have " (deleted)" appended to it.
/// This is removed exec a potentially new file at the old path.
pub fn exec_self() -> Result<(), Errno> {
    const DELETED_SUFFIX: &[u8] = b" (deleted)";

    let mut buf: [u8; 4096] = [0u8; 4096];

    let mut path_len = readlink(c"/proc/self/exe", &mut buf)?;

    if path_len >= DELETED_SUFFIX.len() {
        let start = path_len - DELETED_SUFFIX.len();
        if buf.get(start..path_len) == Some(DELETED_SUFFIX) {
            path_len -= DELETED_SUFFIX.len();
        }
    }

    *buf.get_mut(path_len).ok_or(Errno::ENAMETOOLONG)? = 0;

    // SAFETY: We explicitly inserted a trailing NUL byte and the kernel never writes interior NULs.
    let pathname = unsafe {
        CStr::from_bytes_with_nul_unchecked(buf.get(..=path_len).ok_or(Errno::ENAMETOOLONG)?)
    };

    exec_filepath(pathname)
}
