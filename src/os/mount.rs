use crate::err::*;
use crate::types::*;

pub use crate::syscall::MountFlags;

#[inline]
pub fn mount(
    source: Option<&CStr>,
    target: &CStr,
    filesystemtype: Option<&CStr>,
    mountflags: MountFlags,
    data: Option<&CStr>,
) -> Result<(), Errno> {
    unsafe { crate::syscall::mount(source, target, filesystemtype, mountflags, data) }
}
