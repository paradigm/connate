use crate::constants::*;
use crate::err::*;
use crate::types::{StrLen, pid_t};
use crate::util::BufWriter;
use core::ffi::CStr;

// Macro to define both the `enum Request` and `enum RequestHeader` without typo-prone duplication
macro_rules! request_defs {
    ($(
        // Variant(args...) = header
        $variant:ident $( ( $($arg:ty),* ) )? = $header:expr ;
    )*) => {
        /// Request from conctl to connate
        pub enum Request<'a> {
            $(
                $variant $( ( $($arg),* ) )?,
            )*
        }

        #[repr(u8)]
        pub enum RequestHeader {
            $(
                $variant = $header,
            )*
        }

        impl<'a> Request<'a> {
            fn as_header_byte(&self) -> u8 {
                match self {
                    $(
                        request_defs!(@pat $variant $( ( $( $arg ),* ) )? ) => {
                            RequestHeader::$variant as u8
                        }
                    )*
                }
            }
        }

        impl core::convert::TryFrom<u8> for RequestHeader {
            type Error = ();

            fn try_from(value: u8) -> Result<Self, Self::Error> {
                match value {
                    $(
                        $header => Ok(RequestHeader::$variant),
                    )*
                    _ => Err(())
                }
            }
        }
    };

    // 0 args
    (@pat $variant:ident) => {
        Request::$variant
    };
    // 1 arg
    (@pat $variant:ident ( $a:ty )) => {
        Request::$variant(_)
    };
    // 2 args
    (@pat $variant:ident ( $a:ty, $b:ty )) => {
        Request::$variant(_, _)
    };
}

