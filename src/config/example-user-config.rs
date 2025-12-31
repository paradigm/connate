use crate::config::config_api::*;

/// Example connate configuration file
impl Config for Connate {
    const LOCK_FILE: Option<&'static str> = Some("/run/user/1000/connate-lock");

    const DEFAULT_SERVICE: Service = Service {
        name: "unspecified-service-name",
        init_target: Target::Down,
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
        retry: Retry::Never,
        // Execution attribute entries
        log: Log::Inherit,
        env: &[
            "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
            "XDG_RUNTIME_DIR=/run/user/1000",
            "GPG_AGENT_INFO=/run/user/1000/gnupg/S.gpg-agent::1",
            "PULSE_SERVER=unix:/run/user/1000/pulse/native",
            "MPD_HOST=/run/user/1000/S.mpd",
            "CONNATE_LOCK_FILE=/run/user/1000/connate-lock",
            "DISPLAY=:0",
        ],
        user: None,
        group: None,
        chdir: None,
        no_new_privs: false,
    };

    const SERVICES: &[Service] = &[
        // =======
        // Session
        // =======
        Service {
            name: "session",
            init_target: Target::Up,
            groups: &["gpg-agent", "ssh-agent", "dbus"],
            ..Self::DEFAULT_SERVICE
        },
        Service {
            name: "gpg-agent",
            init_target: Target::Up,
            run: Run::Exec(&["/usr/bin/gpg-agent", "--daemon", "--verbose"]),
            // gpg-agent supports a --supervised mode which expects sockets specifically with
            // systemd in mind.
            //
            // The alternative requires `--daemon`ization
            ready: Ready::Daemonize,
            log: Log::File {
                path: "/run/user/1000/log/gpg-agent.log",
                mode: FileMode::Overwrite,
                permissions: FilePerm::Private,
            },
            // pinentry sometimes locks up
            stop_all_children: true,
            ..Self::DEFAULT_SERVICE
        },
        Service {
            name: "ssh-agent",
            init_target: Target::Up,
            // Ensure no lingering socket
            setup: Run::Exec(&["/bin/rm", "-f", "/run/user/1000/S.ssh-agent"]),
            run: Run::Exec(&[
                "/usr/bin/ssh-agent",
                "-D", // foreground
                "-a",
                "/run/user/1000/S.ssh-agent",
            ]),
            log: Log::File {
                path: "/run/user/1000/log/ssh-agent.log",
                mode: FileMode::Overwrite,
                permissions: FilePerm::Private,
            },
            ..Self::DEFAULT_SERVICE
        },
        Service {
            name: "dbus",
            init_target: Target::Up,
            run: Run::Exec(&[
                "/usr/bin/dbus-daemon",
                "--nofork",
                "--session",
                "--address",
                "unix:path=/run/user/1000/S.dbus",
            ]),
            log: Log::File {
                path: "/run/user/1000/log/dbus.log",
                mode: FileMode::Overwrite,
                permissions: FilePerm::Private,
            },
            ..Self::DEFAULT_SERVICE
        },
        // ===
        // GUI
        // ===
        Service {
            name: "xorg",
            groups: &["dwm", "dwmstatus", "xcape", "dunst"],
            // Requires `Xwrapper.config` to have `allowed_users=anybody`
            run: Run::Exec(&["/usr/bin/xinit"]),
            ready: Ready::Notify, // put `conctl ready` in .xinitrc
            log: Log::File {
                path: "/run/user/1000/log/xorg.log",
                mode: FileMode::Overwrite,
                permissions: FilePerm::Private,
            },
            env: &[
                "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
                "XDG_RUNTIME_DIR=/run/user/1000",
                "GPG_AGENT_INFO=/run/user/1000/gnupg/S.gpg-agent::1",
                "PULSE_SERVER=unix:/run/user/1000/pulse/native",
                "MPD_HOST=/run/user/1000/S.mpd",
                "CONNATE_LOCK_FILE=/run/user/1000/connate-lock",
                // "DISPLAY=:0", // Let it set its own
            ],
            ..Self::DEFAULT_SERVICE
        },
        Service {
            name: "dwm",
            needs: &["xorg"],
            groups: &["dwmstatus"],
            run: Run::Exec(&["/usr/bin/dwm"]),
            ..Self::DEFAULT_SERVICE
        },
        Service {
            name: "dwmstatus",
            needs: &["xorg", "dwm"],
            run: Run::Exec(&["/usr/bin/dwmstatus"]),
            ..Self::DEFAULT_SERVICE
        },
        Service {
            name: "xcape",
            needs: &["xorg"],
            run: Run::Exec(&["/usr/bin/xcape", "-d"]),
            ..Self::DEFAULT_SERVICE
        },
        Service {
            name: "dunst",
            needs: &["xorg"],
            run: Run::Exec(&["/usr/bin/dunst"]),
            log: Log::File {
                path: "/run/user/1000/log/dunst.log",
                mode: FileMode::Overwrite,
                permissions: FilePerm::Private,
            },
            ..Self::DEFAULT_SERVICE
        },
        // =====
        // Audio
        // =====
        Service {
            name: "mpd",
            conflicts: &["moc"],
            run: Run::Exec(&["/usr/bin/mpd", "--no-daemon", "--verbose"]),
            log: Log::File {
                path: "/run/user/1000/log/mpd.log",
                mode: FileMode::Overwrite,
                permissions: FilePerm::Private,
            },
            ..Self::DEFAULT_SERVICE
        },
        Service {
            name: "moc",
            conflicts: &["mpd"],
            run: Run::Exec(&["/usr/bin/mocp", "--foreground", "--server"]),
            log: Log::File {
                path: "/run/user/1000/log/moc.log",
                mode: FileMode::Overwrite,
                permissions: FilePerm::Private,
            },
            ..Self::DEFAULT_SERVICE
        },
    ];
}
