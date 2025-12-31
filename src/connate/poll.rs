use connate::constants::*;
use connate::err::*;
use connate::internal_api::*;
use connate::ipc::*;
use connate::os::SignalFd;
use connate::syscall::{PollEvents, PollFd, poll};
use connate::types::*;

/// Connate-specific poll abstraction
///
/// Typically this code base prefers generic os abstractions in src/os.  However, poll() is
/// difficult abstract both generically and without allocation; thus, we just special-case it to
/// connate here.
pub struct Poll {
    fds: [PollFd; 2],
}

impl Poll {
    pub fn new(signalfd: &SignalFd, ipc_server: &IpcServer) -> Self {
        let events = PollEvents::POLLIN;
        let revents = PollEvents::empty();

        let fds = [
            PollFd {
                fd: signalfd.as_raw(),
                events,
                revents,
            },
            PollFd {
                fd: ipc_server.fd_req_read().as_raw(),
                events,
                revents,
            },
        ];

        Self { fds }
    }

    pub fn poll(&mut self, timeout_millis: Option<i32>) -> PollFdReady {
        let timeout = timeout_millis.unwrap_or(-1);

        let _ = unsafe { poll(&mut self.fds, timeout) }.or_abort("Unable to call poll()");

        if self.fds[0].revents.contains(PollEvents::POLLIN) {
            PollFdReady::SignalFd
        } else if self.fds[1].revents.contains(PollEvents::POLLIN) {
            PollFdReady::Request
        } else {
            PollFdReady::TimeoutExpired
        }
    }
}

pub enum PollFdReady {
    TimeoutExpired,
    SignalFd,
    Request,
}

/// Calculate remaining ms until timeout for a single service, or None if no timeout needed
fn service_timeout(svc: &Service, now: timespec) -> Option<i64> {
    let target_ms: i64 = match svc.state {
        State::SettingUp => svc.cfg.max_setup_time_millis? as i64,
        State::Starting => svc.cfg.max_ready_time_millis? as i64,
        State::Up if svc.attempt_count != 0 => UP_TIME_MILLIS,
        State::Stopping => svc.cfg.max_stop_time_millis? as i64,
        State::CleaningUp => svc.cfg.max_cleanup_time_millis? as i64,
        State::Retrying if matches!(svc.target, Target::Down | Target::Restart) => return None,
        State::Retrying => svc.retry_delay_millis(),
        // Other states don't automatically transition on timeout
        _ => return None,
    };

    let elapsed = now.millis_since(svc.time);
    Some(target_ms.saturating_sub(elapsed))
}

/// Calculate the poll timeout needed for all services
///
/// Returns a mutable reference to the service that will timeout next and an optional timeout value.
/// If the timeout is `None`, no timeout is needed (infinite wait).
/// If the timeout is `Some(0)`, a timeout has already expired.
/// If the timeout is `Some(ms)`, that's the minimum time until next timeout.
pub fn calculate_poll_timeout<const N: usize>(
    svcs: &mut [Service; N],
    now: timespec,
) -> (Option<i32>, Option<&mut Service>) {
    let mut min_time: Option<i32> = None;
    let mut min_svc: Option<&mut Service> = None;

    for svc in svcs.iter_mut() {
        let Some(remaining) = service_timeout(svc, now) else {
            continue;
        };

        let remaining = remaining.clamp(0, i32::MAX as i64) as i32;

        if min_time.is_none() || min_time.is_some_and(|m| remaining < m) {
            min_time = Some(remaining);
            min_svc = Some(svc);
        }
    }

    (min_time, min_svc)
}
