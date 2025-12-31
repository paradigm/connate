use crate::build_util::*;
use std::collections::{HashMap, HashSet};
use std::ffi::CString;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

#[cfg(not(test))]
use crate::config::*;
#[cfg(test)]
use connate::config::*;

#[cfg(not(test))]
use crate::constants::*;
#[cfg(test)]
use connate::constants::*;

/// Trait providing compile-time configuration checking methods.
pub trait ConfigCheck: Config {
    fn check_config() {
        let uid_map = get_uid_map();
        let gid_map = get_gid_map();
        let svc_map = get_svc_map(Self::SERVICES);

        // Checks for things that aren't specific to one services
        Self::check_lock_file();
        Self::check_name_uniqueness();
        Self::check_name_default();
        Self::check_log_uniqueness();

        if Self::SERVICES.is_empty() {
            panic!("No services configured.");
        }

        // Per-service checks
        for svc in Self::SERVICES {
            svc.check_name();
            // svc.check_init_target(); // type system check is comprehensive
            svc.check_needs(&svc_map);
            svc.check_wants(&svc_map);
            svc.check_conflicts(&svc_map);
            svc.check_groups(&svc_map);
            svc.check_setup();
            svc.check_run();
            svc.check_ready();
            svc.check_cleanup();
            // svc.check_stop_all_children(); // type system check is comprehensive
            svc.check_max_setup_time();
            svc.check_max_ready_time();
            svc.check_max_stop_time();
            svc.check_max_cleanup_time();
            svc.check_retry();
            svc.check_log(&svc_map);
            svc.check_env();
            svc.check_user(
                #[cfg(feature = "host-checks")]
                &uid_map,
            );
            svc.check_group(
                #[cfg(feature = "host-checks")]
                &gid_map,
            );
            svc.check_chdir();
            // svc.check_no_new_privs(); // type system check is comprehensive
        }

        // Graph traversals for cycle detection
        // This must follow prior checks ensuring dependencies exist at all
        for svc in Self::SERVICES {
            svc.check_start_stop_cycle(&svc_map);
            svc.check_target_up_stable(&svc_map);
        }
    }

    fn check_lock_file() {
        let Some(path) = Self::LOCK_FILE else {
            return;
        };

        // Check for null bytes
        if path.contains('\0') {
            panic!("The configured LOCK_FILE '{path}' contains a disallowed null byte");
        }

        // Check that path is absolute
        let path_obj = Path::new(path);
        if !path_obj.is_absolute() {
            panic!(
                "The configured LOCK_FILE '{path}' is not absolute. Only absolute paths are allowed."
            );
        }

        #[cfg(feature = "host-checks")]
        {
            // Check that file exists
            if !path_obj.exists() {
                panic!(
                    "The configured LOCK_FILE '{path}' does not exist.
                        If you are building for a remote system, build with --no-default-features.
                        Otherwise, either create the file or change the path."
                );
            }

            // Check that it's a file, not a directory
            if !path_obj.is_file() {
                panic!(
                    "The configured LOCK_FILE '{path}' exists but is not a file (it may be a directory)"
                );
            }
        }
    }

    fn check_name_uniqueness() {
        let mut names = HashSet::new();

        for svc in Self::SERVICES {
            if !names.insert(svc.name) {
                panic!("Service name '{}' is not unique", svc.name);
            }
        }
    }

    fn check_name_default() {
        for svc in Self::SERVICES {
            if svc.name == Self::DEFAULT_SERVICE.name {
                panic!(
                    "At least one service inherited default name ('{}'), which was probably unintentional.",
                    Self::DEFAULT_SERVICE.name
                );
            }
        }
    }

    fn check_log_uniqueness() {
        let mut log_files: HashSet<&str> = HashSet::new();

        for svc in Self::SERVICES {
            if let Log::File { path, .. } = &svc.log
                && !log_files.insert(*path)
            {
                panic!(
                    "Multiple services are configured to log to the same file: '{}'",
                    path,
                );
            }
        }
    }
}

impl Service {
    // ===================
    // Direct field checks
    // ===================

