pub use crate::syscall::{
    LINUX_REBOOT_CMD_HALT, LINUX_REBOOT_CMD_POWER_OFF, LINUX_REBOOT_CMD_RESTART,
    LINUX_REBOOT_MAGIC1, LINUX_REBOOT_MAGIC2,
};
use core::ptr;
use syscalls::Errno;

/// Halt the system
///
/// Requires CAP_SYS_BOOT capability.
/// This function does not return on success.
pub fn halt() -> Result<(), Errno> {
    unsafe {
        crate::syscall::reboot(
            LINUX_REBOOT_MAGIC1,
            LINUX_REBOOT_MAGIC2,
            LINUX_REBOOT_CMD_HALT,
            ptr::null(),
        )
    }
}

/// Power off the system
///
/// Requires CAP_SYS_BOOT capability.
/// This function does not return on success.
pub fn shutdown() -> Result<(), Errno> {
    unsafe {
        crate::syscall::reboot(
            LINUX_REBOOT_MAGIC1,
            LINUX_REBOOT_MAGIC2,
            LINUX_REBOOT_CMD_POWER_OFF,
            ptr::null(),
        )
    }
}

/// Reboot the system
///
/// Requires CAP_SYS_BOOT capability.
/// This function does not return on success.
pub fn reboot() -> Result<(), Errno> {
    unsafe {
        crate::syscall::reboot(
            LINUX_REBOOT_MAGIC1,
            LINUX_REBOOT_MAGIC2,
            LINUX_REBOOT_CMD_RESTART,
            ptr::null(),
        )
    }
}
