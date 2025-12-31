//! Session state management
//!
//! This module provides functionality to serialize and deserialize connate's
//! runtime state across exec() calls, enabling seamless re-exec for configuration
//! updates without losing service state.

use crate::internal::*;
use connate::constants::*;
use connate::err::*;
use connate::internal_api::*;
use connate::ipc::*;
use connate::os::*;
use connate::types::*;
use connate::util::*;

pub struct SessionFd(Fd);

/// Maximum serialized size of a single Service
///
/// Format for each service:
/// - ServiceStart header + name length (u16) + name bytes
/// - State header (enum variant)
/// - Target header (enum variant)
/// - Optional fields (header + value, only if Some)
/// - Integer fields (header + value, only if non-zero)
/// - Boolean flags (header only, only if true)
/// - ServiceEnd header
pub const SESSION_SERVICE_SIZE: usize = 1 // ServiceStart header
    + size_of::<u16>() // name length
    + MSG_SVC_NAME_SIZE // max name bytes
    + 1 // state header (enum variant)
    + 1 // target header (enum variant)
    + 1 + size_of::<i32>() // pid: header + value
    + 1 + size_of::<i32>() // supervisor_pid: header + value
    + 1 + size_of::<i32>() * 2 // stdin_pipe: header + 2 fds
    + 1 + size_of::<i32>() // exit_code: header + value
    + 1 + size_of::<u32>() // attempt_count: header + value
    + 1 + size_of::<i64>() // time_sec: header + value
    + 1 + size_of::<i64>() // time_nsec: header + value
    + 1 + size_of::<i64>() // sigkill_sec: header + value
    + 1 + size_of::<i64>() // sigkill_nsec: header + value
    + 1 // ready: header only
    + 1 + size_of::<i32>() * 2 // settle_pipe: header + 2 fds
    + 1; // ServiceEnd header

macro_rules! session_field_defs {
    ($(
        // Variant = byte
        $variant:ident = $byte:expr ,
    )*) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        #[repr(u8)]
        pub enum SessionField {
            $(
                $variant = $byte,
            )*
        }

        impl SessionField {
            pub const fn as_byte(self) -> u8 {
                self as u8
            }
        }

        impl core::convert::TryFrom<u8> for SessionField {
            type Error = ();

            fn try_from(value: u8) -> Result<Self, Self::Error> {
                match value {
                    $(
                        $byte => Ok(SessionField::$variant),
                    )*
                    _ => Err(())
                }
            }
        }
    };
}

session_field_defs! {
    // Service boundary markers
    ServiceStart = b'[',
    ServiceEnd = b']',

    // State enum variants
    StateDown = b'd',
    StateWaitingToStart = b'w',
    StateSettingUp = b's',
    StateStarting = b'S',
    StateUp = b'u',
    StateWaitingToStop = b'W',
    StateStopping = b'g',
    StateCleaningUp = b'c',
    StateRetrying = b'r',
    StateFailed = b'f',
    StateForceDown = b'F',
    StateCannotStop = b'C',

    // Target enum variants
    TargetDown = b'D',
    TargetUp = b'U',
    TargetRestart = b'R',
    TargetOnce = b'O',

    // Optional fields (presence = Some, absence = None)
    Pid = b'p',
    SupervisorPid = b'P',
    StdinPipe = b'i',
    ReturnValue = b'v',
    SettlePipe = b'q',

    // Integer fields (zipped indicates value is zero)
    AttemptCount = b'a',
    TimeSec = b't',
    TimeNsec = b'n',

    // Boolean flags (presence = true, absence = false)
    Ready = b'y',
    // Dirty flag is not meaningful across exec when configured service relations may have changed
    // Cost to re-check a dirty service once is low.
    // Thus, SERVICES.initialize() initializes `dirty = true`.
}

macro_rules! read_u16 {
    ( $session:ident, $buf:ident ) => {{
        let n = $session.0.read($buf.get_mut(..2).ok_or(Errno::EINVAL)?)?;
        if n != 2 {
            return Err(Errno::EINVAL);
        }
        u16::from_le_bytes([
            *$buf.first().ok_or(Errno::EINVAL)?,
            *$buf.get(1).ok_or(Errno::EINVAL)?,
        ])
    }};
}

