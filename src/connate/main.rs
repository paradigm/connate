#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

// Implementation of features Rust core expects from libc
//
// This conflicts with anything which uses std, including src/build/main.rs and tests.  To avoid
// this conflict, connate and conctl `mod` this directly rather than `mod`ing it in the shared
// `lib.rs`.
//
// We don't need to use this, just make it visible to the linker.

#[cfg(not(test))]
#[path = "../libc_shim/mod.rs"]
mod libc_shim;

// Internal representations of user-facing config.rs
//
// We don't want the generated file to be loaded by src/build/main.rs lest it cyclically
// depend on itself.  Thus, this file should not be `mod`'d by either build or lib.rs.  Instead,
// connate and conctl `mod` it directly.
#[path = "../internal.rs"]
mod internal;

mod handle_request;
mod handle_signal;
mod next_state;
mod poll;
mod session;
mod setup;
mod spawn;

use crate::handle_request::*;
use crate::handle_signal::*;
use crate::next_state::*;
use crate::poll::*;
use crate::session::*;
use crate::setup::*;
use connate::err::*;
use connate::internal_api::*;
use connate::ipc::*;
use connate::os::*;

/// # Safety
///
/// Platform ABI guarantees incoming C-style format
#[cfg(not(test))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn main(
    _argc: isize,
    _argv: *const *const core::ffi::c_char,
    _envp: *const *const core::ffi::c_char,
) -> isize {
    let now = get_time_monotonic().or_abort("Unable to get current time");

    // Load services from .bss
    //
    // Safety:
    // - This occurs very early on; no threading could occur.
    // - Occurs exactly once
    let svcs = unsafe { internal::SERVICES.initialize(now) };

    // Setup process properties
    //
    // These are idempotent and can be called redundantly when resuming a session
    acquire_lock_file();
    block_signals().or_abort("Unable to block signals");
    set_child_subreaper().or_abort("Unable to set PR_SET_CHILD_SUBREAPER");

    // Resume or initialize file descriptors
    let mut ipc_server = IpcServer::try_resume().unwrap_or_else(IpcServer::new);
    let mut signalfd = resume_or_new_signalfd();
    let mut session_fd = SessionFd::resume_or_new(svcs, &mut ipc_server);

    let mut shutting_down = false;
    let mut poll = Poll::new(&signalfd, &ipc_server);

    // Main loop
    loop {
        let now = get_time_monotonic().or_abort("Unable to get current time");

        // Handle state transitions
        while let Some(i) = svcs.find_dirty_index() {
            NextState::new(svcs, i, now).apply(svcs, i, now);
        }

        // Handle shutting down
        if shutting_down && svcs.all_down_or_err() {
            exit(if svcs.any_bad() { 1 } else { 0 });
        }

        // Sleep until an event occurs, then handle event
        let (timeout_ms, timeout_svc) = calculate_poll_timeout(svcs, now);
        match poll.poll(timeout_ms) {
            PollFdReady::TimeoutExpired => timeout_svc.map_or((), |svc| svc.dirty = true),
            PollFdReady::SignalFd => {
                handle_signal(&mut signalfd, svcs, &mut shutting_down, &mut session_fd)
            }
            PollFdReady::Request => handle_request(svcs, &mut ipc_server, &mut session_fd, now),
        }
    }
}
