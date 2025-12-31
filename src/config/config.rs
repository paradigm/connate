//! Connate configuration file
//!
//! - Read ./src/config/config_api.rs
//! - Read ./src/config/example*.rs
//! - Implement Config for Connate

use crate::config::config_api::*;

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

    const SERVICES: &[Service] = &[];
}