macro_rules! read_i32 {
    ( $session:ident, $buf:ident ) => {{
        let n = $session.0.read($buf.get_mut(..4).ok_or(Errno::EINVAL)?)?;
        if n != 4 {
            return Err(Errno::EINVAL);
        }
        i32::from_le_bytes([
            *$buf.first().ok_or(Errno::EINVAL)?,
            *$buf.get(1).ok_or(Errno::EINVAL)?,
            *$buf.get(2).ok_or(Errno::EINVAL)?,
            *$buf.get(3).ok_or(Errno::EINVAL)?,
        ])
    }};
}

macro_rules! read_u32 {
    ( $session:ident, $buf:ident ) => {{
        let n = $session.0.read($buf.get_mut(..4).ok_or(Errno::EINVAL)?)?;
        if n != 4 {
            return Err(Errno::EINVAL);
        }
        u32::from_le_bytes([
            *$buf.first().ok_or(Errno::EINVAL)?,
            *$buf.get(1).ok_or(Errno::EINVAL)?,
            *$buf.get(2).ok_or(Errno::EINVAL)?,
            *$buf.get(3).ok_or(Errno::EINVAL)?,
        ])
    }};
}

macro_rules! read_i64 {
    ( $session:ident, $buf:ident ) => {{
        let n = $session.0.read($buf.get_mut(..8).ok_or(Errno::EINVAL)?)?;
        if n != 8 {
            return Err(Errno::EINVAL);
        }
        i64::from_le_bytes([
            *$buf.first().ok_or(Errno::EINVAL)?,
            *$buf.get(1).ok_or(Errno::EINVAL)?,
            *$buf.get(2).ok_or(Errno::EINVAL)?,
            *$buf.get(3).ok_or(Errno::EINVAL)?,
            *$buf.get(4).ok_or(Errno::EINVAL)?,
            *$buf.get(5).ok_or(Errno::EINVAL)?,
            *$buf.get(6).ok_or(Errno::EINVAL)?,
            *$buf.get(7).ok_or(Errno::EINVAL)?,
        ])
    }};
}

macro_rules! read_pipe {
    ( $session:ident, $buf:ident ) => {{
        let read_fd = read_i32!($session, $buf);
        let write_fd = read_i32!($session, $buf);
        (Fd::from_raw(read_fd), Fd::from_raw(write_fd))
    }};
}

impl SessionFd {
    pub fn resume_or_new(svcs: &mut [Service; SERVICE_COUNT], ipc_server: &mut IpcServer) -> Self {
        let old_fd = Fd::from_raw(FD_SESSION_STATE);
        if old_fd.is_valid() {
            let fd = Self(old_fd);
            fd.deserialize(svcs).or_abort("Unable to load session");
            ipc_server.respond(Response::Okay);
            fd
        } else {
            Fd::new_memfd(c"connate", MemfdFlags::empty())
                .or_abort("Unable to create memfd")
                .move_to(FD_SESSION_STATE)
                .map(Self)
                .or_abort("Unable to move memfd to fixed FD")
        }
    }

