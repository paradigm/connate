use connate::err::*;
use connate::ipc::*;
use connate::os::*;
use connate::types::*;
use connate::util::*;
use itoa::Integer; // ::MAX_STR_LEN

pub fn cmd_ready(mut ipc_client: IpcClient, connate_pid: pid_t) -> ! {
    // Walk up process tree to find connate's direct child
    let child_pid = find_connate_child(connate_pid).or_abort(
        "Unable to find connate in process ancestry.  Is this being called from a service?",
    );

    // Send ready notification
    let response = ipc_client.send_and_receive(Request::ServiceReady(child_pid));

    // Check result and print
    let failed = response.cmd_return_failed();
    println(response);

    exit(if failed { 1 } else { 0 });
}

/// Find connate's direct child by walking up the process tree
///
/// Starting from conctl's parent, read /proc/<pid>/stat to get PPID,
/// continuing until we find a process whose PPID is connate_pid.
pub fn find_connate_child(connate_pid: pid_t) -> Result<pid_t, Errno> {
    let mut current_pid = getppid();

    loop {
        let ppid = read_proc_stat_ppid(current_pid)?;

        if ppid == connate_pid {
            // Found it - current_pid is connate's direct child
            return Ok(current_pid);
        }

        if ppid == 0 || ppid == 1 {
            // Reached init or kernel, connate is not an ancestor
            return Err(Errno::ENOENT);
        }

        current_pid = ppid;
    }
}

/// Read PPID from /proc/<pid>/stat
pub fn read_proc_stat_ppid(pid: pid_t) -> Result<pid_t, Errno> {
    const PATH_SIZE: usize = b"/proc/".len() + pid_t::MAX_STR_LEN + b"/stat\0".len();

    let mut path_buf = [0u8; PATH_SIZE];
    let mut pid_buf = itoa::Buffer::new();
    let pid_str = pid_buf.format(pid).as_bytes();

    // Build path: /proc/<pid>/stat
    let mut writer = BufWriter::new(&mut path_buf);
    writer.push(b"/proc/")?;
    writer.push(pid_str)?;
    writer.push(b"/stat\0")?;
    // Safety: writer ensures we only expose initialized bytes ending with '\0'
    let path = unsafe { CStr::from_bytes_with_nul_unchecked(writer.as_slice()) };

    let fd = Fd::open(path, OpenFlags::O_RDONLY, 0)?;

    const STAT_BUF_SIZE: usize = //
        pid_t::MAX_STR_LEN // pid field
        + 1  // space: one byte
        + 1  // '(': one byte
        + 16 // (comm) field: TASK_COMM_LEN which is 16 bytes including terminating null byte
        + 1  // ')': one byte
        + 1  // space: one byte
        + 1  // state field: one byte
        + 1  // space: one byte
        + pid_t::MAX_STR_LEN // ppid field
        + 1; // space: one byte (to mark end of ppid)
    let mut stat_buf = [0u8; STAT_BUF_SIZE];
    let n = fd.read(&mut stat_buf)?;
    fd.close()?;
    let stat_data = stat_buf.get(..n).ok_or(Errno::EINVAL)?;

    // Parse PPID from stat data
    parse_stat_ppid(stat_data)
}

/// Parse PPID from /proc/\<pid\>/stat
///
/// File format is:
///
/// ```text
/// pid (comm) state ppid [...]
/// ```
///
/// The comm field can contain almost anything, including both `)` and whitespace.  However, it is
/// followed by the last `)` in the file; no following fields may contain a `)`.  Thus, we search
/// for the last `)`, skip the following state field, then read out ppid.
pub fn parse_stat_ppid(data: &[u8]) -> Result<pid_t, Errno> {
    // Find last ')'  - this marks the end of comm field
    //
    // We know it's followed by a space, and so continue from one character beyond that.
    let pos = data.iter().rposition(|&b| b == b')').ok_or(Errno::EINVAL)?;
    let data = data.get(pos + 2..).ok_or(Errno::EINVAL)?;

    // Find next space.  This marks the end of state field.
    let pos = data.iter().position(|&b| b == b' ').ok_or(Errno::EINVAL)?;
    let data = data.get(pos + 1..).ok_or(Errno::EINVAL)?;

    // Find next space. This marks end of pid field.
    let end = data.iter().position(|&b| b == b' ').ok_or(Errno::EINVAL)?;
    let pid_bytes = data.get(..end).ok_or(Errno::EINVAL)?;

    pid_bytes.parse_pid()
}
