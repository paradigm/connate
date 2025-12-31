use crate::err::*;
use crate::syscall::{SigprocmaskHow, sigprocmask};
use crate::types::*;

#[inline]
pub fn block_signals() -> Result<(), Errno> {
    let sigset = sigset_t::new_full_set();
    unsafe { sigprocmask(SigprocmaskHow::SIG_BLOCK, &sigset) }
}

#[inline]
pub fn unblock_all_signals() -> Result<(), Errno> {
    let sigset = sigset_t::new_empty_set();
    unsafe { sigprocmask(SigprocmaskHow::SIG_SETMASK, &sigset) }
}