    fn deserialize<const N: usize>(&self, svcs: &mut [Service; N]) -> Result<(), Errno> {
        self.0.lseek(0, SeekWhence::SEEK_SET)?;
        let mut buf = [0u8; SESSION_SERVICE_SIZE];

        // Fields may be left out, in which case we want the default value.
        //
        // Set these to the initial value to handle that case.
        let mut svc = None;
        let mut state = State::Down;
        let mut target = Target::Down;
        let mut pid: Option<pid_t> = None;
        let mut supervisor_pid: Option<pid_t> = None;
        let mut stdin_pipe: Option<(Fd, Fd)> = None;
        let mut exit_code: Option<c_int> = None;
        let mut attempt_count: u32 = 0;
        let mut time_sec: i64 = 0;
        let mut time_nsec: i64 = 0;
        let mut ready: bool = false;
        let mut settle_pipe: Option<(Fd, Fd)> = None;

        loop {
            let n = self.0.read(buf.get_mut(..1).ok_or(Errno::EINVAL)?)?;
            if n == 0 {
                break;
            }

            let header = *buf.first().ok_or(Errno::EINVAL)?;
            let Ok(header) = SessionField::try_from(header) else {
                // We will only see an unrecognized header if the user tries to go from a newer
                // connate version to an older one. This is both unsupported and unlikely for
                // users to try.
                //
                // However, in the unlikely event this occurs anyways, throwing away
                // recoverable state data and aborting with an error results in a strictly
                // worse experience than trying to recover. Thus, we should try to continue.
                //
                // If either:
                // - The new field is only a header byte without following data
                // - The new field has following data that (by pure chance) does not resemble
                // any known header bytes
                //
                // Then we can handle it by simply ignoring the unexpected bytes.
                //
                // If the new field has some other following data that looks like a header,
                // there's nothing we can do.
                continue;
            };

            match header {
                SessionField::ServiceStart => {
                    // Read two bytes for name length
                    let name_len = read_u16!(self, buf) as usize;
                    // Read name
                    let n = self.0.read(buf.get_mut(..name_len).ok_or(Errno::EINVAL)?)?;
                    if n != name_len {
                        return Err(Errno::EINVAL);
                    }
                    let name = buf.get(..name_len).ok_or(Errno::EINVAL)?;
                    // Find matching service in our array
                    //
                    // Note the new session may lack the previous session's service, and this can
                    // be None.
                    //
                    // We want to continue reading out an outdated service's data to consume it to
                    // potentially read a still valid following service.
                    svc = svcs.find_by_name_mut(name);

                    // Initialize other fields to default values.
                    state = State::Down;
                    target = Target::Down;
                    pid = None;
                    supervisor_pid = None;
                    stdin_pipe = None;
                    exit_code = None;
                    attempt_count = 0;
                    time_sec = 0;
                    time_nsec = 0;
                    ready = false;
                    settle_pipe = None;
                }

                SessionField::ServiceEnd => {
                    // Apply state to matching service or handle unrecognized service
                    if let Some(svc) = svc.as_mut() {
                        svc.state = state;
                        svc.target = target;
                        svc.pid = pid;
                        svc.supervisor_pid = supervisor_pid;
                        if svc.cfg.is_logger {
                            svc.stdin_pipe = stdin_pipe.take();
                        } else if let Some((read_fd, write_fd)) = stdin_pipe.take() {
                            let _ = read_fd.close();
                            let _ = write_fd.close();
                        }
                        svc.exit_code = exit_code;
                        svc.attempt_count = attempt_count;
                        svc.time = timespec {
                            tv_sec: time_sec,
                            tv_nsec: time_nsec,
                        };
                        svc.ready = ready;

                        // settle_pipe handling:
                        // - If feature enabled: assign to svc.settle_pipe
                        // - If feature disabled: close both FDs to avoid leak
                        #[cfg(feature = "settle")]
                        {
                            svc.settle_pipe = settle_pipe.take();
                        }
                        #[cfg(not(feature = "settle"))]
                        if let Some((read_fd, write_fd)) = settle_pipe {
                            let _ = read_fd.close();
                            let _ = write_fd.close();
                        }
                    } else {
                        // Unrecognized service: SIGTERM any running processes
                        if let Some(pid) = pid {
                            let _ = kill(pid, Signal::SIGTERM);
                        }
                        if let Some(pid) = supervisor_pid {
                            let _ = kill(pid, Signal::SIGTERM);
                        }
                        // Close any pipe FDs to avoid leaks
                        if let Some((read_fd, write_fd)) = stdin_pipe.take() {
                            let _ = read_fd.close();
                            let _ = write_fd.close();
                        }
                        if let Some((read_fd, write_fd)) = settle_pipe.take() {
                            let _ = read_fd.close();
                            let _ = write_fd.close();
                        }
                    }
                }

                SessionField::StateDown => state = State::Down,
                SessionField::StateWaitingToStart => state = State::WaitingToStart,
                SessionField::StateSettingUp => state = State::SettingUp,
                SessionField::StateStarting => state = State::Starting,
                SessionField::StateUp => state = State::Up,
                SessionField::StateWaitingToStop => state = State::WaitingToStop,
                SessionField::StateStopping => state = State::Stopping,
                SessionField::StateCleaningUp => state = State::CleaningUp,
                SessionField::StateRetrying => state = State::Retrying,
                SessionField::StateFailed => state = State::Failed,
                SessionField::StateForceDown => state = State::ForceDown,
                SessionField::StateCannotStop => state = State::CannotStop,

                SessionField::TargetDown => target = Target::Down,
                SessionField::TargetUp => target = Target::Up,
                SessionField::TargetRestart => target = Target::Restart,
                SessionField::TargetOnce => target = Target::Once,

                SessionField::Pid => {
                    pid = Some(read_i32!(self, buf));
                    if pid.is_some_and(|pid| pid <= 0) {
                        pid = None;
                    }
                }
                SessionField::SupervisorPid => {
                    supervisor_pid = Some(read_i32!(self, buf));
                    if supervisor_pid.is_some_and(|pid| pid <= 0) {
                        supervisor_pid = None;
                    }
                }

                SessionField::StdinPipe => stdin_pipe = Some(read_pipe!(self, buf)),

                SessionField::ReturnValue => exit_code = Some(read_i32!(self, buf)),

                SessionField::SettlePipe => settle_pipe = Some(read_pipe!(self, buf)),

                SessionField::AttemptCount => attempt_count = read_u32!(self, buf),

                SessionField::TimeSec => time_sec = read_i64!(self, buf),
                SessionField::TimeNsec => {
                    time_nsec = read_i64!(self, buf);
                    if !(0..=999_999_999).contains(&time_nsec) {
                        time_nsec = 0;
                    }
                }

                SessionField::Ready => ready = true,
            }
        }

        Ok(())
    }

