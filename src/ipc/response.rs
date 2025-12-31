use crate::constants::*;
use crate::err::Errno;
use crate::internal_api::{State, Target};
use crate::os::{Print, print, print_color};
use crate::types::{StrLen, c_int, pid_t};
use crate::util::BufWriter;

// Macro to define both the `enum Response` and `enum ResponseHeader` without typo-prone duplication
macro_rules! response_defs {
    ($(
        // Variant(args...) = header
        $variant:ident $( ( $($arg:ty),* ) )? = $header:expr ;
    )*) => {
        /// Response from connate to conctl
        pub enum Response<'a> {
            $(
                $variant $( ( $($arg),* ) )?,
            )*
        }

        #[repr(u8)]
        pub enum ResponseHeader {
            $(
                $variant = $header,
            )*
        }

        impl<'a> Response<'a> {
            fn as_header_byte(&self) -> u8 {
                match self {
                    $(
                        response_defs!(@pat $variant $( ( $( $arg ),* ) )? ) => {
                            ResponseHeader::$variant as u8
                        }
                    )*
                }
            }
        }

        impl core::convert::TryFrom<u8> for ResponseHeader {
            type Error = ();

            fn try_from(value: u8) -> Result<Self, Self::Error> {
                match value {
                    $(
                        $header => Ok(ResponseHeader::$variant),
                    )*
                    _ => Err(())
                }
            }
        }
    };

    // 0 args
    (@pat $variant:ident) => {
        Response::$variant
    };
    // 1 arg
    (@pat $variant:ident ( $a:ty )) => {
        Response::$variant(_)
    };
    // 2 args
    (@pat $variant:ident ( $a:ty, $b:ty )) => {
        Response::$variant(_, _)
    };
    // 3 args
    (@pat $variant:ident ( $a:ty, $b:ty, $c:ty )) => {
        Response::$variant(_, _, _)
    };
    // 4 args
    (@pat $variant:ident ( $a:ty, $b:ty, $c:ty, $d:ty )) => {
        Response::$variant(_, _, _, _)
    };
    // 5 args
    (@pat $variant:ident ( $a:ty, $b:ty, $c:ty, $d:ty, $e:ty )) => {
        Response::$variant(_, _, _, _, _)
    };
}

