use crate::err::*;
use syscalls::{Sysno, syscall};

pub unsafe fn sync() -> Result<(), Errno> {
    syscall!(Sysno::sync).map(|_| ())
}
