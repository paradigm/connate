//! Configuration API / documentation.

use crate::err::Errno;

/// To configure connate, implement `trait Config` on this `struct Connate` in
/// `src/config/config.rs`
pub struct Connate {}

/// Connate configuration API
///
/// To configure connate, implement this trait on `struct Connate` in `src/config/config.rs`
pub trait Config {
    /// Lock file path
    ///
    /// If None:
    /// - connate session is identified via PID
    /// - conctl defaults to PID=1, assuming init.
    /// - Recommended for init
    ///
    /// If Some:
    /// - connate session is identified via lock file or PID.
    /// - conctl defaults to specified path.
    /// - Recommended for user sessions.
    ///
    /// Examples:
    ///
    /// ```ignore
    /// const LOCK_FILE: Option<&'static str> = None;
    /// const LOCK_FILE: Option<&'static str> = Some("/etc/connate.lock");
    /// const LOCK_FILE: Option<&'static str> = Some("/home/user/.connate.lock");
    /// const LOCK_FILE: Option<&'static str> = Some("/run/1000/connate.lock");
    /// ```
    const LOCK_FILE: Option<&'static str>;

    /// Default fields that can be used to avoid verbosely populating every field in every service.
    ///
    /// Overwrite in config.rs as desired then include in a given Service definition to implement
    /// default field values.
    ///
    /// Examples:
    ///
    /// ```ignore
    /// Service {
    ///     name: "sshd",
    ///     needs: &["network"],
    ///     run: Run::Exec(&["/usr/sbin/sshd", "-D"]),
    ///     ..Self::DEFAULT_SERVICE // pull other fields from DEFAULT_SERVICE
    /// }
    ///
    /// // Virtual service; no process, only used for dependency effects
    /// Service {
    ///     name: "gui",
    ///     groups: &["xorg", "openbox", "dunst", "xcape"],
    ///     ..Self::DEFAULT_SERVICE // pull other fields from DEFAULT_SERVICE
    /// }
    /// ```
    ///
    const DEFAULT_SERVICE: Service = Service {
        name: "unspecified-service-name",
        init_target: Target::Up,
        // Dependency entries
        needs: &[],
        wants: &[],
        conflicts: &[],
        groups: &[],
        // Execution entries
        setup: Run::None,
        run: Run::None,
        ready: Ready::Immediately,
        cleanup: Run::None,
        stop_all_children: false,
        // Retry and timeout entries
        max_setup_time: Some(core::time::Duration::from_secs(10)),
        max_ready_time: Some(core::time::Duration::from_secs(10)),
        max_stop_time: Some(core::time::Duration::from_secs(2)),
        max_cleanup_time: Some(core::time::Duration::from_secs(10)),
        retry: Retry::Never,
        // Execution attribute entries
        log: Log::Inherit,
        env: &[],
        user: None,
        group: None,
        chdir: None,
        no_new_privs: false,
    };

    /// The list of services to run
    const SERVICES: &'static [Service];
}