    pub fn save<const N: usize>(&mut self, svcs: &[Service; N]) -> Result<(), Errno> {
        self.0.lseek(0, SeekWhence::SEEK_SET)?;
        self.0.ftruncate(0)?;
        let mut buf = [0u8; SESSION_SERVICE_SIZE];
        let mut writer = BufWriter::new(&mut buf);

        for svc in svcs {
            writer.reset();

            // ServiceStart + name
            writer.push(&[SessionField::ServiceStart.as_byte()])?;
            let name_len = svc.cfg.name.len() as u16;
            writer.push(&name_len.to_le_bytes())?;
            writer.push(svc.cfg.name)?;

            let state_header = match svc.state {
                State::Down => SessionField::StateDown,
                State::WaitingToStart => SessionField::StateWaitingToStart,
                State::SettingUp => SessionField::StateSettingUp,
                State::Starting => SessionField::StateStarting,
                State::Up => SessionField::StateUp,
                State::WaitingToStop => SessionField::StateWaitingToStop,
                State::Stopping => SessionField::StateStopping,
                State::CleaningUp => SessionField::StateCleaningUp,
                State::Retrying => SessionField::StateRetrying,
                State::Failed => SessionField::StateFailed,
                State::ForceDown => SessionField::StateForceDown,
                State::CannotStop => SessionField::StateCannotStop,
            };
            writer.push(&[state_header.as_byte()])?;

            let target_header = match svc.target {
                Target::Down => SessionField::TargetDown,
                Target::Up => SessionField::TargetUp,
                Target::Restart => SessionField::TargetRestart,
                Target::Once => SessionField::TargetOnce,
            };
            writer.push(&[target_header.as_byte()])?;

            if let Some(pid) = svc.pid {
                writer.push(&[SessionField::Pid.as_byte()])?;
                writer.push(&pid.to_le_bytes())?;
            }

            if let Some(supervisor_pid) = svc.supervisor_pid {
                writer.push(&[SessionField::SupervisorPid.as_byte()])?;
                writer.push(&supervisor_pid.to_le_bytes())?;
            }

            if let Some((read_fd, write_fd)) = &svc.stdin_pipe {
                writer.push(&[SessionField::StdinPipe.as_byte()])?;
                writer.push(&read_fd.as_raw().to_le_bytes())?;
                writer.push(&write_fd.as_raw().to_le_bytes())?;
            }

            if let Some(exit_code) = svc.exit_code {
                writer.push(&[SessionField::ReturnValue.as_byte()])?;
                writer.push(&exit_code.to_le_bytes())?;
            }

            #[cfg(feature = "settle")]
            if let Some((read_fd, write_fd)) = &svc.settle_pipe {
                writer.push(&[SessionField::SettlePipe.as_byte()])?;
                writer.push(&read_fd.as_raw().to_le_bytes())?;
                writer.push(&write_fd.as_raw().to_le_bytes())?;
            }

            // Integer fields (only if non-zero)
            if svc.attempt_count != 0 {
                writer.push(&[SessionField::AttemptCount.as_byte()])?;
                writer.push(&svc.attempt_count.to_le_bytes())?;
            }

            if svc.time.tv_sec != 0 {
                writer.push(&[SessionField::TimeSec.as_byte()])?;
                writer.push(&svc.time.tv_sec.to_le_bytes())?;
            }

            if svc.time.tv_nsec != 0 {
                writer.push(&[SessionField::TimeNsec.as_byte()])?;
                writer.push(&svc.time.tv_nsec.to_le_bytes())?;
            }

            // Boolean flags (header only if true)
            if svc.ready {
                writer.push(&[SessionField::Ready.as_byte()])?;
            }

            // ServiceEnd
            writer.push(&[SessionField::ServiceEnd.as_byte()])?;

            let n = self.0.write(writer.as_slice())?;
            if n != writer.pos() {
                return Err(Errno::EINVAL);
            }
        }

        Ok(())
    }
}
