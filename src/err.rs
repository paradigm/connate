//! # Error handling
//!
//! Three unrecoverable scenarios to consider:
//!
//! - connate as PID 1 should never exit, only ever exec()
//! - connate as non-PID 1 should just display message and exit
//! - conctl should always just display message and exit (as it should never be PID 1)
//!
//! Connate as PID1 will handle unrecoverable errors by trying to re-exec every 60 seconds.  This
//! keeps PID 1 alive (and thus the rest of the system) and retains the possibility, however small,
//! to recover the system.
//!
//! Our print machinery does not support typical Rust `{}`-formatting.  Instead, we have
//! specialized functions here for expected multi-field errors such as printing the file path
//! associated with an error message.

use crate::os::*;
use crate::types::*;

pub type Errno = syscalls::Errno;

/// - If pid 1 (only occurs as connate), try to re-exec every 60 seconds
/// - Otherwise, exit immediately
fn abort() -> ! {
    if getpid() != 1 {
        exit(1);
    }

    loop {
        eprintln("");
        eprintln("ERROR: Hit unrecoverable scenario while running as PID 1.");
        eprintln("Attempt to save data immediately!");
        eprintln("Will attempt to re-exec /proc/self/exe after 60 seconds.");
        eprintln("If you can ensure /proc is mounted and replace original executable with fix,");
        eprintln("system may potentially be recovered.");
        eprintln("Otherwise, a hard restart may be required.");

        let _ = sleep(60);

        // Safety:
        // - This is only accessible in this function
        // - We never fork threads from this point
        // let _ = reexec_self();
        // TODO!("re-exec-self");

        // Re-exec failed.  Will loop and try again.
    }
}

pub fn abort_with_msg(msg: &str) -> ! {
    eprint("ERROR: ");
    eprint(msg);
    eprint("\n");

    abort()
}

pub trait OrAbortResult<T> {
    fn or_abort<M: Print>(self, msg: M) -> T;
    fn or_fs_abort(self, operation: &str, path: &CStr) -> T;
}

impl<T> OrAbortResult<T> for Result<T, Errno> {
    fn or_abort<M: Print>(self, msg: M) -> T {
        let e = match self {
            Ok(t) => return t,
            Err(e) => e,
        };

        eprint("ERROR: ");
        eprint(msg);
        if let Some(e) = e.description() {
            eprint(": ");
            eprint(e);
        }
        eprint("\n");

        abort();
    }

    fn or_fs_abort(self, operation: &str, path: &CStr) -> T {
        let e = match self {
            Ok(t) => return t,
            Err(e) => e,
        };

        eprint("ERROR: Unable to ");
        eprint(operation);
        eprint(" ");
        eprint(path);
        if let Some(e) = e.description() {
            eprint(": ");
            eprint(e);
        }
        eprint("\n");

        abort();
    }
}

pub trait OrAbortOption<T> {
    fn or_abort<M: Print>(self, msg: M) -> T;
    fn or_fs_abort(self, operation: &str, path: &CStr) -> T;
}

impl<T> OrAbortOption<T> for Option<T> {
    fn or_abort<M: Print>(self, msg: M) -> T {
        if let Some(t) = self {
            return t;
        };

        eprint("ERROR: ");
        eprint(msg);
        eprint("\n");

        abort();
    }

    fn or_fs_abort(self, operation: &str, path: &CStr) -> T {
        if let Some(t) = self {
            return t;
        };

        eprint("ERROR: Unable to ");
        eprint(operation);
        eprint(" ");
        eprint(path);
        eprint("\n");

        abort();
    }
}

pub fn abort_lock_held_by_pid(path: &CStr, pid: pid_t) -> ! {
    eprint("ERROR: Lock path ");
    eprint(path);
    eprint(" is already held by PID ");
    eprint(pid);
    eprintln(". Is another connate process running with that lock file?");

    abort();
}

pub fn abort_acquire_lock(path: &CStr, errno: Option<Errno>) -> ! {
    eprint("ERROR: Unable to acquire lock at ");
    eprint(path);

    if let Some(e) = errno.and_then(|e| e.description()) {
        eprint(": ");
        eprint(e);
    }

    eprintln("");

    abort();
}