/// A service definition
pub struct Service {
    /// The service's name
    pub name: &'static str,
    /// The service's target state when connate first learns of the service.  This is applied:
    /// - When connate first starts
    /// - When connate re-execs itself and sees a new service
    pub init_target: Target,
    //
    // Dependency entries
    //
    /// Dependencies which must be up before this service can start
    pub needs: &'static [&'static str],
    /// Dependencies which must be up or failed (i.e. attempted to go up) before this service can
    /// start
    pub wants: &'static [&'static str],
    /// Services which must be down or failed for this service to start.
    pub conflicts: &'static [&'static str],
    /// Services which inherit this service's target state when it changes.
    /// Useful to start/stop related services in one go.
    pub groups: &'static [&'static str],
    //
    // Execution entries
    //
    /// How to setup the service
    ///
    /// - Usually a short-lived process
    /// - Example use cases:
    ///   - Creating directories
    ///   - Loading kernel modules
    ///   - Initialization phase of a process
    pub setup: Run,
    /// How to run the service
    ///
    /// - Runs after `.setup`
    /// - This defines what the service does while fulfilling dependencies.
    /// - Usually a long-lived process
    /// - Example use cases:
    ///   - Services such as sshd or cupsd
    pub run: Run,
    /// Indicates when the service's `.run` is ready to fulfill dependencies
    pub ready: Ready,
    /// How to clean up after the service
    ///
    /// - Runs after `.setup` and `.run`
    /// - Usually a short-lived process
    /// - Example use cases:
    ///   - Save state to disk
    ///   - Remove temporary files
    pub cleanup: Run,
    /// Indicates whether to stop only the "main" process or all processes spawned by the service
    ///
    /// If true, stops all processes spawned by the service.
    /// If false, only stops "main" process; allows "non-main" process to continue untracked.
    ///
    /// This adds a small amount of overhead for a supervisor process.
    pub stop_all_children: bool,
    //
    // Retry and timeout entries
    //
    /// The maximum amount of time a service's `.setup` may run before it is assumed to be hanging
    /// and forcibly killed.
    pub max_setup_time: Option<core::time::Duration>,
    /// The maximum amount of time a service's `.run` may run before indicating `.ready` before it
    /// is assumed to be hanging and forcibly killed.
    pub max_ready_time: Option<core::time::Duration>,
    /// The maximum amount of time a service's `.run` may run after `.stop` tells it to stop before
    /// it is assumed to be hanging and forcibly killed.
    pub max_stop_time: Option<core::time::Duration>,
    /// The maximum amount of time a service's `.cleanup` may run before it is assumed to be hanging
    /// and forcibly killed.
    pub max_cleanup_time: Option<core::time::Duration>,
    /// The retry strategy should a Service fail
    pub retry: Retry,
    //
    // Execution attribute entries
    //
    /// How to handle this service's stdout and stderr
    pub log: Log,
    /// The environment variables to set for the service's execution Run::Exec and Run::Shell
    /// entries.  Is ignored by Run::Fn() entries.
    ///
    /// Populate as a list of VAR=VALUE, e.g.
    /// ```ignore
    /// env: &[
    ///     "PATH=/usr/bin:/bin",
    ///     "DISPLAY=:0"
    /// ],
    /// ```
    pub env: &'static [&'static str],
    /// Run the service processes as the given user.  If None, retains connate daemon user.
    ///
    /// Requires root.  Intended to be used by an init / system-wide service manager to drop
    /// permissions for a given service.
    pub user: Option<&'static str>,
    /// Run the service processes as the given group.  If None, retains connate daemon group.
    ///
    /// Requires root.  Intended to be used by an init / system-wide service manager to drop
    /// permissions for a given service.
    pub group: Option<&'static str>,
    /// Set the service's working directory. If None, retains connate daemon's working directory.
    pub chdir: Option<&'static str>,
    /// Prevent the service and its children from gaining new privileges.
    pub no_new_privs: bool,
}

pub enum Target {
    /// The service's target state is Down.
    Down,
    /// The service's target state is Up.
    Up,
    /// The service's immediate target state is Down.  Once Down or Failed, its target changes to
    /// Up.
    Restart,
    /// The service's immediate target state is Up.  Once Down or Failed, its target changes to
    /// Down.
    Once,
}

/// How to run a given `.setup`, `.run`, or `.cleanup` phase
pub enum Run {
    /// Skip doing anything in this phase.
    None,
    /// Execute the given full filepath (no $PATH searching) with arguments.
    ///
    /// Example:
    /// Exec(&["/usr/bin/mkdir", "-p", "/var/log"]),
    /// Exec(&["/usr/sbin/sshd"]), // Daemonizes, pair with Ready::Daemonize
    Exec(&'static [&'static str]),
    /// Run command in a shell
    ///
    /// Effectively `/bin/sh -c <command>`
    Shell(&'static str),
    /// Run the given function
    Fn(fn() -> Result<(), Errno>),
}

