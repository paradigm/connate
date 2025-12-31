//! Service state transition logic
//!
//! Supervisor process behavior to keep in mind:
//! - When connate wants to spawn_* a service that needs multi-process tracking, it first forks off
//!   a supervisor process.
//! - The supervisor sends a Request::ServiceStarting with the new service's PID
//! - There is a brief expected window during Starting where
//!   `svc.supervisor_pid().is_some() && svc.pid.is_none()`
//! - This is the only window where the supervisor should exist without the service as its child:
//!   - If the supervisor fails to start the service, it exits.
//!   - If the supervisor detects the service died expectedly, it cleans up then exits.
//!   - If the supervisor detects the child died unexpectedly, it cleans up then exits.
//! - We can assume if the service dies (e.g. we send it SIGKILL) the supervisor will die as well.

use crate::spawn::*;
use connate::constants::*;
use connate::internal_api::*;
use connate::os::*;
use connate::types::*;

pub enum NextState {
    // Change svc.state
    Down,
    WaitingToStart,
    SettingUp,
    Starting,
    Up,
    WaitingToStop,
    Stopping,
    CleaningUp,
    ForceDown,
    FailedOrRetry,
    CannotStop,
    // Retain state
    None,     // no change
    UpStable, // stable; reset retry count
}

impl NextState {
    pub fn new<const N: usize>(svcs: &[Service; N], i: usize, now: timespec) -> Self {
        let Some(svc) = svcs.get(i) else {
            return Self::None;
        };

        match svc.state {
            State::Down => Self::from_down(svc),
            State::WaitingToStart => Self::from_waiting_to_start(svc, svcs),
            State::SettingUp => Self::from_setting_up(svc, now),
            State::Starting => Self::from_starting(svc, now),
            State::Up => Self::from_up(svc, now),
            State::WaitingToStop => Self::from_waiting_to_stop(svc, svcs),
            State::Stopping => Self::from_stopping(svc, now),
            State::CleaningUp => Self::from_cleaning_up(svc, now),
            State::ForceDown => Self::from_forced_down(svc, now),
            State::Retrying => Self::from_retrying(svc, now),
            State::Failed => Self::from_failed(svc),
            State::CannotStop => Self::from_cannot_stop(svc),
        }
    }

    pub fn apply<const N: usize>(self, svcs: &mut [Service; N], i: usize, now: timespec) {
        // Immutable read all svcs to get logger_fd, then once we have it get the service we are
        // interested in as mutable.
        let Some(svc) = svcs.get(i) else {
            return;
        };
        let logger_fd = svc.logger_fd(svcs);
        let Some(svc) = svcs.get_mut(i) else {
            return;
        };

        match self {
            // Change svc.state
            Self::Down => apply_down(svc),
            Self::WaitingToStart => apply_waiting_to_start(svc),
            Self::SettingUp => apply_setting_up(svc, logger_fd),
            Self::Starting => apply_starting(svc, logger_fd),
            Self::Up => apply_up(svc),
            Self::WaitingToStop => apply_waiting_to_stop(svc),
            Self::Stopping => apply_stopping(svc),
            Self::CleaningUp => apply_cleaning_up(svc, logger_fd),
            Self::FailedOrRetry => apply_failed_or_retry(svc),
            Self::ForceDown => apply_force_down(svc),
            Self::CannotStop => apply_cannot_stop(svc),
            // Retain state but do something
            Self::None => svc.dirty = false,
            Self::UpStable => {
                svc.dirty = false;
                svc.attempt_count = 0;
            }
        }

        // Set common items for when the service state actually changes
        match self {
            Self::None | Self::UpStable => {}
            _ => {
                // Update time since state change if the state changed
                svc.time = now;
                // If `.ready` was meaningful, it would have been consumed in the apply_* above.
                svc.ready = false;
                // If this service's state changed, there may be another following change available.
                svc.dirty = true;
                // If this service state changed, services waiting on this service may no longer be
                // blocked and need to be re-checked.
                for &i in svc.cfg.propagate_dirty {
                    if let Some(svc) = svcs.get_mut(i) {
                        svc.dirty = true;
                    }
                }
            }
        }
    }

    fn from_down(svc: &Service) -> Self {
        match svc.target {
            _ if svc.has_pid() => Self::ForceDown, // Stop unexpected process
            Target::Down => Self::None,
            // apply_down() performs Restart->Up transition. If we're down and target=Restart, the
            // user set it while we're down.
            //
            // Continuing upward doesn't make sense with target=Restart. Instead, apply_down()
            // again to apply the Restart->Up target transition.
            Target::Restart => Self::Down,
            // apply_down() performs Once->Down transition. If we're down and target=Once, the
            // user set it while we're down.
            //
            // If we're down and target=Once, respect the user's request and continue upward.
            Target::Up | Target::Once => Self::WaitingToStart,
        }
    }

