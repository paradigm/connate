//! Helper functions for config files
//!
//! These functions provide higher-level abstractions for common patterns in config files.

use crate::config::Target;
use crate::err::Errno;
use crate::ipc::{IpcClient, Request, Response};
use crate::os::*;
use crate::types::{CStr, mode_t};
use core::ffi::c_int;

/// Wrap operation(s) in a step which prints their start and resulting success or failure
///
/// Example:
///   step("Create /tmp/example", || mkdir(c"/tmp/example", 0o755) );
pub fn step(msg: &str, f: fn() -> Result<(), Errno>) -> Result<(), Errno> {
    print_color(Color::Dim, "[ ] ");
    println(msg);
    match f() {
        Ok(()) => {
            print_color(Color::Okay, "[X] ");
            println(msg);
            Ok(())
        }
        Err(e) => {
            print_color(Color::Error, "[!] ");
            print(msg);
            if let Some(e) = e.description() {
                print_color(Color::Error, " FAILED: ");
                println(e);
            } else {
                print_color(Color::Error, " FAILED!");
                println("");
            }
            Err(e)
        }
    }
}

/// Convert user friendly `[&str; N]` to `[&CStr.to_ptr(); N+1]` with trailing null as needed by
/// execve().
#[macro_export]
macro_rules! cargv {
    ([$($argv:literal),* $(,)?]) => {{
        unsafe {
            [
                $(
                    CStr::from_bytes_with_nul_unchecked(concat!($argv, "\0").as_bytes()).as_ptr(),
                )*
                core::ptr::null(),
            ]
        }
    }};
}

/// Replace the current process with a new binary.
///
/// argv[0] must be an absolute path to binary (no $PATH searching).
/// Following argv elements are parameters for binary
///
/// Example:
/// exec!(["/bin/ls", "-l"])?;
#[macro_export]
macro_rules! exec {
    ([$first:literal $(, $rest:literal)* $(,)?]) => {{
        // If host-checks enabled, check if path exists on build system
        #[cfg(feature = "host-checks")]
        const _: &[u8] = include_bytes!($first);

        let pathname = unsafe { CStr::from_bytes_with_nul_unchecked(concat!($first, "\0").as_bytes()) };
        let argv = $crate::cargv!([$first $(, $rest)*]);
        unsafe { crate::syscall::execve(pathname, argv.as_ptr(), core::ptr::null()).map(|_|()) }
    }};
}

/// Execute an external command and wait for it to complete.
///
/// argv[0] must be an absolute path to binary (no $PATH searching).
/// Following argv elements are parameters for binary
///
/// Example:
/// run!(&["/bin/ls", "-l")?;
#[macro_export]
macro_rules! run {
    ([$first:literal $(, $rest:literal)* $(,)?]) => {{
        match fork()? {
            ForkResult::Child => {
                let _ = $crate::exec!([$first $(, $rest)*]);
                exit(127);
            }
            ForkResult::Parent(pid) => {
                let (_, status) = waitpid(pid, WaitPidOptions::empty())?;
                if wifexited(status) {
                    let code = wexitstatus(status);
                    if code == 0 {
                        Ok(())
                    } else {
                        Err(Errno::new(code))
                    }
                } else if wifsignaled(status) {
                    Err(Errno::new(128 + wtermsig(status)))
                } else {
                    Err(Errno::new(status))
                }
            }
        }
    }};
}