    fn check_name(&self) {
        if self.name.len() > MSG_SVC_NAME_SIZE {
            panic!(
                "Service name '{}' has more bytes than max allowed of {}",
                self.name, MSG_SVC_NAME_SIZE
            );
        }
        if self.name.chars().any(|c| c.is_ascii_whitespace()) {
            panic!(
                "Service name '{}' contains a disallowed whitespace character",
                self.name
            );
        }
        if self.name.is_empty() {
            panic!("A service has a disallowed empty name");
        }
        if let Err(e) = CString::from_str(self.name) {
            panic!(
                "Service name '{}' cannot be converted into a C string: {}",
                self.name, e
            );
        }
    }

    fn check_needs(&self, svc_map: &HashMap<&'static str, &'static Service>) {
        self.check_dependency(self.needs, "needs", svc_map);
    }

    fn check_wants(&self, svc_map: &HashMap<&'static str, &'static Service>) {
        self.check_dependency(self.wants, "wants", svc_map);

        for want in self.wants {
            if self.needs.contains(want) {
                panic!(
                    "Service '{}' both needs and wants '{}', in which case the wants dependency does nothing.  This is probably an unintentional configuration.",
                    self.name, want
                )
            }
        }
    }

    fn check_conflicts(&self, svc_map: &HashMap<&'static str, &'static Service>) {
        self.check_dependency(self.conflicts, "conflicts", svc_map);
    }

    fn check_groups(&self, svc_map: &HashMap<&'static str, &'static Service>) {
        self.check_dependency(self.groups, "groups", svc_map);
    }

    fn check_setup(&self) {
        match self.setup {
            Run::None => {}
            Run::Exec(args) => self.check_exec_args(args, "setup"),
            Run::Shell(cmd) => self.check_shell_command(cmd, "setup"),
            Run::Fn(_) => {}
        }
    }

    fn check_run(&self) {
        match self.run {
            Run::None => {}
            Run::Exec(args) => self.check_exec_args(args, "run"),
            Run::Shell(cmd) => self.check_shell_command(cmd, "run"),
            Run::Fn(_) => {}
        }
    }

    fn check_cleanup(&self) {
        match self.cleanup {
            Run::None => {}
            Run::Exec(args) => self.check_exec_args(args, "cleanup"),
            Run::Shell(cmd) => self.check_shell_command(cmd, "cleanup"),
            Run::Fn(_) => {}
        }
    }

    fn check_ready(&self) {
        match (&self.run, &self.ready) {
            (Run::None, Ready::Notify) => panic!(
                "Service '{}' has ready: Ready::Notify but run: Run::None. \
                 Notify requires a running process to signal readiness.",
                self.name
            ),
            (Run::None, Ready::Daemonize) => panic!(
                "Service '{}' has ready: Ready::Daemonize but run: Run::None. \
                 Daemonize requires a running process to fork.",
                self.name
            ),
            _ => {}
        }
    }

    fn check_max_setup_time(&self) {
        self.check_duration(self.max_setup_time, "max_setup_time");
    }

    fn check_max_ready_time(&self) {
        self.check_duration(self.max_ready_time, "max_ready_time");
    }

    fn check_max_stop_time(&self) {
        self.check_duration(self.max_stop_time, "max_stop_time");
    }

    fn check_max_cleanup_time(&self) {
        self.check_duration(self.max_cleanup_time, "max_cleanup_time");
    }

    fn check_retry(&self) {
        match self.retry {
            Retry::Never => {}
            Retry::AfterFixed { after, .. } => self.check_duration(Some(after), "retry after"),
            Retry::AfterDoublingDelay { initial_delay, .. } => {
                self.check_duration(Some(initial_delay), "retry initial_delay")
            }
        };
    }