    fn from_waiting_to_start<const N: usize>(svc: &Service, svcs: &[Service; N]) -> Self {
        match svc.target {
            _ if svc.has_pid() => Self::ForceDown, // Stop unexpected process
            Target::Down | Target::Restart => Self::Down,
            Target::Up | Target::Once if start_dep_satisfied(svc, svcs) => Self::SettingUp,
            Target::Up | Target::Once => Self::None,
        }
    }

    fn from_setting_up(svc: &Service, now: timespec) -> Self {
        // Once we're in SettingUp, target doesn't matter until we transition through to Up.
        // When Up, we can consider continuing downward if target is downward.

        match svc.cfg.setup {
            Run::None if svc.has_pid() => Self::ForceDown, // Stop unexpected process
            Run::None => Self::Starting,
            _ if svc.pid.is_none() && svc.exit_code == Some(0) => Self::Starting,
            _ if svc.pid.is_none() => Self::FailedOrRetry,
            _ if svc.pid.is_some() && setup_time_elapsed(svc, now) => Self::ForceDown,
            _ => Self::None,
        }
    }

    fn from_starting(svc: &Service, now: timespec) -> Self {
        // Once we're in Staring, target doesn't matter until we transition through to Up.
        // When Up, we can consider continuing downward if target is downward.

        match svc.cfg.run {
            Run::None if svc.has_pid() => Self::ForceDown, // Stop unexpected process
            Run::None => Self::Up,
            _ if !svc.has_pid() => Self::FailedOrRetry,
            _ if matches!(svc.cfg.ready, Ready::Immediately) => Self::Up,
            _ if svc.ready => Self::Up,
            _ if start_time_elapsed(svc, now) => Self::ForceDown,
            _ => Self::None,
        }
    }

    fn from_up(svc: &Service, now: timespec) -> Self {
        if matches!(svc.target, Target::Down | Target::Restart) {
            return Self::WaitingToStop;
        }

        match svc.cfg.run {
            Run::None if svc.has_pid() => Self::ForceDown, // Stop unexpected process
            Run::None => Self::None,
            _ if !svc.has_pid() => Self::FailedOrRetry,
            _ if svc.attempt_count > 0 && up_time_elapsed(svc, now) => Self::UpStable,
            _ => Self::None,
        }
    }

    fn from_waiting_to_stop<const N: usize>(svc: &Service, svcs: &[Service; N]) -> Self {
        match svc.target {
            Target::Up | Target::Once => Self::Up,
            Target::Down | Target::Restart if stop_deps_satisfied(svc, svcs) => Self::Stopping,
            Target::Down | Target::Restart => Self::None,
        }
    }

    fn from_stopping(svc: &Service, now: timespec) -> Self {
        if !svc.has_pid() {
            Self::CleaningUp
        } else if stop_time_elapsed(svc, now) {
            Self::ForceDown
        } else {
            Self::None
        }
    }

    fn from_cleaning_up(svc: &Service, now: timespec) -> Self {
        if !svc.has_pid() {
            Self::Down
        } else if clean_up_time_elapsed(svc, now) {
            Self::ForceDown
        } else {
            Self::None
        }
    }

    fn from_forced_down(svc: &Service, now: timespec) -> Self {
        if !svc.has_pid() {
            match svc.target {
                Target::Up | Target::Once => Self::FailedOrRetry,
                Target::Down | Target::Restart => Self::Down,
            }
        } else if forced_down_time_elapsed(svc, now) {
            Self::CannotStop
        } else {
            Self::None
        }
    }

    fn from_retrying(svc: &Service, now: timespec) -> Self {
        match svc.target {
            _ if svc.has_pid() => Self::ForceDown, // Stop unexpected process
            Target::Down | Target::Restart => Self::Down,
            // Skip Down and go straight to WaitingToStart to avoid resetting attempt counter
            Target::Up | Target::Once if retry_period_elapsed(svc, now) => Self::WaitingToStart,
            Target::Up | Target::Once => Self::None,
        }
    }

    fn from_failed(svc: &Service) -> Self {
        if svc.has_pid() {
            Self::ForceDown // Stop unexpected process
        } else {
            // From the point of view of this module, failed is a terminal state.
            //
            // The user can nudge the service out of the failed state by setting a target via
            // `conctl`, which will update not only the target but also move the state to Down.
            Self::None
        }
    }

