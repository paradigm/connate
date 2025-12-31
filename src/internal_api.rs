//! Internal, non-user-facing service definition information
//!
//! This has pre-processed versions of user-defined config values such as:
//!
//! - Flattened dependency lists
//! - Complex time types converted to milliseconds
//! - System call oriented pointers

use crate::err::*;
use crate::ipc::*;
use crate::os::*;
use crate::types::*;

pub struct Service {
    /// Service's current state
    pub state: State,
    /// Service's current target state
    pub target: Target,
    /// Service's current pid
    pub pid: Option<pid_t>,
    /// Supervisor PID
    pub supervisor_pid: Option<pid_t>,
    /// Logger stdin
    pub stdin_pipe: Option<(Fd, Fd)>,
    /// Number of times service has tried to start
    pub attempt_count: u32,
    /// Return value of last "main" process
    pub exit_code: Option<c_int>,
    /// Time service entered current state
    /// Delta from current time provides time spent in state
    pub time: timespec,
    /// Readiness flag: true when external process informs us service is ready to transition to Up
    /// - Supervisor will set ready when daemonizing process daemonizes
    /// - Run::Notify will send Request::Ready signal (re-using same pid)
    pub ready: bool,
    /// The service needs to be checked for a potential state change
    pub dirty: bool,
    /// Settle pipe for conctl to wait for stable states
    /// Created lazily on first settle request
    #[cfg(feature = "settle")]
    pub settle_pipe: Option<(Fd, Fd)>,
    /// Read-only, preprocessed user-made service configuration
    pub cfg: &'static ServiceConfig,
}

pub struct ServiceConfig {
    pub name: &'static [u8],
    pub index: usize,
    pub init_target: Target,
    //
    // Dependency entries
    //
    pub needs: &'static [usize],
    pub wants: &'static [usize],
    pub conflicts: &'static [usize],
    pub stop_dependencies: &'static [usize],
    pub groups: &'static [usize],
    /// Services which should have their target set upward when this service's target is set to
    /// Up or Once, either directly or via the latter half of a Restart.
    pub target_up_propagate_up: &'static [usize],
    /// Services which should have their target set downward when this service's target is changed
    /// to Up or Once, either directly or via the latter half of a Restart.
    pub target_up_propagate_down: &'static [usize],
    /// Services which should have their target set downward when this service's target is changed
    /// to Down or Restart.
    pub target_down_propagate_down: &'static [usize],
    /// Services whose state should be revisited when this service's state changes.
    pub propagate_dirty: &'static [usize],
    //
    // Execution entries
    //
    pub setup: Run,
    pub run: Run,
    pub ready: Ready,
    pub cleanup: Run,
    pub stop_all_children: bool,
    //
    // Retry and timeout entries
    //
    /// Using c_int millis because that's the poll() system call expectation.
    pub max_setup_time_millis: Option<c_int>,
    pub max_ready_time_millis: Option<c_int>,
    pub max_stop_time_millis: Option<c_int>,
    pub max_cleanup_time_millis: Option<c_int>,
    pub retry_wait_period_millis: c_int,
    pub retry_wait_multiplier: c_int, // either 1 or 2
    pub max_attempt_count: Option<u32>,
    //
    // Execution attribute entries
    //
    pub log: Log,
    pub is_logger: bool,
    pub uid: Option<uid_t>,
    pub gid: Option<gid_t>,
    pub no_new_privs: bool,
    pub chdir: Option<&'static CStr>,
}

#[repr(u8)]
#[derive(Copy, Clone)]
pub enum State {
    // ==========
    // Happy path
    // ==========
    //
    // Upward flow:
    // Down -> WaitingToStart -> SettingUp -> Starting -> Up
    //
    // Downward flow:
    // Up -> WaitingToStop -> Stopping -> CleaningUp -> Down
    /// The service is Down and intends to stay Down.
    Down = b'd',
    /// The service is effectively Down but intends to transition to SettingUp once dependencies are
    /// fulfilled.
    WaitingToStart = b'w',
    /// The service is running `.setup`
    SettingUp = b's',
    /// The service is running `.run` but hasn't yet triggered `.ready`
    Starting = b'S',
    /// The service is Up and fulfilling dependencies.
    Up = b'u',
    /// The service is effectively Up but intends to transition toward Stopping once dependents are
    /// Down or Failed.
    WaitingToStop = b'W',
    /// The service's `.run` process was sent SIGTERM but has not yet exited.
    Stopping = b'g',
    /// The service is running `.cleanup`
    CleaningUp = b'c',
    // ================
    // Failure handling
    // ================
    /// The service has failed and is waiting for `retry_period` to attempt to go Up again.
    Retrying = b'r',
    /// The service has failed and exhausted `max_attempt_count` attempts.  Manual intervention is
    /// required.
    Failed = b'f',
    /// The service did not stop when expected to.  We're forcing it down via SIGKILL.
    /// Waiting for either process to exit or timeout indicating we can't kill it.
    ForceDown = b'F',
    /// Connate cannot stop the service.  This may occur if connate itself is unprivileged but has
    /// a setuid service that can ignore connate's sigkill signals.
    ///
    /// If this occurred, something went wrong.  Avoid returning to the happy path.  Manual
    /// intervention is required.
    CannotStop = b'C',
}

