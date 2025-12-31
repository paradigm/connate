use crate::internal::ServiceArrayFind;
use crate::next_state::*;
use crate::session::*;
use connate::internal_api::*;
use connate::ipc::*;
use connate::os::*;
use connate::types::*;
use core::cmp::max;

/// Handle an IPC request from conctl or a supervisor
pub fn handle_request<const N: usize>(
    mut svcs: &mut [Service; N],
    ipc_server: &mut IpcServer,
    session_fd: &mut SessionFd,
    now: timespec,
) {
    use Target::*;

    let response = match ipc_server.receive() {
        Request::Exec(cstr) => {
            // Save state into memfd before exec'ing
            if session_fd.save(svcs).is_err() {
                ipc_server.respond(Response::Failed);
                return;
            }
            // Exec the new binary.
            //
            // If this returns, exec failed.
            // (successful exec replaces the process and never returns)
            let _ = if cstr.is_empty() {
                connate::os::exec_self()
            } else {
                connate::os::exec_filepath(cstr)
            };
            ipc_server.respond(Response::Failed);
            return;
        }
        Request::QueryByIndexStatus(i) => match svcs.get(i) {
            Some(svc) => Response::Status(
                svc.state,
                svc.target,
                svc.pid,
                svc.exit_code,
                max(0, now.tv_sec - svc.time.tv_sec),
            ),
            None => Response::ServiceNotFound,
        },
        Request::QueryByIndexName(i) => match svcs.get(i) {
            Some(svc) => Response::Name(svc.cfg.name),
            None => Response::ServiceNotFound,
        },
        Request::QueryByIndexState(i) => match svcs.get(i) {
            Some(svc) => Response::State(svc.state),
            None => Response::ServiceNotFound,
        },
        Request::QueryByIndexTarget(i) => match svcs.get(i) {
            Some(svc) => Response::Target(svc.target),
            None => Response::ServiceNotFound,
        },
        Request::QueryByIndexPid(i) => match svcs.get(i).map(|svc| svc.pid) {
            Some(Some(pid)) => Response::Pid(pid),
            Some(None) => Response::FieldIsNone,
            None => Response::ServiceNotFound,
        },
        Request::QueryByIndexExitCode(i) => match svcs.get(i).map(|svc| svc.exit_code) {
            Some(Some(value)) => Response::ExitCode(value),
            Some(None) => Response::FieldIsNone,
            None => Response::ServiceNotFound,
        },
        Request::QueryByIndexAttemptCount(i) => match svcs.get(i) {
            Some(svc) => Response::AttemptCount(svc.attempt_count as u64),
            None => Response::ServiceNotFound,
        },
        Request::QueryByIndexTime(i) => match svcs.get(i) {
            Some(svc) => Response::Time(max(0, now.tv_sec - svc.time.tv_sec)),
            None => Response::ServiceNotFound,
        },
        Request::QueryByNameStatus(name) => match svcs.find_by_name(name) {
            Some(svc) => Response::Status(
                svc.state,
                svc.target,
                svc.pid,
                svc.exit_code,
                max(0, now.tv_sec - svc.time.tv_sec),
            ),
            None => Response::ServiceNotFound,
        },
        Request::QueryByNameState(name) => match svcs.find_by_name(name) {
            Some(svc) => Response::State(svc.state),
            None => Response::ServiceNotFound,
        },
        Request::QueryByNameTarget(name) => match svcs.find_by_name(name) {
            Some(svc) => Response::Target(svc.target),
            None => Response::ServiceNotFound,
        },
        Request::QueryByNamePid(name) => match svcs.find_by_name(name).map(|svc| svc.pid) {
            Some(Some(pid)) => Response::Pid(pid),
            Some(None) => Response::FieldIsNone,
            None => Response::ServiceNotFound,
        },
        Request::QueryByNameExitCode(name) => {
            match svcs.find_by_name(name).map(|svc| svc.exit_code) {
                Some(Some(value)) => Response::ExitCode(value),
                Some(None) => Response::FieldIsNone,
                None => Response::ServiceNotFound,
            }
        }
        Request::QueryByNameAttemptCount(name) => match svcs.find_by_name(name) {
            Some(svc) => Response::AttemptCount(svc.attempt_count as u64),
            None => Response::ServiceNotFound,
        },
        Request::QueryByNameTime(name) => match svcs.find_by_name(name) {
            Some(svc) => Response::Time(max(0, now.tv_sec - svc.time.tv_sec)),
            None => Response::ServiceNotFound,
        },
        Request::QueryNeeds(i, name) => match svcs.find_by_name(name) {
            Some(svc) => match svc.cfg.needs.get(i).and_then(|&i| svcs.get(i)) {
                Some(dep) => Response::Name(dep.cfg.name),
                None => Response::FieldIsNone,
            },
            None => Response::ServiceNotFound,
        },
        Request::QueryWants(i, name) => match svcs.find_by_name(name) {
            Some(svc) => match svc.cfg.wants.get(i).and_then(|&i| svcs.get(i)) {
                Some(dep) => Response::Name(dep.cfg.name),
                None => Response::FieldIsNone,
            },
            None => Response::ServiceNotFound,
        },
        Request::QueryConflicts(i, name) => match svcs.find_by_name(name) {
            Some(svc) => match svc.cfg.conflicts.get(i).and_then(|&i| svcs.get(i)) {
                Some(dep) => Response::Name(dep.cfg.name),
                None => Response::FieldIsNone,
            },
            None => Response::ServiceNotFound,
        },
        Request::QueryGroups(i, name) => match svcs.find_by_name(name) {
            Some(svc) => match svc.cfg.groups.get(i).and_then(|&i| svcs.get(i)) {
                Some(dep) => Response::Name(dep.cfg.name),
                None => Response::FieldIsNone,
            },
            None => Response::ServiceNotFound,
        },
        Request::QueryByIndexLog(i) => match svcs.get(i) {
            Some(svc) => svc.cfg.log.as_response(svcs),
            None => Response::ServiceNotFound,
        },
        Request::QueryByNameLog(name) => match svcs.find_by_name(name) {
            Some(svc) => svc.cfg.log.as_response(svcs),
            None => Response::ServiceNotFound,
        },
        Request::SetTargetUp(name) => match svcs.find_by_name(name) {
            Some(svc) => set_target(svcs, svc.cfg.index, now, Up),
            None => Response::ServiceNotFound,
        },
        Request::SetTargetDown(name) => match svcs.find_by_name(name) {
            Some(svc) => set_target(svcs, svc.cfg.index, now, Down),
            None => Response::ServiceNotFound,
        },
        Request::SetTargetRestart(name) => match svcs.find_by_name(name) {
            Some(svc) => set_target(svcs, svc.cfg.index, now, Restart),
            None => Response::ServiceNotFound,
        },
        Request::SetTargetOnce(name) => match svcs.find_by_name(name) {
            Some(svc) => set_target(svcs, svc.cfg.index, now, Once),
            None => Response::ServiceNotFound,
        },
        #[cfg(feature = "settle")]
        Request::QuerySettleFd(name) => match svcs.find_by_name_mut(name) {
            Some(svc) => {
                // Create settle pipe lazily if it doesn't exist
                if svc.settle_pipe.is_none() {
                    match Fd::new_pipe(OpenFlags::O_NONBLOCK) {
                        Ok(pipe) => svc.settle_pipe = Some(pipe),
                        Err(_) => {
                            ipc_server.respond(Response::Failed);
                            return;
                        }
                    }
                }
                // Return the read FD number
                match svc.settle_pipe {
                    Some((ref read_fd, _)) => Response::SettleFd(read_fd.as_raw()),
                    None => Response::Failed,
                }
            }
            None => Response::ServiceNotFound,
        },
        #[cfg(not(feature = "settle"))]
        Request::QuerySettleFd(_) => Response::SettleDisabled,
        // A supervisor has forked the service process and provided us the pid.  Look up via name.
        Request::ServiceStarting(pid, name) => match svcs.find_by_name_mut(name) {
            Some(svc) => {
                if pid < 2 {
                    Response::InvalidRequest
                } else {
                    svc.pid = Some(pid);
                    svc.dirty = true;
                    Response::Okay
                }
            }
            None => Response::ServiceNotFound,
        },
        // Service ran `conctl ready` or `notify_ready()`.  Notably, this doesn't require
        // submitting the service's own name.
        //
        // These mechanisms searches up the process tree until it finds connate, then provides its
        // ancestor closest to connate.  This could be either the service's main pid or the
        // supervisor.
        Request::ServiceReady(pid) => match svcs.find_by_direct_or_supervisor_pid_mut(pid) {
            Some(svc) => {
                svc.ready = true;
                svc.dirty = true;
                Response::Okay
            }
            None => Response::ServiceNotFound,
        },
        // A supervisor witnessed its service daemonize, which both indicates readiness and updates
        // the pid.
        Request::DaemonReady(pid, name) => match svcs.find_by_name_mut(name) {
            Some(svc) => {
                if pid < 2 {
                    Response::InvalidRequest
                } else {
                    svc.pid = Some(pid);
                    svc.ready = true;
                    svc.dirty = true;
                    Response::Okay
                }
            }
            None => Response::ServiceNotFound,
        },
        Request::Invalid => Response::InvalidRequest,
    };

    ipc_server.respond(response);
}