/// Look up a service's target
///
/// This only works when called from a non-daemon service, as daemons are tracked with a supervisor
/// process and this naively assumes the parent process is connate.
///
/// # Example
/// ```ignore
/// use config::helper::get_service_target;
///
/// match (get_service_target("xorg"), get_service_target("Wayland")) {
///     (Some(Target::Up), _) => {
///         todo!("start X variant");
///     }
///     (_, Some(Target::Up)) => {
///         todo!("start Wayland variant");
///     }
/// }
/// ```
pub fn get_service_target(name: &str) -> Option<Target> {
    let connate_pid = getppid();
    let mut ipc_client = IpcClient::from_pid(connate_pid);
    let request = Request::QueryByNameTarget(name.as_bytes());
    ipc_client.lock_quiet();
    let target = match ipc_client.send_and_receive(request) {
        Response::Target(crate::internal_api::Target::Up) => Some(Target::Up),
        Response::Target(crate::internal_api::Target::Down) => Some(Target::Down),
        Response::Target(crate::internal_api::Target::Restart) => Some(Target::Restart),
        Response::Target(crate::internal_api::Target::Once) => Some(Target::Once),
        _ => None,
    };
    ipc_client.unlock();
    target
}

/// Check if a file exists at the given path.
pub fn file_exists(path: &CStr) -> bool {
    Fd::open(path, OpenFlags::O_RDONLY, 0)
        .map(|fd| {
            let _ = fd.close();
        })
        .is_ok()
}

/// Create a directory with the given mode, succeeding if it already exists.
///
/// Common modes:
/// - 0o755 (rwxr-xr-x): Standard directory, owner can write
/// - 0o700 (rwx------): Private directory, owner only
/// - 0o1777 (sticky + rwxrwxrwx): Shared temp directory (e.g., /tmp)
pub fn mkdir_mode(path: &CStr, mode: mode_t) -> Result<(), Errno> {
    match mkdir(path, mode) {
        Ok(()) | Err(Errno::EEXIST) => Ok(()),
        Err(e) => Err(e),
    }
}

/// Create or touch a file with the given mode.
///
/// Common modes:
/// - 0o644 (rw-r--r--): Standard file, owner can write
/// - 0o600 (rw-------): Private file, owner only
/// - 0o664 (rw-rw-r--): Group-writable file
pub fn touch_file(path: &CStr, mode: c_int) -> Result<(), Errno> {
    let fd = Fd::open(path, OpenFlags::O_WRONLY | OpenFlags::O_CREAT, mode)?;
    fd.close()
}

const MAX_ARGS: usize = 16;

/// Replace the current process with a new program.
///
/// argv[0] must be an absolute path to binary (no $PATH searching).
/// Following argv elements are parameters for binary
/// Only returns on error.
pub fn exec(argv: &[&CStr]) -> Result<(), Errno> {
    let mut ptrs = [core::ptr::null::<core::ffi::c_char>(); MAX_ARGS];
    for (i, arg) in argv.iter().enumerate() {
        if let Some(slot) = ptrs.get_mut(i) {
            *slot = arg.as_ptr();
        }
    }
    // SAFETY: argv is properly null-terminated array of null-terminated C strings
    unsafe { crate::syscall::execve(argv[0], ptrs.as_ptr(), core::ptr::null()).map(|_| ()) }
}

/// Read file contents into a buffer, returning bytes read.
pub fn read_file(path: &CStr, buf: &mut [u8]) -> Result<usize, Errno> {
    let fd = Fd::open(path, OpenFlags::O_RDONLY, 0)?;
    let n = match fd.read(buf) {
        Ok(n) => n,
        Err(e) => {
            let _ = fd.close();
            return Err(e);
        }
    };
    let _ = fd.close();
    Ok(n)
}

/// Write content to an existing file.
pub fn write_file(path: &CStr, content: &[u8]) -> Result<(), Errno> {
    let fd = Fd::open(path, OpenFlags::O_WRONLY, 0)?;
    let _ = fd.write(content)?;
    fd.close()
}

pub fn copy(input: &CStr, output: &CStr) -> Result<(), Errno> {
    let mut buf = [0u8; crate::constants::PIPE_BUF];
    let input = Fd::open(input, OpenFlags::O_RDONLY, 0)?;
    let output = Fd::open(output, OpenFlags::O_WRONLY, 0)?;

    loop {
        let n = input.read(&mut buf)?;
        if n == 0 {
            return Ok(());
        }
        if let Some(buf) = buf.get(0..n) {
            output.write(&buf)?;
        }
    }
}