    fn check_log(&self, svc_map: &HashMap<&'static str, &'static Service>) {
        match &self.log {
            Log::None => {}
            Log::Inherit => {}
            Log::File { path, .. } => {
                if path.contains('\0') {
                    panic!(
                        "Service '{}' has log path '{}' which contains a disallowed null byte",
                        self.name, path
                    );
                }
                if path.len() > MSG_PATH_SIZE {
                    panic!(
                        "Service '{}' has log path '{}' with more bytes than max allowed of {}",
                        self.name, path, MSG_PATH_SIZE,
                    );
                }
                let path_obj = Path::new(path);
                if !path_obj.is_absolute() {
                    panic!(
                        "Service '{}' has log path '{}' which is not absolute. Only absolute paths are allowed.",
                        self.name, path
                    );
                }

                #[cfg(feature = "host-checks")]
                {
                    if path_obj.is_dir() {
                        panic!(
                            "Service '{}' has log file path '{}' which is a directory. Can only log to files.",
                            self.name, path
                        );
                    }
                    let Some(parent) = path_obj.parent() else {
                        panic!(
                            "Service '{}' has log file path '{}' which has no parent directory",
                            self.name, path
                        );
                    };
                    if !parent.exists() {
                        panic!(
                            "Service '{}' has log file path '{}' whose parent directory '{}' does not exist",
                            self.name,
                            path,
                            parent.display()
                        );
                    }
                    if !parent.is_dir() {
                        panic!(
                            "Service '{}' has log file path '{}' whose parent '{}' is not a directory",
                            self.name,
                            path,
                            parent.display()
                        );
                    }
                }
            }
            Log::Service(log_svc_name) => {
                // Check for self-logging
                if *log_svc_name == self.name {
                    panic!("Service '{}' cannot log to itself", self.name);
                }

                // Check that the log service exists
                if !svc_map.contains_key(log_svc_name) {
                    panic!(
                        "Service '{}' has log service '{}' which does not exist",
                        self.name, log_svc_name
                    );
                }

                // Check that this service doesn't conflict with its log service
                if self.conflicts.contains(log_svc_name) {
                    panic!(
                        "Service '{}' logs to service '{}' but also conflicts with it. This creates an impossible dependency.",
                        self.name, log_svc_name
                    );
                }

                // Check that the log service doesn't conflict with this service
                let log_svc = svc_map[log_svc_name];
                if log_svc.conflicts.contains(&self.name) {
                    panic!(
                        "Service '{}' logs to service '{}', but '{}' conflicts with '{}'. This creates an impossible dependency.",
                        self.name, log_svc_name, log_svc_name, self.name
                    );
                }

                // Check that the log service can accept stdin
                if matches!(log_svc.run, Run::None) {
                    panic!(
                        "Service '{}' logs to service '{}', but '{}' has run set to None and thus cannot accept stdin",
                        self.name, log_svc_name, log_svc_name
                    );
                }
            }
        }
    }

    fn check_env(&self) {
        let mut vars = HashSet::new();

        for var_eq_val in self.env {
            if var_eq_val.is_empty() {
                panic!("Service '{}' has an empty environment variable", self.name);
            }
            if var_eq_val.contains('\0') {
                panic!(
                    "Service '{}' has environment variable '{}' which contains a disallowed null byte",
                    self.name, var_eq_val
                );
            }

            let Some((var, _val)) = var_eq_val.split_once('=') else {
                panic!(
                    "Service '{}' has environment variable '{}' which lacks an equals sign ('=')",
                    self.name, var_eq_val
                );
            };

            if var.is_empty() {
                panic!(
                    "Service '{}' has environment variable '{}' with an empty name",
                    self.name, var
                );
            }

            // Check for duplicate variable names
            if !vars.insert(var) {
                panic!(
                    "Service '{}' has duplicate environment variable name '{}'",
                    self.name, var
                );
            }

            // Validate variable name follows POSIX rules
            // - Must start with letter or underscore
            // - Rest must be alphanumeric or underscore
            let first_char = var.chars().next().unwrap();
            if !first_char.is_ascii_alphabetic() && first_char != '_' {
                panic!(
                    "Service '{}' has environment variable '{}' with invalid name '{}'. Names must start with a letter or underscore.",
                    self.name, var, var
                );
            }

            for char in var.chars() {
                if !char.is_ascii_alphanumeric() && char != '_' {
                    panic!(
                        "Service '{}' has environment variable '{}' with invalid name '{}'. Names may only contain letters, digits, and underscores.",
                        self.name, var, var
                    );
                }
            }
        }
    }

    fn check_user(&self, #[cfg(feature = "host-checks")] uid_map: &HashMap<String, u32>) {
        let Some(user) = &self.user else {
            return;
        };

        #[cfg(not(feature = "host-checks"))]
        {
            panic!(
                "Service '{}' has user '{}' set, but host-checks feature is disabled. \
                 User/group configuration requires build-time uid/gid lookup from /etc/passwd. \
                 Either enable host-checks feature or remove the user field.",
                self.name, user
            );
        }

        #[cfg(feature = "host-checks")]
        if !uid_map.contains_key(*user) {
            panic!(
                "Service '{}' has user '{}' which does not exist on this system",
                self.name, user
            );
        }
    }

