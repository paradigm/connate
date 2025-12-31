use crate::constants::*;
use crate::err::*;
use crate::ipc::{Request, Response};
use crate::os::{Fd, OpenFlags};
use crate::types::*;

pub struct IpcServer {
    fd_req_read: Fd,
    fd_resp_write: Fd,
    buf: [u8; MSG_SIZE],
}

#[allow(clippy::new_without_default)]
impl<'b> IpcServer {
    pub fn new() -> Self {
        // The _ignored fds are accessed by IpcClient via /proc/<pid>/fd/<fd>
        //
        // Since Drop is not implemented on Fd, we retain the underlying file descriptors even if
        // we drop the symbols.
        let (fd_req_read, _request_write_fd) = get_pipe(FD_REQ_READ, FD_REQ_WRITE);
        let (_response_read_fd, fd_resp_write) = get_pipe(FD_RESP_READ, FD_RESP_WRITE);

        Self {
            fd_req_read,
            fd_resp_write,
            buf: [0u8; MSG_SIZE],
        }
    }

    #[cfg(test)]
    pub fn new_test(fd_req_read: Fd, fd_resp_write: Fd) -> Self {
        Self {
            fd_req_read,
            fd_resp_write,
            buf: [0u8; MSG_SIZE],
        }
    }

    pub fn try_resume() -> Option<Self> {
        if Fd::from_raw(FD_REQ_READ).is_valid()
            && Fd::from_raw(FD_REQ_WRITE).is_valid()
            && Fd::from_raw(FD_RESP_READ).is_valid()
            && Fd::from_raw(FD_RESP_WRITE).is_valid()
        {
            Some(Self {
                fd_req_read: Fd::from_raw(FD_REQ_READ),
                fd_resp_write: Fd::from_raw(FD_RESP_WRITE),
                buf: [0u8; MSG_SIZE],
            })
        } else {
            // Close any existing FDs to avoid leaks before recreating
            let _ = Fd::from_raw(FD_REQ_READ).close();
            let _ = Fd::from_raw(FD_REQ_WRITE).close();
            let _ = Fd::from_raw(FD_RESP_READ).close();
            let _ = Fd::from_raw(FD_RESP_WRITE).close();
            None
        }
    }

    pub fn fd_req_read(&self) -> &Fd {
        &self.fd_req_read
    }

    pub fn receive(&mut self) -> Request<'_> {
        // Read request from pipe. We read into the full buffer; the sender may send less than
        // MSG_SIZE bytes, but read() will return whatever is available. The deserializer handles
        // variable-length messages based on the message header and structure.
        let msg_len = self
            .fd_req_read
            .read(&mut self.buf)
            .or_fs_abort("read", c"connate request pipe");

        match self.buf.get(0..msg_len) {
            Some(buf) => Request::deserialize(buf),
            None => Request::Invalid,
        }
    }

    pub fn respond(&mut self, response: Response<'b>) {
        let msg_len = response
            .serialize(&mut self.buf)
            .or_abort("Unable to serialize response to client");

        // Write only the necessary bytes to response pipe. Messages are variable-length, so we only
        // send what we need. Since msg_len ≤ PIPE_BUF, POSIX guarantees the write is atomic on
        // non-blocking pipes (all bytes written or EAGAIN error).
        self.fd_resp_write
            .write(self.buf.get(..msg_len).or_abort("Invalid message length"))
            .or_fs_abort("write", c"connate response pipe");
    }
}

fn get_pipe(fd_read_target: c_int, fd_write_target: c_int) -> (Fd, Fd) {
    // Note we do *not* O_CLOEXEC here to ensure the pipe survives a re-exec.
    let pipe_flags = OpenFlags::O_NONBLOCK;
    let (read_orig, write_orig) = Fd::new_pipe(pipe_flags).or_fs_abort("create", c"pipe");

    // Move pipe fds to known values so that conctl can access them via /proc/<pid>/fd/<fd>
    let read_fd = read_orig
        .move_to(fd_read_target)
        .or_fs_abort("move read end", c"pipe");
    let write_fd = write_orig
        .move_to(fd_write_target)
        .or_fs_abort("move write end", c"pipe");

    (read_fd, write_fd)
}