// IPC Requests
request_defs! {
    // Exec a (presumably new) connate binary to change configuration
    Exec(&'a CStr) = b'x';

    // Queries by index
    QueryByIndexStatus(usize) = b'a';
    QueryByIndexName(usize) = b'n';
    QueryByIndexState(usize) = b's';
    QueryByIndexTarget(usize) = b't';
    QueryByIndexPid(usize) = b'p';
    QueryByIndexExitCode(usize) = b'e';
    QueryByIndexAttemptCount(usize) = b'c';
    QueryByIndexTime(usize) = b'i';

    // Queries by name
    QueryByNameStatus(&'a [u8]) = b'A';
    QueryByNameState(&'a [u8]) = b'S';
    QueryByNameTarget(&'a [u8]) = b'T';
    QueryByNamePid(&'a [u8]) = b'P';
    QueryByNameExitCode(&'a [u8]) = b'E';
    QueryByNameAttemptCount(&'a [u8]) = b'C';
    QueryByNameTime(&'a [u8]) = b'I';

    // Queries about dependency information
    // - &'a [u8] is service name
    // - usize is index into dependency list
    QueryNeeds(usize, &'a [u8]) = b'm';
    QueryWants(usize, &'a [u8]) = b'w';
    QueryConflicts(usize, &'a [u8]) = b'f';
    QueryGroups(usize, &'a [u8]) = b'g';
    QueryByIndexLog(usize) = b'l';
    QueryByNameLog(&'a [u8]) = b'L';

    // Set target by service name
    SetTargetUp(&'a [u8]) = b'u';
    SetTargetDown(&'a [u8]) = b'd';
    SetTargetRestart(&'a [u8]) = b'r';
    SetTargetOnce(&'a [u8]) = b'o';

    // Query the settle pipe FD for a service by name
    //
    // Returns the read-end FD number which conctl can poll() on to wait for stable state.
    // Creates the settle pipe lazily if it doesn't exist.
    QuerySettleFd(&'a [u8]) = b'q';

    // Messages from service or supervisor about readiness
    ServiceStarting(pid_t, &'a [u8]) = b'G';
    ServiceReady(pid_t) = b'y';
    DaemonReady(pid_t, &'a [u8]) = b'Y';

    // An invalid request
    //
    // We can't just abort with an error or ignore a bad request. We need to respond to avoid
    // having a buggy conctl hang while locking the communication pipe
    Invalid = b'z';
}

impl<'a> Request<'a> {
    pub fn serialize(self, buf: &mut [u8; MSG_SIZE]) -> Result<usize, Errno> {
        let mut writer = BufWriter::new(buf);

        let header = self.as_header_byte();
        writer.push(&[header])?;

        match self {
            Request::Invalid => {}

            // pid (pid_t)
            Request::ServiceReady(pid) => {
                writer.push(&pid.to_le_bytes())?;
            }

            // index (usize)
            Request::QueryByIndexStatus(n)
            | Request::QueryByIndexName(n)
            | Request::QueryByIndexState(n)
            | Request::QueryByIndexTarget(n)
            | Request::QueryByIndexPid(n)
            | Request::QueryByIndexAttemptCount(n)
            | Request::QueryByIndexExitCode(n)
            | Request::QueryByIndexTime(n)
            | Request::QueryByIndexLog(n) => {
                writer.push(&n.to_le_bytes())?;
            }

            // Service name (&[u8])
            Request::QueryByNameStatus(name)
            | Request::QueryByNameState(name)
            | Request::QueryByNameTarget(name)
            | Request::QueryByNamePid(name)
            | Request::QueryByNameAttemptCount(name)
            | Request::QueryByNameExitCode(name)
            | Request::QueryByNameTime(name)
            | Request::QueryByNameLog(name)
            | Request::SetTargetUp(name)
            | Request::SetTargetDown(name)
            | Request::SetTargetRestart(name)
            | Request::SetTargetOnce(name)
            | Request::QuerySettleFd(name) => {
                debug_assert!(name.len() <= MSG_SVC_NAME_SIZE);
                let len = name.len() as StrLen;
                writer.push(&len.to_le_bytes())?;
                writer.push(name)?;
            }

            // Path (&CStr)
            Request::Exec(cstr) => {
                let bytes = cstr.to_bytes_with_nul();
                // `+ 1` for trailing null
                if bytes.len() > MSG_PATH_SIZE + 1 {
                    return Err(Errno::EOVERFLOW);
                }
                let len = bytes.len() as StrLen;
                writer.push(&len.to_le_bytes())?;
                writer.push(bytes)?;
            }

            // index (usize) + name (&[u8])
            Request::QueryNeeds(index, name)
            | Request::QueryWants(index, name)
            | Request::QueryConflicts(index, name)
            | Request::QueryGroups(index, name) => {
                debug_assert!(name.len() <= MSG_SVC_NAME_SIZE);
                writer.push(&index.to_le_bytes())?;
                let len = name.len() as StrLen;
                writer.push(&len.to_le_bytes())?;
                writer.push(name)?;
            }

            // pid (pid) + name (&[u8])
            Request::ServiceStarting(pid, name) | Request::DaemonReady(pid, name) => {
                debug_assert!(name.len() <= MSG_SVC_NAME_SIZE);
                writer.push(&pid.to_le_bytes())?;
                let len = name.len() as StrLen;
                writer.push(&len.to_le_bytes())?;
                writer.push(name)?;
            }
        }

        Ok(writer.pos())
    }

    pub fn deserialize(buf: &'a [u8]) -> Self {
        let Some(&header) = buf.first() else {
            return Self::Invalid;
        };
        let mut offset = size_of::<u8>();

        macro_rules! read {
            (&str) => {{
                // Read out str_length first
                const LEN: usize = core::mem::size_of::<StrLen>();
                let bytes: [u8; LEN] = match buf.get(offset..offset + LEN).map(|s| s.try_into()) {
                    Some(Ok(s)) => s,
                    _ => return Request::Invalid,
                };
                offset += LEN;
                let str_len = StrLen::from_le_bytes(bytes) as usize;

                // Read out string of expected length
                let str = match buf.get(offset..offset + str_len) {
                    Some(s) => s,
                    None => return Request::Invalid,
                };
                offset += str_len; // sometimes needed, sometimes not
                str
            }};

            (&CStr) => {{
                // Read out str_length first
                const LEN: usize = core::mem::size_of::<StrLen>();
                let bytes: [u8; LEN] = match buf.get(offset..offset + LEN).map(|s| s.try_into()) {
                    Some(Ok(s)) => s,
                    _ => return Request::Invalid,
                };
                offset += LEN;
                let str_len = StrLen::from_le_bytes(bytes) as usize;

                // Read out string of expected length
                let str = match buf.get(offset..offset + str_len) {
                    Some(s) => s,
                    None => return Request::Invalid,
                };
                offset += str_len; // sometimes needed, sometimes not

                match CStr::from_bytes_with_nul(str) {
                    Ok(s) => s,
                    Err(_) => return Request::Invalid,
                }
            }};

            ($ty:ty) => {{
                const LEN: usize = core::mem::size_of::<$ty>();
                let bytes: [u8; LEN] = match buf.get(offset..offset + LEN).map(|s| s.try_into()) {
                    Some(Ok(s)) => s,
                    _ => return Request::Invalid,
                };
                offset += LEN; // sometimes needed, sometimes not
                <$ty>::from_le_bytes(bytes)
            }};
        }

        use Request as R;
        use RequestHeader as RH;

        #[allow(unused_assignments)] // not all read!() invocations use final `offset`
        match RH::try_from(header) {
            Ok(RH::Exec) => R::Exec(read!(&CStr)),
            Ok(RH::QueryByIndexStatus) => R::QueryByIndexStatus(read!(usize)),
            Ok(RH::QueryByIndexName) => R::QueryByIndexName(read!(usize)),
            Ok(RH::QueryByIndexState) => R::QueryByIndexState(read!(usize)),
            Ok(RH::QueryByIndexTarget) => R::QueryByIndexTarget(read!(usize)),
            Ok(RH::QueryByIndexPid) => R::QueryByIndexPid(read!(usize)),
            Ok(RH::QueryByIndexExitCode) => R::QueryByIndexExitCode(read!(usize)),
            Ok(RH::QueryByIndexAttemptCount) => R::QueryByIndexAttemptCount(read!(usize)),
            Ok(RH::QueryByIndexTime) => R::QueryByIndexTime(read!(usize)),
            Ok(RH::QueryByNameStatus) => R::QueryByNameStatus(read!(&str)),
            Ok(RH::QueryByNameState) => R::QueryByNameState(read!(&str)),
            Ok(RH::QueryByNameTarget) => R::QueryByNameTarget(read!(&str)),
            Ok(RH::QueryByNamePid) => R::QueryByNamePid(read!(&str)),
            Ok(RH::QueryByNameExitCode) => R::QueryByNameExitCode(read!(&str)),
            Ok(RH::QueryByNameAttemptCount) => R::QueryByNameAttemptCount(read!(&str)),
            Ok(RH::QueryByNameTime) => R::QueryByNameTime(read!(&str)),
            Ok(RH::QueryNeeds) => R::QueryNeeds(read!(usize), read!(&str)),
            Ok(RH::QueryWants) => R::QueryWants(read!(usize), read!(&str)),
            Ok(RH::QueryConflicts) => R::QueryConflicts(read!(usize), read!(&str)),
            Ok(RH::QueryGroups) => R::QueryGroups(read!(usize), read!(&str)),
            Ok(RH::QueryByIndexLog) => R::QueryByIndexLog(read!(usize)),
            Ok(RH::QueryByNameLog) => R::QueryByNameLog(read!(&str)),
            Ok(RH::SetTargetUp) => R::SetTargetUp(read!(&str)),
            Ok(RH::SetTargetDown) => R::SetTargetDown(read!(&str)),
            Ok(RH::SetTargetRestart) => R::SetTargetRestart(read!(&str)),
            Ok(RH::SetTargetOnce) => R::SetTargetOnce(read!(&str)),
            Ok(RH::QuerySettleFd) => R::QuerySettleFd(read!(&str)),
            Ok(RH::ServiceStarting) => R::ServiceStarting(read!(pid_t), read!(&str)),
            Ok(RH::ServiceReady) => R::ServiceReady(read!(pid_t)),
            Ok(RH::DaemonReady) => R::DaemonReady(read!(pid_t), read!(&str)),
            Ok(RH::Invalid) | Err(()) => R::Invalid,
        }
    }
}