    fn check_group(&self, #[cfg(feature = "host-checks")] gid_map: &HashMap<String, u32>) {
        let Some(group) = &self.group else {
            return;
        };

        #[cfg(not(feature = "host-checks"))]
        {
            panic!(
                "Service '{}' has group '{}' set, but host-checks feature is disabled. \
                 User/group configuration requires build-time uid/gid lookup from /etc/group. \
                 Either enable host-checks feature or remove the group field.",
                self.name, group
            );
        }

        #[cfg(feature = "host-checks")]
        if !gid_map.contains_key(*group) {
            panic!(
                "Service '{}' has group '{}' which does not exist on this system",
                self.name, group
            );
        }
    }

    fn check_chdir(&self) {
        let Some(path) = self.chdir else {
            return;
        };

        if path.contains('\0') {
            panic!(
                "Service '{}' has chdir '{}' which contains a disallowed null byte",
                self.name, path
            );
        }

        let path_obj = Path::new(path);
        if !path_obj.is_absolute() {
            panic!(
                "Service '{}' has chdir '{}' which is not absolute. Only absolute paths are allowed.",
                self.name, path
            );
        }

        #[cfg(feature = "host-checks")]
        {
            if !path_obj.exists() {
                panic!(
                    "Service '{}' has chdir '{}' which does not exist",
                    self.name, path
                );
            }
            if !path_obj.is_dir() {
                panic!(
                    "Service '{}' has chdir '{}' which is not a directory",
                    self.name, path
                );
            }
        }
    }

    fn check_start_stop_cycle(
        self: &'static Service,
        svc_map: &HashMap<&'static str, &'static Service>,
    ) {
        // Depth-first search over the start requirements dependency tree.
        // Currently searching dependencies of `current_svc`
        // If we see `original_svc` as a dependency, that's a cycle.
        //
        // The stop requirement tree is exactly the same, just in the inverse direction.  The same
        // check covers both.
        fn dfs<'a>(
            original_svc: &'static Service,
            current_svc: &'static Service,
            svc_map: &'a HashMap<&'static str, &'static Service>,
            path: &'a mut Vec<(&'static str, &'static Service)>,
            visited: &'a mut HashSet<&'static str>,
        ) {
            // Collect current_svc's relevant dependencies
            let mut deps = Vec::new();
            for &dep in current_svc.needs {
                deps.push(("needs", svc_map[dep]));
            }
            for &dep in current_svc.wants {
                deps.push(("wants", svc_map[dep]));
            }
            if let Log::Service(log_service) = &current_svc.log {
                deps.push(("logs to", svc_map[log_service]));
            }

            // Iterate over current_svc's dependencies:
            // - Checking if we've hit a cycle
            // - further depth first search into tree
            for (dep_type, dep_svc) in deps {
                // Add new item to visited and path
                if visited.contains(dep_svc.name) {
                    // Already checked this service
                    continue;
                }
                visited.insert(dep_svc.name);
                path.push((dep_type, dep_svc));

                // Found a cycle
                if dep_svc.name == original_svc.name {
                    let mut cycle = String::new();
                    for (i, (dep_type, dep_svc)) in path.iter().enumerate() {
                        if i == 0 {
                            cycle.push_str(dep_svc.name);
                        } else if i == 1 {
                            cycle.push(' ');
                            cycle.push_str(dep_type);
                            cycle.push(' ');
                            cycle.push_str(dep_svc.name);
                        } else if i > 1 {
                            cycle.push_str(" which ");
                            cycle.push_str(dep_type);
                            cycle.push(' ');
                            cycle.push_str(dep_svc.name);
                        }
                    }
                    panic!("Dependency cycle: {cycle}");
                }

                // Continue search
                dfs(original_svc, dep_svc, svc_map, path, visited);
                path.pop();
                visited.remove(dep_svc.name);
            }
        }

        // The dependency path taken at a given point in the search.  If the path ever grows to contain
        // the original service, we've found a cycle.  Tracking the full path is useful for good error
        // messages explaining the cycle.
        let mut path = vec![("", self)];

        // List of services we've already checked.
        let mut visited = HashSet::new();