pub fn set_target<'a, const N: usize>(
    svcs: &mut [Service; N],
    index: usize,
    now: timespec,
    target: Target,
) -> Response<'a> {
    // Temporarily get immutable reference to collect data about service
    let Some(svc) = svcs.get(index) else {
        return Response::ServiceNotFound;
    };

    // Grab propagation lists and implicitly drop svc to alleviate borrow checker concerns
    let Some(cfg) = svcs.get(index).map(|svc| svc.cfg) else {
        return Response::ServiceNotFound;
    };

    // If the service was in a failed state, it will not automatically transition.
    // Explicitly (re)setting the target here breaks it out of the failed state
    if matches!(svc.state, State::Failed) {
        NextState::Down.apply(svcs, index, now);
    }

    // Temporarily get mutable reference to service to update target
    let Some(svc) = svcs.get_mut(index) else {
        return Response::ServiceNotFound;
    };

    // Update target and note as dirty
    svc.target = target;
    svc.dirty = true;

    // Propagate target changes to dependents and dependencies to ensure this service isn't blocked
    // on proceeding to its new target
    match target {
        Target::Up | Target::Once => {
            // When going Up or Once:
            // - All dependencies (needs/wants/groups/log service) should go Up
            // - All conflicts should go Down
            for &i in cfg.target_up_propagate_up {
                match svcs.get_mut(i) {
                    Some(svc) => {
                        svc.target = Target::Up;
                        svc.dirty = true;
                    }
                    None => return Response::ServiceNotFound,
                }
            }
            for &i in cfg.target_up_propagate_down {
                match svcs.get_mut(i) {
                    Some(svc) => {
                        svc.target = Target::Down;
                        svc.dirty = true;
                    }
                    None => return Response::ServiceNotFound,
                }
            }
        }
        Target::Down => {
            // When going Down
            // - All dependents (services that need/want/group this) should go Down
            for &i in cfg.target_down_propagate_down {
                match svcs.get_mut(i) {
                    Some(svc) => {
                        svc.target = Target::Down;
                        svc.dirty = true;
                    }
                    None => return Response::ServiceNotFound,
                }
            }
        }
        Target::Restart => {
            // When Restarting:
            // - We're immediately going down, but we'll eventually go back up.
            // - To handle going down, all dependents should immediately go Down.
            //   - However, they may then go back up after this service goes back up.
            //   - If the dependent target is Up, change to Restart so that it'll go down to
            //   unblock us going down, but go back up after this service does and resume the prior
            //   upward target.
            //   - Otherwise, go down.
            // - To handle going back up, all dependencies should eventually go Up.
            //   - This may be immediately Up or after going down i.e. Restart.
            // - To handle going back up, all conflicts should go Down.
            for &i in cfg.target_down_propagate_down {
                match svcs.get_mut(i) {
                    Some(svc) => match svc.target {
                        Target::Down | Target::Restart => {}
                        Target::Up => {
                            svc.target = Target::Restart;
                            svc.dirty = true;
                        }
                        Target::Once => {
                            svc.target = Target::Down;
                            svc.dirty = true;
                        }
                    },
                    None => return Response::ServiceNotFound,
                }
            }
            for &i in cfg.target_up_propagate_up {
                match svcs.get_mut(i) {
                    Some(svc) => match svc.target {
                        Target::Up | Target::Restart | Target::Once => {}
                        Target::Down => {
                            svc.target = Target::Up;
                            svc.dirty = true;
                        }
                    },
                    None => return Response::ServiceNotFound,
                }
            }
            for &i in cfg.target_up_propagate_down {
                match svcs.get_mut(i) {
                    Some(svc) => {
                        svc.target = Target::Down;
                        svc.dirty = true;
                    }
                    None => return Response::ServiceNotFound,
                }
            }
        }
    }

    // Group members inherit the new target
    for &i in cfg.groups {
        match svcs.get_mut(i) {
            Some(svc) => {
                svc.target = target;
                svc.dirty = true;
            }
            None => return Response::ServiceNotFound,
        }
    }

    Response::Okay
}
