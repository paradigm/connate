# Architecture

This document describes the high-level architecture of Connate

## Code entry points

- src/build/
  - Build binary
  - Sanity checks user created config.rs
  - Generates pre-processed internal representation of user config.rs
- src/connate/
  - Service manager binary
- src/conctl/
  - Binary to control connate binary
- src/lib.rs
  - Shared code

## Build flow

- src/build/
  - `mod`'s user-created src/config/config.rs
  - Build-time error if user config fails interface type checks
  - Build-time error if user config fails sanity checks, e.g. dependency cycles
  - Generates pre-processed src/internal.rs
- src/connate/
  - `mod`'s generated src/internal/config.rs
  - Outputs connate binary
- src/conctl/
  - `mod`'s generated src/internal/config.rs
  - Outputs conctl binary

## IPC flow

- connate initialization
  - Optionally fctl locks the configured lock file
  - Creates pipe for requests from conctl
  - Creates pipe for responses to conctl
  - Sleeps with poll() on request pipe and signalfd until activity
- conctl command run
  - Optionally reads lock to determine connate PID
  - fctl locks /proc/${PID}/fd/${REQUEST_WRITE}
  - atomic writes request to /proc/${PID}/fd/${REQUEST_WRITE}
  - atomic blocking read on /proc/${PID}/fd/${RESPONSE_READ}
- connate response
  - poll() awakens on ${REQUEST_READ} available
  - atomic reads ${REQUEST_READ}
  - processes
  - atomic writes response to ${RESPONSE_WRITE}
  - handles possible service state transition
  - sleeps again with poll() until activity
- conctl
  - blocking read on ${RESPONSE_READ} continues with response from connate
  - processes

## Re-exec flow

- Either conctl sends Request::Exec or something with permission sends SIGHUP
- old connate receives Request::Exec or SIGHUP
- old connate serializes state into memfd
- old connate exec's either:
  - Specified path if one was specified via Request::Exec
  - `/proc/self/exec` (after removing ` (deleted)`)
- new exec detects old memefd
- new exec deserializes state from memfd
- new exec responds over IPC with Response::Okay