// IPC Responses
response_defs! {
    // General responses
    Okay = b'o';
    Failed = b'z';
    ServiceNotFound = b'x';
    FieldIsNone = b'X';
    InvalidRequest = b'Z';
    SettleDisabled = b'Q';

    // Response to query about field(s)
    Status(State, Target, Option<pid_t>, Option<c_int>, i64) = b'S';
    State(State) = b's';
    Target(Target) = b't';
    Pid(pid_t) = b'p';
    ExitCode(c_int) = b'e';
    AttemptCount(u64) = b'c';
    Time(i64) = b'T';
    Name(&'a [u8]) = b'n';
    Path(&'a [u8]) = b'P';
    SettleFd(c_int) = b'q';
}

impl<'a> Response<'a> {
    pub fn serialize(self, buf: &mut [u8; MSG_SIZE]) -> Result<usize, Errno> {
        let mut writer = BufWriter::new(buf);

        let header = self.as_header_byte();
        writer.push(&[header])?;

        match self {
            Response::Okay
            | Response::Failed
            | Response::ServiceNotFound
            | Response::FieldIsNone
            | Response::InvalidRequest
            | Response::SettleDisabled => {}
            Response::State(state) => writer.push(&[state.as_byte()])?,

            Response::Target(target) => writer.push(&[target.as_byte()])?,

            Response::Pid(pid) => writer.push(&pid.to_le_bytes())?,

            Response::SettleFd(fd) => writer.push(&fd.to_le_bytes())?,

            Response::ExitCode(code) => writer.push(&code.to_le_bytes())?,

            Response::AttemptCount(count) => writer.push(&count.to_le_bytes())?,

            Response::Time(time) => writer.push(&time.to_le_bytes())?,

            Response::Status(state, target, pid, code, time) => {
                writer.push(&[state.as_byte()])?;
                writer.push(&[target.as_byte()])?;
                // Serialize Option<pid_t> with sentinel for None
                let pid_wire: pid_t = pid.unwrap_or(MSG_PID_NONE_SENTINEL);
                writer.push(&pid_wire.to_le_bytes())?;
                // Serialize Option<c_int> with sentinel for None
                let code_wire: c_int = code.unwrap_or(MSG_EXIT_CODE_NONE_SENTINEL);
                writer.push(&code_wire.to_le_bytes())?;
                writer.push(&time.to_le_bytes())?;
            }

            Response::Name(name) => {
                // Should be checked at compile-time
                //
                // If somehow it fails at --release runtime, writer will return EOVERFLOW
                debug_assert!(name.len() <= MSG_SVC_NAME_SIZE);
                let len = name.len() as StrLen;
                writer.push(&len.to_le_bytes())?;
                writer.push(name)?;
            }

            Response::Path(path) => {
                // Should be checked at compile-time
                //
                // If somehow it fails at --release runtime, writer will return EOVERFLOW
                debug_assert!(path.len() <= MSG_PATH_SIZE);
                let len = path.len() as StrLen;
                writer.push(&len.to_le_bytes())?;
                writer.push(path)?;
            }
        }

        Ok(writer.pos())
    }

    pub fn deserialize(buf: &'a [u8; MSG_SIZE]) -> Result<Self, Errno> {
        let header = buf[0];
        let mut offset = size_of::<u8>();

        macro_rules! read {
            (u8) => {{
                let byte = match buf.get(offset) {
                    Some(s) => s,
                    None => return Err(Errno::EINVAL),
                };
                offset += 1;
                *byte
            }};

            (&str) => {{
                // Read out str_length first
                const LEN: usize = core::mem::size_of::<StrLen>();
                let bytes: [u8; LEN] = match buf.get(offset..offset + LEN).map(|s| s.try_into()) {
                    Some(Ok(s)) => s,
                    _ => return Err(Errno::EINVAL),
                };
                offset += LEN;
                let str_len = StrLen::from_le_bytes(bytes) as usize;

                // Read out string of expected length
                let str = buf.get(offset..offset + str_len).ok_or(Errno::EINVAL)?;
                offset += str.len();

                str
            }};

            ($ty:ty) => {{
                const LEN: usize = core::mem::size_of::<$ty>();
                let bytes: [u8; LEN] = match buf.get(offset..offset + LEN).map(|s| s.try_into()) {
                    Some(Ok(s)) => s,
                    _ => return Err(Errno::EINVAL),
                };
                offset += LEN;
                <$ty>::from_le_bytes(bytes)
            }};
        }

        use Response as R;
        use ResponseHeader as RH;

        #[allow(unused_assignments)] // not all read!() invocations use final `offset`
        match RH::try_from(header) {
            Ok(RH::Okay) => Ok(R::Okay),
            Ok(RH::Failed) => Ok(R::Failed),
            Ok(RH::ServiceNotFound) => Ok(R::ServiceNotFound),
            Ok(RH::FieldIsNone) => Ok(R::FieldIsNone),
            Ok(RH::InvalidRequest) => Ok(R::InvalidRequest),
            Ok(RH::SettleDisabled) => Ok(R::SettleDisabled),
            Ok(RH::Status) => {
                let state = State::from_byte(read!(u8))?;
                let target = Target::from_byte(read!(u8))?;
                let pid_wire = read!(pid_t);
                let pid = (pid_wire != MSG_PID_NONE_SENTINEL).then_some(pid_wire);
                let code_wire = read!(c_int);
                let code = (code_wire != MSG_EXIT_CODE_NONE_SENTINEL).then_some(code_wire);
                let time = read!(i64);
                Ok(R::Status(state, target, pid, code, time))
            }
            Ok(RH::State) => Ok(R::State(State::from_byte(read!(u8))?)),
            Ok(RH::Target) => Ok(R::Target(Target::from_byte(read!(u8))?)),
            Ok(RH::Pid) => Ok(R::Pid(read!(pid_t))),
            Ok(RH::SettleFd) => Ok(R::SettleFd(read!(c_int))),
            Ok(RH::ExitCode) => Ok(R::ExitCode(read!(c_int))),
            Ok(RH::AttemptCount) => Ok(R::AttemptCount(read!(u64))),
            Ok(RH::Time) => Ok(R::Time(read!(i64))),
            Ok(RH::Name) => Ok(R::Name(read!(&str))),
            Ok(RH::Path) => Ok(R::Path(read!(&str))),
            Err(()) => Err(Errno::EINVAL),
        }
    }

    pub fn cmd_return_failed(&self) -> bool {
        matches!(
            self,
            Response::ServiceNotFound
                | Response::Failed
                | Response::InvalidRequest
                | Response::SettleDisabled
        )
    }

    pub fn cmd_return_success(&self) -> bool {
        !self.cmd_return_failed()
    }
}

/// Width tracking for Status response fields
#[derive(Clone, Copy, Default)]
pub struct StatusWidths {
    pub state: usize,
    pub target: usize,
    pub pid: usize,
    pub exit_code: usize,
}

impl StatusWidths {
    pub fn update(&mut self, state: usize, target: usize, pid: usize, exit_code: usize) {
        self.state = core::cmp::max(state, self.state);
        self.target = core::cmp::max(target, self.target);
        self.pid = core::cmp::max(pid, self.pid);
        self.exit_code = core::cmp::max(exit_code, self.exit_code);
    }
}

impl<'a> Print for Response<'a> {
    fn print(&self, _fd: crate::os::Fd) {
        use crate::os::Color::*;
        match *self {
            Response::Okay => print_color(Okay, "okay"),
            Response::Failed => print_color(Error, "failed"),
            Response::ServiceNotFound => print_color(NotFound, "not-found"),
            Response::FieldIsNone => print_color(Dim, "N/A"),
            Response::InvalidRequest => print_color(Error, "invalid-request"),
            Response::SettleDisabled => print_color(Error, "settle-disabled"),
            Response::SettleFd(fd) => print(fd),
            Response::Status(state, target, pid, code, time) => {
                print("state");
                print_color(Glue, "=");
                print(state);
                print(" target");
                print_color(Glue, "=");
                print(target);
                print(" pid");
                print_color(Glue, "=");
                match pid {
                    Some(p) => print(p),
                    None => print_color(Dim, "N/A"),
                }
                print(" code");
                print_color(Glue, "=");
                match code {
                    Some(v) if v == 0 => print_color(Okay, v),
                    Some(v) => print_color(Error, v),
                    None => print_color(Dim, "N/A"),
                }
                print(" time");
                print_color(Glue, "=");
                print_time(time);
            }
            Response::State(state) => print(state),
            Response::Target(target) => print(target),
            Response::Pid(pid) => print(pid),
            Response::ExitCode(code) => {
                if code == 0 {
                    print_color(Okay, code)
                } else {
                    print_color(Error, code)
                }
            }
            Response::AttemptCount(count) => print_color(Transition, count),
            Response::Time(time) => print_time(time),
            Response::Name(name) => print_color(Service, name),
            Response::Path(path) => print_color(Service, path),
        }
    }

    fn print_len(&self) -> usize {
        match *self {
            Response::Okay => "okay".len(),
            Response::Failed => "failed".len(),
            Response::ServiceNotFound => "not-found".len(),
            Response::FieldIsNone => "N/A".len(),
            Response::InvalidRequest => "invalid-request".len(),
            Response::SettleDisabled => "settle-disabled".len(),
            Response::SettleFd(fd) => fd.print_len(),
            Response::Status(state, target, pid, code, time) => {
                // "state=" + state + " target=" + target + " pid=" + pid + " code=" + val + " time=" + time
                let pid_len = match pid {
                    Some(p) => p.print_len(),
                    None => "N/A".len(),
                };
                let val_len = match code {
                    Some(v) => v.print_len(),
                    None => "N/A".len(),
                };
                "state=".len()
                    + state.print_len()
                    + " target=".len()
                    + target.print_len()
                    + " pid=".len()
                    + pid_len
                    + " code=".len()
                    + val_len
                    + " time=".len()
                    + time_print_len(time)
            }
            Response::State(state) => state.print_len(),
            Response::Target(target) => target.print_len(),
            Response::Pid(pid) => pid.print_len(),
            Response::ExitCode(code) => code.print_len(),
            Response::AttemptCount(count) => count.print_len(),
            Response::Time(time) => time_print_len(time),
            Response::Name(name) => name.len(),
            Response::Path(path) => path.len(),
        }
    }
}

impl<'a> Response<'a> {
    /// Get individual field lengths for Status response
    /// Returns (state_len, target_len, pid_len, return_len) if Status variant, None otherwise
    pub fn status_field_lens(&self) -> Option<(usize, usize, usize, usize)> {
        use crate::os::Print;
        match *self {
            Response::Status(state, target, pid, code, _time) => {
                let pid_len = match pid {
                    Some(p) => p.print_len(),
                    None => "N/A".len(),
                };
                let val_len = match code {
                    Some(v) => v.print_len(),
                    None => "N/A".len(),
                };
                Some((state.print_len(), target.print_len(), pid_len, val_len))
            }
            _ => None,
        }
    }

    /// Print Status response with padding for aligned columns
    pub fn print_status_padded(self, widths: &StatusWidths) {
        use crate::os::Color::*;
        use crate::os::Print;

        match self {
            Response::Status(state, target, pid, code, time) => {
                print("state");
                print_color(Glue, "=");
                print(state);
                state.print_padding(widths.state);
                print(" target");
                print_color(Glue, "=");
                print(target);
                target.print_padding(widths.target);
                print(" pid");
                print_color(Glue, "=");
                match pid {
                    Some(p) => {
                        print(p);
                        p.print_padding(widths.pid);
                    }
                    None => {
                        print_color(Dim, "N/A");
                        "N/A".print_padding(widths.pid);
                    }
                }
                print(" code");
                print_color(Glue, "=");
                match code {
                    Some(v) if v == 0 => {
                        print_color(Okay, v);
                        v.print_padding(widths.exit_code);
                    }
                    Some(v) => {
                        print_color(Error, v);
                        v.print_padding(widths.exit_code);
                    }
                    None => {
                        print_color(Dim, "N/A");
                        "N/A".print_padding(widths.exit_code);
                    }
                }
                // time is last field, no padding
                print(" time");
                print_color(Glue, "=");
                print_time(time);
            }
            response => print(response), // Unexpected response, e.g. error
        }
    }
}

fn time_print_len(seconds: i64) -> usize {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    let mut buf = itoa::Buffer::new();

    if days > 0 {
        buf.format(days).len() + 1 + 2 + 1 + 2 + 1 + 2 + 1 // Xd00h00m00s
    } else if hours > 0 {
        buf.format(hours).len() + 1 + 2 + 1 + 2 + 1 // Xh00m00s
    } else if minutes > 0 {
        buf.format(minutes).len() + 1 + 2 + 1 // Xm00s
    } else {
        buf.format(secs).len() + 1 // Xs
    }
}

fn print_time(seconds: i64) {
    use crate::os::Color::{TimeDay, TimeHour, TimeMinute, TimeSecond};

    // Print %dd%dh%dm%ds format with leading zeros on fields that have following fields
    // Each time unit (number + suffix) is printed in its own shade of gray
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    let mut buf = itoa::Buffer::new();

    if days > 0 {
        print_color(TimeDay, buf.format(days).as_bytes());
        print_color(TimeDay, "d");
        // Hours field has following fields (m and s), so add leading zero if needed
        if hours < 10 {
            print_color(TimeHour, "0");
        }
        print_color(TimeHour, buf.format(hours).as_bytes());
        print_color(TimeHour, "h");
        // Minutes field has following field (s), so add leading zero if needed
        if minutes < 10 {
            print_color(TimeMinute, "0");
        }
        print_color(TimeMinute, buf.format(minutes).as_bytes());
        print_color(TimeMinute, "m");
        // Seconds is last field, add leading zero
        if secs < 10 {
            print_color(TimeSecond, "0");
        }
        print_color(TimeSecond, buf.format(secs).as_bytes());
        print_color(TimeSecond, "s");
    } else if hours > 0 {
        print_color(TimeHour, buf.format(hours).as_bytes());
        print_color(TimeHour, "h");
        // Minutes field has following field (s), so add leading zero if needed
        if minutes < 10 {
            print_color(TimeMinute, "0");
        }
        print_color(TimeMinute, buf.format(minutes).as_bytes());
        print_color(TimeMinute, "m");
        // Seconds is last field, add leading zero
        if secs < 10 {
            print_color(TimeSecond, "0");
        }
        print_color(TimeSecond, buf.format(secs).as_bytes());
        print_color(TimeSecond, "s");
    } else if minutes > 0 {
        print_color(TimeMinute, buf.format(minutes).as_bytes());
        print_color(TimeMinute, "m");
        // Seconds has no following field but minutes is before it, so add leading zero
        if secs < 10 {
            print_color(TimeSecond, "0");
        }
        print_color(TimeSecond, buf.format(secs).as_bytes());
        print_color(TimeSecond, "s");
    } else {
        // Only seconds, no leading zero needed
        print_color(TimeSecond, buf.format(secs).as_bytes());
        print_color(TimeSecond, "s");
    }
}