    fn from_cannot_stop(svc: &Service) -> Self {
        if !svc.has_pid() {
            // We could reach this if:
            // - The process finally died on its own
            // - Some external, possibly higher privileged process killed the process
            //
            // If we entered CannotStop, something went wrong. We don't automatically transition to
            // the happy path Down, but instead treat this as a failure even if it eventually
            // unblocked.
            Self::FailedOrRetry
        } else {
            Self::None
        }
    }
}

fn apply_down(svc: &mut Service) {
    // Dependency target propagation (e.g. groups) only apply on user-requested target change.
    // Automatic target changes are handled individually for each service.
    // Thus, we're not propagating target change here, but only applying it directly on the
    // service.
    match svc.target {
        Target::Down | Target::Up => {}
        Target::Restart => svc.target = Target::Up,
        Target::Once => svc.target = Target::Down,
    }

    svc.state = State::Down;
    svc.attempt_count = 0;
    #[cfg(feature = "settle")]
    settle_notify(svc);
}

fn apply_waiting_to_start(svc: &mut Service) {
    svc.state = State::WaitingToStart;
    #[cfg(feature = "settle")]
    settle_clear(svc);
}

fn apply_setting_up(svc: &mut Service, logger_fd: Option<Fd>) {
    match svc.spawn_setting_up(logger_fd) {
        Ok(()) => {
            svc.state = State::SettingUp;
            #[cfg(feature = "settle")]
            settle_clear(svc);
        }
        Err(_) => apply_failed_or_retry(svc),
    }
}

fn apply_starting(svc: &mut Service, logger_fd: Option<Fd>) {
    match svc.spawn_run(logger_fd) {
        Ok(()) => {
            svc.state = State::Starting;
            #[cfg(feature = "settle")]
            settle_clear(svc);
        }
        Err(_) => apply_failed_or_retry(svc),
    }
}

fn apply_up(svc: &mut Service) {
    svc.state = State::Up;
    #[cfg(feature = "settle")]
    settle_notify(svc);
}

fn apply_waiting_to_stop(svc: &mut Service) {
    svc.state = State::WaitingToStop;
    #[cfg(feature = "settle")]
    settle_clear(svc);
}

fn apply_stopping(svc: &mut Service) {
    // Both SIGTERM success and failure lead to the same behavior:
    // - If process dies (on its own or because of signal) => Down
    // - If does not die => ForceDown
    //
    // Thus result doesn't matter and should be ignored.
    if let Some(pid) = svc.pid {
        let _ = kill(pid, Signal::SIGTERM);
    }
    svc.state = State::Stopping;
    #[cfg(feature = "settle")]
    settle_clear(svc);
}

fn apply_cleaning_up(svc: &mut Service, logger_fd: Option<Fd>) {
    match svc.spawn_cleaning_up(logger_fd) {
        Ok(()) => {
            svc.state = State::CleaningUp;
            #[cfg(feature = "settle")]
            settle_clear(svc);
        }
        Err(_) => apply_failed_or_retry(svc),
    }
}

fn apply_force_down(svc: &mut Service) {
    // The kill() return value doesn't matter.  In every scenario, either:
    // - The child dies and we continue as though the kill() was successful.
    // - The child doesn't die, we timeout, and transition to CannotStop.
    // It doesn't matter if it was because of ESRCH indicating the child died before we sent
    // SIGKILL, because of EPERM indicating the child didn't die because we lacked permissions,
    // etc.

    if let Some(pid) = svc.supervisor_pid {
        // tells supervisor to SIGKILL children until there are none left
        let _ = kill(pid, Signal::SIGTERM);
    }

    if let Some(pid) = svc.pid {
        let _ = kill(pid, Signal::SIGKILL);
    }

    svc.state = State::ForceDown;
    #[cfg(feature = "settle")]
    settle_clear(svc);
}

fn apply_failed_or_retry(svc: &mut Service) {
    svc.attempt_count = svc.attempt_count.saturating_add(1);

    let Some(max_attempt_count) = svc.cfg.max_attempt_count else {
        svc.state = State::Retrying;
        #[cfg(feature = "settle")]
        settle_clear(svc);
        return;
    };

    if svc.attempt_count < max_attempt_count {
        svc.state = State::Retrying;
        #[cfg(feature = "settle")]
        settle_clear(svc);
        return;
    }

    svc.state = State::Failed;
    match svc.target {
        Target::Down | Target::Up => {}
        Target::Restart => svc.target = Target::Up,
        Target::Once => svc.target = Target::Down,
    }
    #[cfg(feature = "settle")]
    settle_notify(svc);
}

