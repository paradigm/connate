Connate
=======

Connate is a service manager for Linux which strives to provide a reasonable
feature set while minimizing possible failure modes.

Design Goals
------------

Connate goes to extremes to minimize possible failure modes of the service
manager process:

- Operates in fixed memory at runtime.
    - No heap allocation either directly or indirectly through a library.
    - No recursion.
    - No variable length arrays.
- Core functionality never writes to disk.
    - No socket files.
    - No temporary files.
    - No on-disk state representation.
- Fixed runtime kernel resource allocation.
    - Opens exactly six file descriptors upon initialization:
        - One signalfd
        - One memfd (which is ftruncate'd to a fixed size)
        - Two FDs for a request pipe
        - Two FDs for a response pipe
    - Optionally, may open up to two more FDs per service to broadcast state
    - Optionally, may open up to two more FDs per service to pipe logging info
- No external runtime dependencies
    - Does not require `/bin/sh`, python, etc
- Enforced configuration checking before the configuration is available for use
    - Things like mistyped service names or dependency cycles in configuration
    are caught before the system becomes dependent on the configuration.

Additionally:

- It's small.  The statically linked connate binary, at the time of writing on
  the author's production system, is less than 50KB in size.
- It's reasonably well featured: it includes common modern expectations such as
  parallel running of services and various types of dependencies.
- Useful both as init, managing the entire system, and as an unprivileged user
  process managing per-user services.

Note that none of these constraints are necessarily applied to the configured
services.  You're welcome to have your service written in `/bin/sh` or log to
disk, for example.

The design goals are achieved largely by:

- Coding directly to the system calls without the Rust standard library or libc.
    - No allocator, no hidden panic!() calls, etc.
- Configuration directly in source code.
    - Configuration must thus must pass static checks for things like dependency
    cycles before being included in the binary to actually be used.
- Scanning the binary for panic machinery inclusion, indicating whether the
  compiler succeeded in to ruling out possible panics.

Reasons Connate May Be Wrong For You
------------------------------------

- Connate requires some Rust and Linux background to configure.  This, sadly,
  may make it inaccessible to many users.
- Connate lacks features other service managers include.  If you require
  such features, other service managers maybe preferable for you.
- Connate currently relies on Linux kernel specific features which makes it
  unsuitable for other operating systems.
- Connate is currently difficult to configure automatically, e.g. via a package
  manager, which may make it unsuitable for use as the default service manager
  for Linux distributions.
- Connate has a small user base, which by extension means it is not well
  exercised, unproven, and has limited community support available.

Configuration
-------------

- Review the type definitions within `src/config/config_api.rs`
- Review the example configuration at `src/config/example*.rs`
- Create/update the contents at `src/config/config.rs`
- Note the build-time features in `Cargo.toml`

Building And Installing
-----------------------

After configuring, run either

```
cargo build --release
```

to build for the local system or

```
cargo build --release --no-default-features --features <features>
```

to build with/without specific features.

The service manager will be found at

```
./target/release/connate
```

and the CLI utility to control the service manager will be found at

```
./target/release/conctl
```

Copy `./target/release/conctl` into your `$PATH` (e.g. `/usr/local/bin/conctl`).

Copy `./target/release/connate` to either:

- `/sbin/init` if it is your system-wide init
- Into your `$PATH` if you want to run it as a non-init service manager (e.g.
  `/usr/local/bin/connate`).

Usage
-----

Run `conctl` to perform basic service manager operations such as

- Listing available services and their status
- Changing service target state, e.g. bringing up or down
- Instructing `connate` to re-exec its binary to update configuration

Security
--------

Communication with and control of Connate occurs via

```
/proc/<connate-pid>/fd/<fd>
```

Linux constrains these fd permissions to the user/group of the process.
Whichever user connate is running as is, generally, the only user (other than
root) that can manipulate it.

To avoid multiple connate sessions with the same services, connate optionally
locks a lock file upon starting.  For an adversary to preemptively lock the
file and block connate from starting, they would need write access to the file.

FAQ
---

- Why not an existing service manager / init system?

This has a preferable set of trade-offs for the author's desires.  If you find
another solution which better meets your needs, feel free to use it instead.

- Why name the project "Connate"?

The word "connate" means something which exists in/with something else from the
moment it is born.  Connate's services/configuration are compiled into it; they
exist within it from the very instant it starts.  They are connate to it.

- Why not dynamic loading for configuration?

On Linux with many libcs dynamically loaded library information is often
cached.  When connate attempts to reload newly updated configuration at the
same filepath it will receive the previously cached library.  This hampers
reliably reloading configuration.

- Why never write to disk?

If the process managing the files dies (e.g. due to a power outage), the files
then become stale and outdated.  If state is never written to disk, this is not
a problem.

Moreover, this restriction helps keep Connate's scope in check.  Most uses of
writing to disk, such as logging, can be done by services rather than Connate
proper.

- Why Rust?

Interpreted languages such as Bash, Perl, or Python introduce unneeded
dependencies.

The original Connate 1.0 was written in C, but in-source representation of
necessary concepts was ungainly.  Rust's syntactic sugar is more ergonomic for
Connate's configuration strategy.

The Rust compiler indirectly communicates whether it has ruled out possible
panics due to things like potential segfaults, which we can use to validate the
robustness of the program.

- Why not use cgroups?

One of Connate's goals is to function without special permissions.  Cgroup
requires root (or the assistance of root), and is thus disallowed as a core
feature.  A non-root user (e.g. normal user in non-rooted Android application
such as Termux, a student on a shared university system, etc) cannot use
cgroups without additional, possibly unavailable assistance.

The lack of cgroups does not impair Connate's ability to track daemonizing and
multiprocess services.  Connate spawns a process with `PR_SET_CHILD_SUBREAPER`
set as the parent of appropriate services.  The various service processes are
thus all children of the supervising process and can be trivially found via
/proc.

While Connate itself does not use cgroups, you're welcome to configure its
services to use them to constrain their resource usage.

- Why no "provides" dependency?

Without provides, the dependency graph is acyclic and can be easily flattened
during preprocessing.  Including support for "provides" would add undesired
complexity to Connate's runtime.

- Why so many states?

Connate could have easily hidden most of the states and only surfaced a subset
such as "up" and "down."  However, this would make debugging buggy services
unnecessarily difficult.  For example, a user would benefit from knowing
whether a given service is down due to a bug in its initialization code
or because it is still waiting for dependencies.
