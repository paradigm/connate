use crate::config::config_api::*;
use crate::config::helpers::*;
use crate::err::Errno;
use crate::os::*;
use crate::types::*;
use crate::{exec, run};

/// Example connate configuration file
impl Config for Connate {
    const LOCK_FILE: Option<&'static str> = None;

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
        max_setup_time: Some(core::time::Duration::from_secs(30)),
        max_ready_time: Some(core::time::Duration::from_secs(10)),
        max_stop_time: Some(core::time::Duration::from_secs(10)),
        max_cleanup_time: Some(core::time::Duration::from_secs(10)),
        retry: Retry::AfterDoublingDelay {
            initial_delay: core::time::Duration::from_secs(1),
            max_attempt_count: Some(5),
        },
        // Execution attribute entries
        log: Log::Inherit,
        env: &["PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"],
        user: None,
        group: None,
        chdir: None,
        no_new_privs: true,
    };

    const SERVICES: &[Service] = &[
        // =============================================================================
        // System - manages system lifecycle
        // =============================================================================
        //
        // - Everything should (directly or indirectly) depend on this service so that all other
        // services go down when this one goes down.
        // - This service's `cleanup` checks its own target when stopping:
        //   - If this stops with target=Down, the system shuts down.
        //   - If this stops with target=Reboot, the system reboots.
        Service {
            name: "system",
            cleanup: Run::Fn(|| {
                let _ = step("Syncing filesystems", || sync());

                let _ = step("Remounting root read-only", || {
                    let flags = MountFlags::MS_REMOUNT | MountFlags::MS_RDONLY;
                    mount(None, c"/", None, flags, None)?;
                    sync()
                });

                match get_service_target("system") {
                    Some(Target::Down) => step("Shutting down system", || shutdown()),
                    _ => step("Rebooting system", || reboot()),
                }?;
                print_color(Color::Error, "This should be unreachable!");
                Err(Errno::ERESTART)
            }),
            ..Self::DEFAULT_SERVICE
        },
        // =============================================================================
        // Early setup
        // =============================================================================
        //
        // Setup and services which are needed very early during boot, before typical services run.
        Service {
            name: "pseudofs",
            wants: &["system"],
            setup: Run::Fn(|| {
                step("Mount pseudo-filesystems", || {
                    use MountFlags as F;
                    let noexec = F::MS_NOSUID | F::MS_NOEXEC | F::MS_NODEV;
                    let tmpfs = F::MS_NOSUID | F::MS_NODEV;

                    mount_or_busy(c"/proc", c"proc", noexec, None)?;

                    mount_or_busy(c"/sys", c"sysfs", noexec, None)?;
                    mount_or_busy(c"/sys/kernel/security", c"securityfs", noexec, None)?;
                    mkdir_mode(c"/sys/fs/cgroup", 0o755)?;
                    mount_or_busy(c"/sys/fs/cgroup", c"cgroup2", noexec, Some(c"nsdelegate"))?;
                    if is_dir(c"/sys/firmware/efi/efivars").unwrap_or(false) {
                        mount_or_busy(c"/sys/firmware/efi/efivars", c"efivarfs", noexec, None)?;
                    }

                    mount_or_busy(c"/dev", c"devtmpfs", F::MS_NOSUID, Some(c"mode=0755"))?;
                    mkdir_mode(c"/dev/pts", 0o755)?;
                    mount_or_busy(
                        c"/dev/pts",
                        c"devpts",
                        F::MS_NOSUID | F::MS_NOEXEC,
                        Some(c"gid=5,mode=0620"),
                    )?;
                    mkdir_mode(c"/dev/shm", 0o1777)?;
                    mount_or_busy(c"/dev/shm", c"tmpfs", tmpfs, Some(c"mode=1777"))?;

                    mount_or_busy(c"/run", c"tmpfs", tmpfs, Some(c"mode=0755"))?;
                    mkdir_mode(c"/run/lock", 0o1777)?;
                    mkdir_mode(c"/run/user", 0o755)?;

                    Ok(())
                })
            }),
            // Cleanup not needed. these aren't backed by disk real and don't need to be synced or
            // unmounted at shutdown.
            ..Self::DEFAULT_SERVICE
        },
        Service {
            name: "modules",
            wants: &["pseudofs"],
            // systemd distros (and Gentoo Linux which uses systemd auxiliary binaries):
            setup: Run::Exec(&["/usr/lib/systemd/systemd-modules-load"]),
            // // Void Linux:
            // setup: Run::Exec(&["/usr/bin/modules-load"]),
            // // Alpine Linux:
            // setup: Run::Shell(". /etc/init.d/modules && start"),
            ..Self::DEFAULT_SERVICE
        },
        // Device manager
        //
        // On systemd distros (and Gentoo) use these paths:
        // - /usr/lib/systemd/systemd-udevd
        // - /usr/bin/udevadm
        //
        // On Void Linux:
        // - /usr/bin/udevd
        // - /usr/bin/udevadm
        //
        // On Alpine, use "mdev" service instead.
        Service {
            name: "udev",
            wants: &["modules"],
            run: Run::Fn(|| {
                // We need to send coldplug instructions after launching udevd.
                // Rather than making this a separate service, leverage Ready::Daemonize to:
                // - Run udevd as a daemon
                // - Run the coldplug commands
                // - Exit the main process
                // - Let connate find the daemonized udevd
                run!(["/usr/lib/systemd/systemd-udevd", "--daemon"])?;
                run!([
                    "/usr/bin/udevadm", // /usr/bin/udevdadm on Void Linux
                    "trigger",
                    "--type=subsystems",
                    "--action=add",
                ])?;
                exec!([
                    "/usr/bin/udevadm", // /usr/bin/udevdadm on Void Linux
                    "trigger",
                    "--type=devices",
                    "--action=add",
                ])
            }),
            ready: Ready::Daemonize,
            ..Self::DEFAULT_SERVICE
        },
        // On Alpine, use this instead of "udev" service.
        //
        // Rather than having udev constantly running in the background, this runs a transient
        // `mdev` instance
        // Service {
        //     name: "mdev",
        //     wants: &["load-modules"],
        //     setup: Run::Fn(|| {
        //         write_file(c"/proc/sys/kernel/hotplug", "/sbin/mdev")?;
        //         exec!(["/sbin/mdev", "-s"])
        //     }),
        //     ..Self::DEFAULT_SERVICE
        // },
        Service {
            name: "filesystems",
            wants: &["udev"],
            setup: Run::Fn(|| {
                step("lvm", || run!(["/sbin/vgscan", "--mknodes"]))?;

                step("fsck", || {
                    let flags = MountFlags::MS_REMOUNT | MountFlags::MS_RDONLY;
                    mount(None, c"/", None, flags, None)?;

                    // fsck exit code 1 indicates errors corrected; not error condition
                    match run!(["/usr/sbin/fsck", "-ATat", "noopts=_netdev"]) {
                        Ok(()) => Ok(()),
                        Err(e) if e.into_raw() == 1 => {
                            print_color(Color::Warning, "[W] ");
                            print("fsck corrected filesystem errors\n");
                            Ok(())
                        }
                        Err(e) => Err(e),
                    }?;

                    mount(None, c"/", None, MountFlags::MS_REMOUNT, None)
                })?;

                step("Mount /etc/fstab", || {
                    run!(["/bin/mount", "-at", "noopts=_netdev"])
                })?;

                step("Enabling swap", || run!(["/usr/bin/swapon", "-a"]))
            }),
            cleanup: Run::Fn(|| step("Disabling swap", || run!(["/usr/bin/swapoff", "-a"]))),
            ..Self::DEFAULT_SERVICE
        },
        // =============================================================================
        // System initialization
        // =============================================================================
        Service {
            name: "sysctl",
            wants: &["filesystems"],
            setup: Run::Exec(&["/sbin/sysctl", "--system"]),
            ..Self::DEFAULT_SERVICE
        },
        Service {
            name: "early-files",
            wants: &["filesystems"],
            setup: Run::Fn(|| {
                // We want to create these files early, before potentially malicious users have
                // access to the system which could race things like symlink attacks.
                //
                // Login accounting files
                touch_file(c"/run/utmp", 0o664)?;
                mkdir_mode(c"/var/log", 0o755)?;
                touch_file(c"/var/log/wtmp", 0o664)?;
                touch_file(c"/var/log/btmp", 0o600)?;
                touch_file(c"/var/log/lastlog", 0o644)?;
                // Ensure temp and var directories exist with correct permissions
                mkdir_mode(c"/tmp", 0o1777)?;
                mkdir_mode(c"/var/tmp", 0o1777)?;
                // X11 socket directories (sticky bit prevents users deleting each other's sockets)
                mkdir_mode(c"/tmp/.X11-unix", 0o1777)?;
                mkdir_mode(c"/tmp/.ICE-unix", 0o1777)
            }),
            ..Self::DEFAULT_SERVICE
        },
        Service {
            name: "hostname",
            wants: &["filesystems"],
            setup: Run::Fn(|| copy(c"/etc/hostname", c"/proc/sys/kernel/hostname")),
            ..Self::DEFAULT_SERVICE
        },
        Service {
            name: "hwclock",
            wants: &["filesystems"],
            // --utc: hardware clock stores UTC (Linux default, avoids DST issues)
            // --localtime: hardware clock stores local time (for dual-boot with Windows)
            setup: Run::Exec(&["/sbin/hwclock", "--hctosys", "--utc"]),
            cleanup: Run::Exec(&["/sbin/hwclock", "--systohc", "--utc"]),
            ..Self::DEFAULT_SERVICE
        },
        Service {
            name: "random-seed",
            wants: &["filesystems"],
            setup: Run::Fn(random_seed_load),
            cleanup: Run::Fn(random_seed_save),
            ..Self::DEFAULT_SERVICE
        },
        Service {
            name: "loopback",
            wants: &["pseudofs"],
            setup: Run::Exec(&["/usr/bin/ip", "link", "set", "lo", "up"]),
            ..Self::DEFAULT_SERVICE
        },
        Service {
            name: "dmesg",
            wants: &["filesystems"],
            setup: Run::Fn(|| {
                // Capture kernel ring buffer to file for debugging
                // Run early so boot messages are preserved before buffer wraps
                match fork()? {
                    ForkResult::Child => {
                        let fd = Fd::open(
                            c"/var/log/dmesg",
                            OpenFlags::O_WRONLY | OpenFlags::O_CREAT | OpenFlags::O_TRUNC,
                            0o644,
                        )?;
                        let _ = fd.move_to(1); // redirect stdout
                        exec!(["/bin/dmesg"])?;
                        exit(127);
                    }
                    ForkResult::Parent(pid) => {
                        let (_, status) = waitpid(pid, WaitPidOptions::empty())?;
                        if wifexited(status) {
                            Errno::from_ret(wexitstatus(status) as usize).map(|_| ())
                        } else {
                            Errno::from_ret(128 + wtermsig(status) as usize).map(|_| ())
                        }
                    }
                }
            }),
            ..Self::DEFAULT_SERVICE
        },
        // =============================================================================
        // Network
        // =============================================================================
        // Uncomment ONE network service for your distro:
        //
        // // NetworkManager
        // Service {
        //     name: "network",
        //     needs: &["loopback", "udev"],
        //     run: Run::Exec(&["/usr/bin/NetworkManager", "--no-daemon"]),
        //     log: Log::File {
        //         path: "/var/log/network.log",
        //         mode: FileMode::Overwrite,
        //         permissions: FilePerm::Public,
        //     },
        //     ..Self::DEFAULT_SERVICE
        // },
        //
        // // systemd-networkd
        // Service {
        //     name: "network",
        //     needs: &["loopback", "udev"],
        //     run: Run::Exec(&["/usr/lib/systemd/systemd-networkd"]),
        //     log: Log::File {
        //         path: "/var/log/network.log",
        //         mode: FileMode::Overwrite,
        //         permissions: FilePerm::Public,
        //     },
        //     ..Self::DEFAULT_SERVICE
        // },
        //
        // // Static configuration (edit IP/gateway/interface as needed)
        // Service {
        //     name: "network",
        //     needs: &["loopback", "udev"],
        //     start: Start::Fn(|| {
        //         run!(["/sbin/ip", "addr", "add", "192.168.1.100/24", "dev", "eth0"])?;
        //         run!(["/sbin/ip", "link", "set", "eth0", "up"])?;
        //         run!(["/sbin/ip", "route", "add", "default", "via", "192.168.1.1"])
        //     }),
        //     log: Log::File {
        //         path: "/var/log/network.log",
        //         mode: FileMode::Overwrite,
        //         permissions: FilePerm::Public,
        //     },
        //     ..Self::DEFAULT_SERVICE
        // },
        //
        // // dhcpcd (Arch, Void, Alpine, Gentoo)
        Service {
            name: "network",
            needs: &["loopback", "udev", "early-files"],
            run: Run::Exec(&["/usr/bin/dhcpcd", "--nobackground"]),
            log: Log::File {
                path: "/var/log/network.log",
                mode: FileMode::Overwrite,
                permissions: FilePerm::Public,
            },
            ..Self::DEFAULT_SERVICE
        },
        // =============================================================================
        // Login terminals
        // =============================================================================
        Service {
            name: "agetty-tty1",
            wants: &["early-files"],
            run: Run::Exec(&["/sbin/agetty", "--noclear", "tty1", "38400", "linux"]),
            no_new_privs: false,
            ..Self::DEFAULT_SERVICE
        },
        Service {
            name: "agetty-tty2",
            wants: &["early-files"],
            run: Run::Exec(&["/sbin/agetty", "--noclear", "tty2", "38400", "linux"]),
            no_new_privs: false,
            ..Self::DEFAULT_SERVICE
        },
        Service {
            name: "agetty-tty3",
            wants: &["early-files"],
            run: Run::Exec(&["/sbin/agetty", "--noclear", "tty3", "38400", "linux"]),
            no_new_privs: false,
            ..Self::DEFAULT_SERVICE
        },
        Service {
            name: "agetty-tty4",
            wants: &["early-files"],
            run: Run::Exec(&["/sbin/agetty", "--noclear", "tty4", "38400", "linux"]),
            no_new_privs: false,
            ..Self::DEFAULT_SERVICE
        },
        Service {
            name: "agetty-tty5",
            wants: &["early-files"],
            run: Run::Exec(&["/sbin/agetty", "--noclear", "tty5", "38400", "linux"]),
            no_new_privs: false,
            ..Self::DEFAULT_SERVICE
        },
        Service {
            name: "agetty-tty6",
            wants: &["early-files"],
            run: Run::Exec(&["/sbin/agetty", "--noclear", "tty6", "38400", "linux"]),
            no_new_privs: false,
            ..Self::DEFAULT_SERVICE
        },
        // =============================================================================
        // Daemons
        // =============================================================================
        Service {
            name: "dbus",
            wants: &["early-files"],
            setup: Run::Fn(|| mkdir_mode(c"/run/dbus", 0o755)),
            run: Run::Exec(&[
                "/usr/bin/dbus-daemon",
                "--system",
                "--nofork",
                "--nopidfile",
            ]),
            log: Log::File {
                path: "/var/log/dbus.log",
                mode: FileMode::Overwrite,
                permissions: FilePerm::Public,
            },
            ..Self::DEFAULT_SERVICE
        },
        Service {
            name: "chrony",
            wants: &["network"],
            run: Run::Exec(&[
                "/usr/bin/chronyd",
                "-d", // foreground mode
            ]),
            log: Log::File {
                path: "/var/log/chrony.log",
                mode: FileMode::Overwrite,
                permissions: FilePerm::Public,
            },
            ..Self::DEFAULT_SERVICE
        },
        Service {
            name: "sshd",
            wants: &["network"],
            run: Run::Exec(&[
                "/usr/sbin/sshd",
                "-D", // foreground mode
            ]),
            log: Log::File {
                path: "/var/log/sshd.log",
                mode: FileMode::Overwrite,
                permissions: FilePerm::Private,
            },
            no_new_privs: false,
            ..Self::DEFAULT_SERVICE
        },
        Service {
            name: "cupsd",
            init_target: Target::Down,
            wants: &["dbus"],
            run: Run::Exec(&[
                "/usr/bin/cupsd",
                "-f", // foreground mode
            ]),
            log: Log::File {
                path: "/var/log/cupsd.log",
                mode: FileMode::Overwrite,
                permissions: FilePerm::Private,
            },
            ..Self::DEFAULT_SERVICE
        },
        Service {
            name: "crond",
            wants: &["early-files"],
            run: Run::Exec(&[
                "/usr/bin/crond",
                "-n", // foreground
            ]),
            log: Log::File {
                path: "/var/log/crond.log",
                mode: FileMode::Overwrite,
                permissions: FilePerm::Private,
            },
            no_new_privs: false,
            ..Self::DEFAULT_SERVICE
        },
    ];
}