fn apply_cannot_stop(svc: &mut Service) {
    svc.state = State::CannotStop;
    #[cfg(feature = "settle")]
    settle_notify(svc);
}

fn start_dep_satisfied<const N: usize>(svc: &Service, svcs: &[Service; N]) -> bool {
    needs_satisfied(svc, svcs) && wants_satisfied(svc, svcs) && conflicts_satisfied(svc, svcs)
}

fn needs_satisfied<const N: usize>(svc: &Service, svcs: &[Service; N]) -> bool {
    svc.cfg.needs.iter().all(|&i| {
        svcs.get(i)
            .map(|dep| matches!(dep.state, State::Up))
            .unwrap_or(true)
    })
}

fn wants_satisfied<const N: usize>(svc: &Service, svcs: &[Service; N]) -> bool {
    svc.cfg.wants.iter().all(|&i| {
        svcs.get(i)
            .map(|dep| matches!(dep.state, State::Up | State::Failed | State::CannotStop))
            .unwrap_or(true)
    })
}

fn conflicts_satisfied<const N: usize>(svc: &Service, svcs: &[Service; N]) -> bool {
    svc.cfg.conflicts.iter().all(|&i| {
        svcs.get(i)
            .map(|dep| matches!(dep.state, State::Down | State::Failed))
            .unwrap_or(true)
    })
}

fn stop_deps_satisfied<const N: usize>(svc: &Service, svcs: &[Service; N]) -> bool {
    svc.cfg.stop_dependencies.iter().all(|&dep_idx| {
        svcs.get(dep_idx)
            .map(|dep| {
                matches!(
                    dep.state,
                    State::Down | State::WaitingToStart | State::Failed | State::CannotStop
                )
            })
            .unwrap_or(true)
    })
}

fn setup_time_elapsed(svc: &Service, now: timespec) -> bool {
    svc.cfg
        .max_setup_time_millis
        .is_some_and(|max| now.millis_since(svc.time) >= max as i64)
}

fn start_time_elapsed(svc: &Service, now: timespec) -> bool {
    svc.cfg
        .max_ready_time_millis
        .is_some_and(|max| now.millis_since(svc.time) >= max as i64)
}

fn up_time_elapsed(svc: &Service, now: timespec) -> bool {
    now.millis_since(svc.time) >= UP_TIME_MILLIS
}

fn stop_time_elapsed(svc: &Service, now: timespec) -> bool {
    svc.cfg
        .max_stop_time_millis
        .is_some_and(|max| now.millis_since(svc.time) >= max as i64)
}

fn clean_up_time_elapsed(svc: &Service, now: timespec) -> bool {
    svc.cfg
        .max_cleanup_time_millis
        .is_some_and(|max| now.millis_since(svc.time) >= max as i64)
}

fn forced_down_time_elapsed(svc: &Service, now: timespec) -> bool {
    now.millis_since(svc.time) > FORCED_DOWN_TIME_MILLIS
}

fn retry_period_elapsed(svc: &Service, now: timespec) -> bool {
    now.millis_since(svc.time) >= svc.retry_delay_millis()
}

/// Write a byte to the settle pipe to notify waiters that service reached a stable state
#[cfg(feature = "settle")]
fn settle_notify(svc: &Service) {
    if let Some((_, ref write_fd)) = svc.settle_pipe {
        let _ = write_fd.write(&[1u8]);
    }
}

/// Drain all bytes from the settle pipe when leaving a stable state
#[cfg(feature = "settle")]
fn settle_clear(svc: &Service) {
    if let Some((ref read_fd, _)) = svc.settle_pipe {
        let mut buf = [0u8; PIPE_BUF];
        // This is guaranteed to fully drain the pipe:
        // - We're blocking signals, and so we can't be interrupted by a signal
        // - settle_pipe is created as non-blocking
        // - We *should* only see one byte written into the pipe before it is cleared such that a
        // single byte read should clear it. To be extra safe, we're reading the entire pipe
        // buffer. From `man 2 pipe`:
        //
        // > If a read(2) specifies a buffer size that is smaller than the next packet, then the
        // > requested number of bytes are read, and the excess bytes in the packet are discarded.
        // > Specifying a buffer size of PIPE_BUF will be sufficient to read the largest possible
        // > packets (see the previous point).
        let _ = read_fd.read(&mut buf);
    }
}