/// The service's target state
#[derive(Copy, Clone)]
#[repr(u8)]
pub enum Target {
    /// The service's target state is down.
    Down = b'd',
    /// The service's target state is up.
    Up = b'u',
    /// The service's immediate target state is down.  Once down or failed, its target state
    /// changes to up.
    Restart = b'r',
    /// The service's immediate target state is up.  Once down or failed, its target state
    /// its target state changes to down.
    Once = b'o',
}

/// How to run a given `.setup`, `.run`, or `.cleanup` phase
pub enum Run {
    /// Skip doing anything in this phase.
    None,
    /// Execute the given full filepath (no $PATH searching) with arguments.
    Exec {
        pathname: &'static CStr,
        argv: *const *const c_char,
        envp: *const *const c_char,
        log_overwrite: bool,
    },
    /// Run the given function
    Fn {
        f: fn() -> Result<(), Errno>,
        log_overwrite: bool,
    },
}

/// When `.run` (if any) is ready to fulfill dependencies and should be considered "Up"
pub enum Ready {
    /// Ready immediately after `.setup` irrelevant of `.run` value.
    ///
    /// If `Ready::Notify` is available, it should be preferred. Otherwise, this is useful for
    /// services that lack a mechanism to indicate readiness.
    ///
    /// Required if `run: Run::None` (as Notify and Daemonize are impossible).
    Immediately,
    /// Ready as soon as `.run` runs `conctl ready` or the `notify_ready()` helper function.
    Notify,
    /// Ready as soon as `.run` daemonizes, i.e. forks off a new process and has the main process
    /// exit successfully (exit code 0).
    ///
    /// This adds a small amount of additional overhead for a supervisor process.  If the
    /// process support a non-daemonizing mode, this is usually preferred.
    Daemonize,
}

impl Service {
    pub fn has_pid(&self) -> bool {
        self.pid.is_some() || self.supervisor_pid.is_some()
    }

    pub fn logger_fd<const N: usize>(&self, svcs: &[Service; N]) -> Option<Fd> {
        let Log::Service(i) = self.cfg.log else {
            return None;
        };

        let log_svc = svcs.get(i)?;

        let Some((_, write_fd)) = &log_svc.stdin_pipe else {
            return None;
        };

        Some(write_fd.clone())
    }

    /// Calculate retry delay in milliseconds for current attempt
    pub fn retry_delay_millis(&self) -> i64 {
        (self.cfg.retry_wait_period_millis as i64).saturating_mul(
            self.cfg
                .retry_wait_multiplier
                .saturating_pow(self.attempt_count.saturating_sub(1)) as i64,
        )
    }
}

pub trait ServiceArray {
    fn all_down_or_err(&self) -> bool;
    fn any_bad(&self) -> bool;
    fn find_dirty_index(&self) -> Option<usize>;
    fn find_by_pid_mut(&mut self, pid: pid_t) -> Option<&mut Service>;
    fn find_by_supervisor_pid_mut(&mut self, pid: pid_t) -> Option<&mut Service>;
    fn find_by_direct_or_supervisor_pid_mut(&mut self, pid: pid_t) -> Option<&mut Service>;
    // Available via ServiceArrayExt trait in src/internal.rs :
    // fn find_by_name(&self, name: &[u8]) -> Option<&Service>;
    // fn find_by_name_mut(&mut self, name: &[u8]) -> Option<&mut Service>;
}

impl<const N: usize> ServiceArray for &mut [Service; N] {
    fn all_down_or_err(&self) -> bool {
        self.iter()
            .all(|svc| matches!(svc.state, State::Down | State::Failed | State::CannotStop))
    }

    fn any_bad(&self) -> bool {
        self.iter()
            .any(|svc| matches!(svc.state, State::Failed | State::CannotStop))
    }

    fn find_dirty_index(&self) -> Option<usize> {
        self.iter()
            .enumerate()
            .find_map(|(i, svc)| svc.dirty.then_some(i))
    }

    fn find_by_pid_mut(&mut self, pid: pid_t) -> Option<&mut Service> {
        self.iter_mut().find(|svc| svc.pid == Some(pid))
    }

    fn find_by_supervisor_pid_mut(&mut self, pid: pid_t) -> Option<&mut Service> {
        self.iter_mut().find(|svc| svc.supervisor_pid == Some(pid))
    }

