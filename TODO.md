TODO
====

- config features
	- capabilities: Option<&'static [&'static str]> — ambient/effective sets.
	- supplementary_groups: Option<&'static [&'static str]>
	- seccomp: Option<&'static str> — precompiled filter name.
	- selinux_label: Option<&'static str>, apparmor_profile: Option<&'static str>
	- namespaces: Option<Namespaces> — pid|net|mnt|uts|ipc|user toggles.
	- cgroup: Option<Cgroup> — v2 knobs: cpu_max, memory_max, io_max, pids_max, oom_score_adj.
	- nice
	- ioprio
	- rlimits
	- oom_score_adjust
	- chroot
	- cgroups