        dfs(self, self, svc_map, &mut path, &mut visited)
    }

    fn check_target_up_stable(
        self: &'static Service,
        svc_map: &HashMap<&'static str, &'static Service>,
    ) {
        // Depth-first search of target propagation when self is set to upward.
        //
        // If a service gets conflicting requirements to be both up and down, its target is
        // unstable.
        fn dfs_up<'a>(
            original_svc: &'static Service,
            current_svc: &'static Service,
            svc_map: &'a HashMap<&'static str, &'static Service>,
            path: &'a mut Vec<(&'static str, &'static Service)>,
            visited_up: &'a mut HashSet<&'static str>,
        ) {
            // Collect current_svc's relevant dependencies
            let mut deps_up = Vec::new();
            for &dep in current_svc.needs {
                deps_up.push(("needs", svc_map[dep]));
            }
            for &dep in current_svc.wants {
                deps_up.push(("wants", svc_map[dep]));
            }
            for &dep in current_svc.groups {
                deps_up.push(("groups", svc_map[dep]));
            }
            if let Log::Service(log_service) = &current_svc.log {
                deps_up.push(("logs to", svc_map[log_service]));
            }

            // Iterate over services to which we're propagating target-up
            //
            // Target-up propagation, alone, cannot cause a dependency cycle.  A given service
            // having its target set to up redundantly is okay.
            for (dep_type, dep_svc) in deps_up {
                // Add new item to visited and path
                if visited_up.contains(dep_svc.name) {
                    // Already checked this service
                    continue;
                }

                // Continue search
                visited_up.insert(dep_svc.name);
                path.push((dep_type, dep_svc));
                dfs_up(original_svc, dep_svc, svc_map, path, visited_up);
                path.pop();
                visited_up.remove(dep_svc.name);
            }

            // Collect services to which a current_svc's target being set up propagates a
            // target-down.
            let mut deps_down = Vec::new();
            for &dep in current_svc.conflicts {
                deps_down.push(("conflicts with", svc_map[dep]));
            }

            // Iterate over services to which we're propagating target-down.
            //
            // If this includes the original service, then we've found a cycle.
            for (dep_type, dep_svc) in deps_down {
                path.push((dep_type, dep_svc));
                let mut visited_down = HashSet::new();
                dfs_down(original_svc, dep_svc, svc_map, path, &mut visited_down);
                path.pop();
            }
        }

        // Depth first search over target propagation when target is set to down.
        fn dfs_down<'a>(
            original_svc: &'static Service,
            current_svc: &'static Service,
            svc_map: &'a HashMap<&'static str, &'static Service>,
            path: &'a mut Vec<(&'static str, &'static Service)>,
            visited_down: &'a mut HashSet<&'static str>,
        ) {
            // Found a cycle
            if current_svc.name == original_svc.name {
                let mut cycle = String::new();
                for (i, (dep_type, dep_svc)) in path.iter().enumerate() {
                    if i == 0 {
                        cycle.push_str(dep_svc.name);
                    } else if i == 1 {
                        cycle.push(' ');
                        cycle.push_str(dep_type);
                        cycle.push(' ');
                        cycle.push_str(dep_svc.name);
                    } else if i > 1 {
                        cycle.push_str(" which ");
                        cycle.push_str(dep_type);
                        cycle.push(' ');
                        cycle.push_str(dep_svc.name);
                    }
                }
                panic!("Dependency cycle: {cycle}");
            }

            // Collect services to which a current_svc's target being set down propagates further
            // target-down.
            //
            // Note a target-down can not propagate a target-up.
            let mut deps_down = Vec::new();
            for &dep in current_svc.groups {
                deps_down.push(("groups", svc_map[dep]));
            }
            for svc in svc_map.values() {
                if svc.needs.contains(&current_svc.name) {
                    deps_down.push(("is needed by", svc));
                }
                if svc.wants.contains(&current_svc.name) {
                    deps_down.push(("is wanted by", svc));
                }
                if svc.groups.contains(&current_svc.name) {
                    deps_down.push(("is grouped by", svc));
                }
                if let Log::Service(log_service) = &svc.log
                    && *log_service == current_svc.name
                {
                    deps_down.push(("receives logs from", svc));
                }
            }

            // Continue search
            for (dep_type, dep_svc) in deps_down {
                // Already checked this service
                if visited_down.contains(dep_svc.name) {
                    continue;
                }

                // Add new item to visited and path
                visited_down.insert(dep_svc.name);
                path.push((dep_type, dep_svc));
                dfs_down(original_svc, dep_svc, svc_map, path, visited_down);
                visited_down.remove(dep_svc.name);
                path.pop();
            }
        }

        // The dependency path taken at a given point in the search.  If the path ever grows to contain
        // the original service, we've found a cycle.  Tracking the full path is useful for good error
        // messages explaining the cycle.
        let mut path = vec![("", self)];

        // List of services we've already checked.
        let mut visited_up = HashSet::new();

        dfs_up(self, self, svc_map, &mut path, &mut visited_up)
    }

    // ===================
    // Helpers
    // ===================

    fn check_dependency(
        &self,
        dep_list: &[&str],
        dep_type: &str,
        svc_map: &HashMap<&'static str, &'static Service>,
    ) {
        let mut seen: HashSet<&str> = HashSet::new();
        for dep in dep_list {
            // Check for self-reference
            if *dep == self.name {
                panic!("Service '{}' references itself in {}", self.name, dep_type);
            }

            // Check that dependency exists
            if svc_map.get(*dep).is_none() {
                panic!(
                    "Service '{}' has {} '{}' which does not exist",
                    self.name, dep_type, dep
                );
            }

            // Check for duplicate dependencies
            if !seen.insert(dep) {
                panic!(
                    "Service '{}' has duplicate {} '{}'",
                    self.name, dep_type, dep
                );
            }
        }
    }

    /// Helper function to validate arguments for start/run/finish
    fn check_exec_args(&self, args: &[&str], context: &str) {
        let Some(path) = args.first() else {
            panic!("Service '{}' has an empty {} argument", self.name, context);
        };
        if path.contains('\0') {
            panic!(
                "Service '{}' has {} path '{}' which contains a disallowed null byte",
                self.name, context, path
            );
        }
        if path.len() > MSG_PATH_SIZE {
            panic!(
                "Service '{}' has {} path '{}' with more bytes than max allowed of {}",
                self.name, context, path, MSG_PATH_SIZE,
            );
        }
        let path_obj = Path::new(path);
        if !path_obj.is_absolute() {
            panic!(
                "Service '{}' has {} path '{}' which is not absolute. Only absolute paths are allowed.",
                self.name, context, path
            );
        }

        #[cfg(feature = "host-checks")]
        {
            if !path_obj.exists() {
                panic!(
                    "Service '{}' has {} path '{}' which does not exist",
                    self.name, context, path
                );
            }
            if !path_obj.is_file() {
                panic!(
                    "Service '{}' has {} path '{}' which is not a file (it may be a directory)",
                    self.name, context, path
                );
            }
            let Ok(metadata) = path_obj.metadata() else {
                panic!(
                    "Service '{}' has {} path '{}' which is unreadable",
                    self.name, context, path
                );
            };
            // Check if any execute bit is set (owner, group, or other)
            if metadata.permissions().mode() & 0o111 == 0 {
                panic!(
                    "Service '{}' has {} path '{}' which is not executable",
                    self.name, context, path
                );
            }
        }

        for arg in args {
            if arg.is_empty() {
                panic!("Service '{}' has an empty {} argument", self.name, context);
            }
            if CString::from_str(arg).is_err() {
                panic!(
                    "Service '{}' has a {} argument which cannot be converted into a C string: {}",
                    self.name, context, arg
                );
            }
        }
    }

    fn check_shell_command(&self, cmd: &str, context: &str) {
        if cmd.is_empty() {
            panic!(
                "Service '{}' has an empty {} Shell command",
                self.name, context
            );
        }
        if CString::from_str(cmd).is_err() {
            panic!(
                "Service '{}' has a {} Shell command which cannot be converted into a C string: {}",
                self.name, context, cmd
            );
        }

        #[cfg(feature = "host-checks")]
        {
            if !Path::new("/bin/sh").exists() {
                panic!(
                    "Service '{}' uses Shell for {} but /bin/sh does not exist",
                    self.name, context
                );
            }
        }
    }

    fn check_duration(&self, duration: Option<Duration>, duration_name: &str) {
        let Some(duration) = duration else {
            return;
        };

        // For a convenient interface, we're using Rust's Duration type which can represent a very
        // large number of milliseconds.  However, this is being fed into poll(2) which takes an i32.
        if duration.as_millis() > i32::MAX as u128 {
            panic!(
                "Service '{}' has {} duration which is larger than maximum allowed {} milliseconds, or roughly {} days",
                self.name,
                duration_name,
                i32::MAX,
                i32::MAX / 1000 / 60 / 60 / 24
            );
        }
    }
}