/// When `.run` (if any) is ready to fulfill dependencies and should be considered "Up"
pub enum Ready {
    /// Ready immediately after `.setup` phase irrelevant of `.run` value.
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

/// How to stop `.run`
pub enum Stop {
    /// Send signal
    ///
    /// Most services probably want `.stop: Stop::Signal(Signal::SIGTERM),`
    Signal(crate::types::Signal),
    /// Execute command
    ///
    /// Example:
    /// Exec(&["/usr/lib/postresql/17/bin/pg_ctl", "stop"]),
    Exec(&'static [&'static str]),
    /// Run command in a shell
    ///
    /// Example:
    /// Shell("SSH_AGENT_PID=$(conctl pid ssh-agent) exec /usr/bin/ssh-agent -k"),
    Shell(&'static str),
    /// Run the given function
    Fn(fn() -> Result<(), Errno>),
}

/// Retry strategy
pub enum Retry {
    Never,
    AfterFixed {
        /// Retry after a fixed duration
        after: core::time::Duration,
        /// Maximum number of times to attempt to start and remain up.
        ///
        /// None indicates no limit.
        max_attempt_count: Option<u32>,
    },
    AfterDoublingDelay {
        /// First retry is after this delay
        ///
        /// Following retries double the delay time each attempt
        initial_delay: core::time::Duration,
        /// Maximum number of times to attempt to start and remain up.
        ///
        /// None indicates no limit.
        max_attempt_count: Option<u32>,
    },
}

/// Logging configuration for a service
///
/// Determines where the service's stdout and stderr output should be sent.
pub enum Log {
    /// No logging. Stdout and stderr are redirected to /dev/null
    None,
    /// Inherit connate's stdout and stderr
    ///
    /// Useful for early boot services (e.g. fsck) that need to send messages to the console
    /// before logging infrastructure is available.
    Inherit,
    /// Log to a given file path
    File {
        /// stdout and stderr are redirected to this path
        ///
        /// File is created if it does not already exist
        path: &'static str,
        /// Whether to append to or overwrite a preexisting file
        mode: FileMode,
        /// Whether to make a new file public (world readable) or private (only readable by owner)
        permissions: FilePerm,
    },
    /// Log to another service's stdin via a pipe
    ///
    /// - This service's stdout and stderr will be redirected to the specified service's stdin
    /// - The logger service will be implicitly added to the needs list (must start before this service)
    /// - The logger service will be implicitly added to the groups list (stops with this service)
    Service(&'static str),
}

/// How to handle logging to a file path that already has a file
pub enum FileMode {
    /// Append to the end of the existing file
    Append,
    /// Overwrite the file
    Overwrite,
}

/// How to handle permissions when logging to a new file
pub enum FilePerm {
    /// File is readable by everyone (`0o644`)
    Public,
    /// File is only readable by owner (`0o600`)
    Private,
}

/// The states a service can be in.
/// Intended as documentation for user to understand state flow.
#[allow(unused)]
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
    Down,
    /// The service is effectively Down but intends to transition to SettingUp once dependencies are
    /// fulfilled.
    WaitingToStart,
    /// The service is running `.setup`
    SettingUp,
    /// The service is running `.run` but hasn't yet triggered `.ready`
    Starting,
    /// The service is Up and fulfilling dependencies.
    Up,
    /// The service is effectively Up but intends to transition toward Stopping once dependents are
    /// Down or Failed.
    WaitingToStop,
    /// The service's `.run` process was told to `.stop` but has not yet exited.
    Stopping,
    /// The service is running `.cleanup`
    CleaningUp,
    // ================
    // Failure handling
    // ================
    /// The service has failed and is waiting for `retry_period` to attempt to go Up again.
    Retrying,
    /// The service has failed and exhausted `max_attempt_count` attempts.  Manual intervention is
    /// required.
    Failed,
    /// The service did not stop when expected to.  We're forcing it down via SIGKILL.
    /// Waiting for either process to exit or timeout indicating we can't kill it.
    ForceDown,
    /// Connate cannot stop the service.  This may occur if connate itself is unprivileged but has
    /// a setuid service that can ignore connate's sigkill signals.
    ///
    /// If this occurred, something went wrong.  Avoid returning to the happy path.  Manual
    /// intervention is required.
    CannotStop,
}
