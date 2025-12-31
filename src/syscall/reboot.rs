use syscalls::{Errno, Sysno, syscall};

// Linux reboot magic numbers from include/uapi/linux/reboot.h
pub const LINUX_REBOOT_MAGIC1: i32 = 0xfee1dead_u32 as i32;
pub const LINUX_REBOOT_MAGIC2: i32 = 0x28121969;

// Linux reboot commands
pub const LINUX_REBOOT_CMD_POWER_OFF: i32 = 0x4321FEDC_u32 as i32;
pub const LINUX_REBOOT_CMD_RESTART: i32 = 0x01234567;
pub const LINUX_REBOOT_CMD_HALT: i32 = 0xCDEF0123_u32 as i32;

// `man 2 reboot`:
//
// SYNOPSIS
//        int reboot(int magic, int magic2, int cmd, void *arg);
//
// DESCRIPTION
//        The reboot() call reboots the system, or enables/disables the
//        reboot keystroke (abbreviated CAD, since the default is Ctrl-Alt-Delete).
//
//        The magic values are required to prevent accidental reboot calls.
//        The cmd argument controls the operation.
//
// RETURN VALUE
//        For the values of cmd that stop or restart the system, a successful
//        call to reboot() does not return. For the other cmd values, zero is
//        returned on success. In all cases, -1 is returned on failure, and
//        errno is set to indicate the error.
//
// ERRORS
//        EFAULT Problem with getting user-space data under LINUX_REBOOT_CMD_SW_SUSPEND.
//        EINVAL Bad magic numbers or cmd.
//        EPERM  The calling process has insufficient privilege to call reboot();
//               the CAP_SYS_BOOT capability is required.
//
pub unsafe fn reboot(magic: i32, magic2: i32, cmd: i32, arg: *const u8) -> Result<(), Errno> {
    match syscall!(Sysno::reboot, magic, magic2, cmd, arg) {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}