    fn find_by_direct_or_supervisor_pid_mut(&mut self, pid: pid_t) -> Option<&mut Service> {
        self.iter_mut()
            .find(|svc| svc.pid == Some(pid) || svc.supervisor_pid == Some(pid))
    }
}

impl State {
    pub fn as_byte(&self) -> u8 {
        *self as u8
    }

    pub fn from_byte(byte: u8) -> Result<Self, Errno> {
        match byte {
            b'd' => Ok(State::Down),
            b'w' => Ok(State::WaitingToStart),
            b's' => Ok(State::SettingUp),
            b'S' => Ok(State::Starting),
            b'u' => Ok(State::Up),
            b'W' => Ok(State::WaitingToStop),
            b'g' => Ok(State::Stopping),
            b'c' => Ok(State::CleaningUp),
            b'r' => Ok(State::Retrying),
            b'f' => Ok(State::Failed),
            b'F' => Ok(State::ForceDown),
            b'C' => Ok(State::CannotStop),
            _ => Err(Errno::EINVAL),
        }
    }

    /// Check if a state is stable (won't transition automatically)
    pub fn stable(&self) -> bool {
        matches!(
            self,
            State::Down | State::Up | State::Failed | State::CannotStop
        )
    }

    /// Check if a state is a "bad" stable state (Failed or CannotStop)
    pub fn bad(&self) -> bool {
        matches!(self, State::Failed | State::CannotStop)
    }
}

impl Print for State {
    fn print(&self, _fd: Fd) {
        use Color::*;
        match *self {
            State::Down => print_color(Dim, "down"),
            State::WaitingToStart => print_color(Transition, "waiting-to-start"),
            State::SettingUp => print_color(Transition, "setting-up"),
            State::Starting => print_color(Transition, "starting"),
            State::Up => print_color(Okay, "up"),
            State::WaitingToStop => print_color(Transition, "waiting-to-stop"),
            State::Stopping => print_color(Transition, "stopping"),
            State::CleaningUp => print_color(Transition, "cleaning-up"),
            State::Retrying => print_color(Transition, "retrying"),
            State::Failed => print_color(Error, "failed"),
            State::ForceDown => print_color(Error, "force-down"),
            State::CannotStop => print_color(Error, "cannot-stop"),
        }
    }

    fn print_len(&self) -> usize {
        match *self {
            State::Down => "down".len(),
            State::WaitingToStart => "waiting-to-start".len(),
            State::SettingUp => "setting-up".len(),
            State::Starting => "starting".len(),
            State::Up => "up".len(),
            State::WaitingToStop => "waiting-to-stop".len(),
            State::Stopping => "stopping".len(),
            State::CleaningUp => "cleaning-up".len(),
            State::Retrying => "retrying".len(),
            State::Failed => "failed".len(),
            State::ForceDown => "force-down".len(),
            State::CannotStop => "cannot-stop".len(),
        }
    }
}

impl<'a> Target {
    pub fn as_byte(&'a self) -> u8 {
        *self as u8
    }

    pub fn from_byte(byte: u8) -> Result<Self, Errno> {
        match byte {
            b'd' => Ok(Target::Down),
            b'u' => Ok(Target::Up),
            b'r' => Ok(Target::Restart),
            b'o' => Ok(Target::Once),
            _ => Err(Errno::EINVAL),
        }
    }
}

impl Print for Target {
    fn print(&self, _fd: Fd) {
        use Color::*;
        use {print, print_color};
        match *self {
            Target::Up => print("up"),
            Target::Down => print_color(Dim, "down"),
            Target::Once => print("once"),
            Target::Restart => print_color(Transition, "restart"),
        }
    }

    fn print_len(&self) -> usize {
        match *self {
            Target::Up => "up".len(),
            Target::Down => "down".len(),
            Target::Once => "once".len(),
            Target::Restart => "restart".len(),
        }
    }
}

/// Internal logging configuration
///
/// This is the build-time processed version of the user-facing `crate::service::Log` enum.
pub enum Log {
    /// No logging - stdout and stderr are redirected to /dev/null
    None,
    /// Inherit connate's stdout and stderr
    Inherit,
    /// Log to a given file path
    File {
        /// stdout and stderr are redirected to this path
        ///
        /// File is created if it does not already exist
        filepath: &'static CStr,
        /// if creating new file, specifies permissions
        mode: c_int,
    },
    /// Log to another service's stdin via a pipe
    ///
    /// The index refers to the service in the services array
    Service(usize),
}

impl Log {
    pub fn as_response<'a, const N: usize>(&'a self, svcs: &[Service; N]) -> Response<'a> {
        match self {
            Log::None => Response::FieldIsNone,
            Log::Inherit => Response::Name(b"inherit"),
            Log::File { filepath, .. } => Response::Path(filepath.to_bytes()),
            Log::Service(i) => svcs
                .get(*i)
                .map(|svc| Response::Name(svc.cfg.name))
                .unwrap_or(Response::FieldIsNone),
        }
    }
}
