use connate::constants::*;
use connate::err::*;
use connate::os::*;

/// Acquire lock file (if configured)
///
/// On re-exec, we need to re-lock the file in case the configured path changed.  The lock
/// subsystem is re-entrant such that the same process locking the same path twice is okay.
pub fn acquire_lock_file() {
    let Some(path) = crate::internal::CONFIG_LOCK_FILE else {
        return;
    };

    let flags = OpenFlags::O_RDWR | OpenFlags::O_CREAT;
    let fd = Fd::open(path, flags, 0o600)
        .or_fs_abort("open", path)
        .move_to(FD_LOCK_FILE)
        .or_abort("Unable to dup lock file FD");

    match fd.lock_nonblocking() {
        Ok(()) => {}
        Err(err) if err == Errno::EACCES || err == Errno::EAGAIN => match fd.get_locking_pid() {
            Ok(Some(pid)) => abort_lock_held_by_pid(path, pid),
            Ok(None) => abort_acquire_lock(path, None),
            Err(err) => abort_acquire_lock(path, Some(err)),
        },
        Err(err) => abort_acquire_lock(path, Some(err)),
    }
}

pub fn resume_or_new_signalfd() -> SignalFd {
    if Fd::from_raw(FD_SIGNAL).is_valid() {
        SignalFd::from_raw(FD_SIGNAL)
    } else {
        SignalFd::new()
            .or_abort("Unable to create signalfd")
            .move_to(FD_SIGNAL)
            .or_abort("Unable to move signalfd")
    }
}
