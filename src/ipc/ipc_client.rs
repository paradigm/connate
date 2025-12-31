use crate::constants::*;
use crate::err::*;
use crate::ipc::{Request, Response};
use crate::os::{Fd, OpenFlags, eprint};
use crate::types::*;
use crate::util::{BufWriter, memzero};
use itoa::Integer; // ::MAX_STR_LEN

/// cctl and supervisor side of communication channel to connate
pub struct IpcClient {
    fd_req_write: Fd,
    fd_resp_read: Fd,
    buf: [u8; MSG_SIZE],
}

impl<'a> IpcClient {
    pub fn from_pid(connate_pid: pid_t) -> Self {
        const COMM_PATH_SIZE: usize = b"/proc/".len()
            + pid_t::MAX_STR_LEN // pid number
            + b"/fd/".len()
            + c_int::MAX_STR_LEN // fd number
            + size_of::<u8>(); // trailing null
        let mut buf = [0u8; COMM_PATH_SIZE];
        let mut writer = BufWriter::new(&mut buf);

        let mut itoa_buf = itoa::Buffer::new();
        let pid_str = itoa_buf.format(connate_pid);

        // Build path: /proc/<pid>/fd/<response-read-fd>\0
        writer
            .push(b"/proc/")
            .and_then(|_| writer.push(pid_str.as_bytes()))
            .and_then(|_| writer.push(b"/fd/"))
            .and_then(|_| writer.push(FD_RESP_READ_STR))
            .and_then(|_| writer.push(b"\0"))
            // This should be unreachable
            .or_abort("buffer overflow");

        // Safety: We just built this buffer including the trailing null
        let read_fd_path: &CStr = unsafe { CStr::from_bytes_with_nul_unchecked(writer.as_slice()) };
        // Open in non-blocking mode to avoid blocking if connate isn't running and to allow draining
        let fd_resp_read = Fd::open(read_fd_path, OpenFlags::O_RDONLY | OpenFlags::O_NONBLOCK, 0)
            .or_fs_abort("open", read_fd_path);

        // Build path: /proc/<pid>/fd/<request-write-fd>\0
        writer.reset();
        writer
            .push(b"/proc/")
            .and_then(|_| writer.push(pid_str.as_bytes()))
            .and_then(|_| writer.push(b"/fd/"))
            .and_then(|_| writer.push(FD_REQ_WRITE_STR))
            .and_then(|_| writer.push(b"\0"))
            // This should be unreachable
            .or_abort("buffer overflow");

        // Safety: buffer was zero-initialized, guaranteeing following unread byte is null
        let write_fd_path: &CStr =
            unsafe { CStr::from_bytes_with_nul_unchecked(writer.as_slice()) };
        let fd_req_write =
            Fd::open(write_fd_path, OpenFlags::O_RDWR, 0).or_fs_abort("open", write_fd_path);

        // Clear any stale data in the response pipe while it's still non-blocking
        let mut drain_buf = [0u8; MSG_SIZE];
        loop {
            match fd_resp_read.read(&mut drain_buf) {
                Ok(_) => continue,           // Keep draining
                Err(Errno::EAGAIN) => break, // Pipe is empty (EWOULDBLOCK is same value)
                Err(_) => break,             // Other error, stop trying
            }
        }

        // Switch to blocking mode to wait for connate's response to our request
        fd_resp_read
            .set_blocking()
            .or_abort("Failed to set response pipe to blocking mode");

        Self {
            fd_req_write,
            fd_resp_read,
            buf: [0u8; MSG_SIZE],
        }
    }

    #[cfg(test)]
    pub fn new_test(fd_req_write: Fd, fd_resp_read: Fd) -> Self {
        Self {
            fd_req_write,
            fd_resp_read,
            buf: [0u8; MSG_SIZE],
        }
    }

    pub fn unlock(&mut self) {
        self.fd_req_write
            .unlock()
            .or_fs_abort("unlock", c"connate request pipe");
    }

    pub fn lock_with_warning(&mut self) {
        match self.fd_req_write.lock_nonblocking() {
            Ok(()) => {}
            Err(err) if err == Errno::EACCES || err == Errno::EAGAIN => {
                let locking_pid = self
                    .fd_req_write
                    .get_locking_pid()
                    .or_fs_abort("get lock holder", c"connate request pipe");

                if let Some(pid) = locking_pid {
                    eprint("WARNING: Waiting for PID ");
                    eprint(pid);
                    eprint(" to release lock... ");
                } else {
                    eprint("WARNING: Waiting for another process to release lock... ");
                }

                self.fd_req_write
                    .lock_blocking()
                    .or_fs_abort("lock", c"connate request pipe");
            }
            Err(e) => Err::<(), Errno>(e).or_fs_abort("lock", c"connate request pipe"),
        }
    }

    pub fn lock_quiet(&mut self) {
        self.fd_req_write
            .lock_blocking()
            .or_fs_abort("lock", c"connate request pipe");
    }

    pub fn send_and_receive(&'a mut self, request: Request) -> Response<'a> {
        let msg_len = request
            .serialize(&mut self.buf)
            .or_abort("Unable to serialize request to connate");

        // Write only the necessary bytes to request pipe. Messages are variable-length, so we only
        // send what we need. Since msg_len ≤ PIPE_BUF, POSIX guarantees the write is atomic on
        // non-blocking pipes (all bytes written or EAGAIN error).
        let _ = self
            .fd_req_write
            .write(self.buf.get(..msg_len).or_abort("Invalid message length"))
            .or_fs_abort("write", c"connate request pipe");

        memzero(&mut self.buf);

        // Read response from pipe. We read into the full buffer; the sender may send less than
        // MSG_SIZE bytes, but read() will return whatever is available. The deserializer handles
        // variable-length messages based on the message header and structure.
        let _ = self
            .fd_resp_read
            .read(&mut self.buf)
            .or_fs_abort("read", c"connate response pipe");

        Response::deserialize(&self.buf).or_abort("Unable to deserialize response from connate")
    }
}

// Naively, one might expect us to unlock or close FDs on drop.  However, the kernel handles this on
// process death such that it is unneeded.
//
// impl Drop for IpcClient {
//     fn drop(&mut self) {
//         todo!();
//     }
// }