// =============================================================================
// Helper functions
// =============================================================================

/// Mounts filesystem if it is not unmounted.
/// If busy, assumes already mounted and silently continues.
fn mount_or_busy(
    target: &CStr,
    fstype: &CStr,
    flags: MountFlags,
    opts: Option<&CStr>,
) -> Result<(), Errno> {
    match mount(None, target, Some(fstype), flags, opts) {
        Ok(()) | Err(Errno::EBUSY) => Ok(()),
        Err(e) => Err(e),
    }
}

// =============================================================================
// Service implementation functions
// =============================================================================

const RANDOM_SEED_PATH: &CStr = c"/var/lib/misc/random-seed";
const RANDOM_SEED_SIZE: usize = 512;

fn random_seed_load() -> Result<(), Errno> {
    // Load saved entropy into the kernel random pool
    let mut buf = [0u8; RANDOM_SEED_SIZE];
    let n = read_file(RANDOM_SEED_PATH, &mut buf).unwrap_or(0);

    if n == 0 {
        return Ok(()); // No seed file is acceptable on first boot
    }

    // Write seed to kernel entropy pool
    if let Some(data) = buf.get(..n) {
        let _ = write_file(c"/dev/urandom", data);
    }

    // Overwrite seed file to prevent reuse
    let _ = random_seed_save();

    Ok(())
}

fn random_seed_save() -> Result<(), Errno> {
    // Ensure parent directory exists (rwxr-xr-x)
    mkdir_mode(c"/var/lib/misc", 0o755)?;

    // Read fresh entropy from kernel
    let mut buf = [0u8; RANDOM_SEED_SIZE];
    let n = read_file(c"/dev/urandom", &mut buf)?;

    // Save to seed file (rw------- for security)
    let fd = Fd::open(
        RANDOM_SEED_PATH,
        OpenFlags::O_WRONLY | OpenFlags::O_CREAT | OpenFlags::O_TRUNC,
        0o600,
    )?;

    if let Some(data) = buf.get(..n) {
        let _ = fd.write(data);
    }
    let _ = fd.close();

    Ok(())
}
