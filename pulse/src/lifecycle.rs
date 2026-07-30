//! Service lifecycle state and transition validation ("service
//! lifecycle management" from Pulse's README).

/// A service's lifecycle state. Doesn't track which state a real
/// service is actually in -- there's no real running service yet to
/// track -- only defines what the states are and which transitions
/// between them make sense.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServiceState {
    Stopped,
    Starting,
    Running,
    Stopping,
    Failed,
}

/// Whether transitioning from `from` to `to` is a valid lifecycle
/// step. This is one reasonable, minimal lifecycle shape -- not
/// claimed as the final word on Pulse's actual service model, same
/// caveat as security.rs's is_authorized on the kernel side. Notable
/// modeling choices, stated rather than left implicit:
///
/// - `Stopping -> Failed` is allowed: a stop can time out and need a
///   force-kill, which doesn't count as a clean stop.
/// - `Failed -> Starting` is allowed (a restart attempt) *and*
///   `Failed -> Stopped` is allowed (giving up rather than retrying)
///   -- restart::RestartPolicy::should_restart is what actually
///   decides which of those two should happen for a real failure,
///   this only confirms both are legal outcomes to land in.
/// - No self-transitions (Running -> Running, etc.) are valid: a
///   transition implies a change, not a no-op.
pub fn is_valid_transition(from: ServiceState, to: ServiceState) -> bool {
    use ServiceState::*;
    matches!(
        (from, to),
        (Stopped, Starting)
            | (Starting, Running)
            | (Starting, Failed)
            | (Running, Stopping)
            | (Running, Failed)
            | (Stopping, Stopped)
            | (Stopping, Failed)
            | (Failed, Starting)
            | (Failed, Stopped)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn cannot_skip_starting_to_reach_running_directly() {
        assert!(!is_valid_transition(ServiceState::Stopped, ServiceState::Running));
    }

    #[test]
    fn cannot_transition_to_the_same_state() {
        use ServiceState::*;
        for state in [Stopped, Starting, Running, Stopping, Failed] {
            assert!(
                !is_valid_transition(state, state),
                "{:?} -> itself should be invalid",
                state
            );
        }
    }

    #[test]
    fn stopping_can_end_in_failed_via_a_forced_kill() {
        assert!(is_valid_transition(ServiceState::Stopping, ServiceState::Failed));
    }

    #[test]
    fn failed_can_either_restart_or_give_up() {
        assert!(is_valid_transition(ServiceState::Failed, ServiceState::Starting));
        assert!(is_valid_transition(ServiceState::Failed, ServiceState::Stopped));
    }

    #[test]
    fn transition_table_matches_exactly_the_documented_valid_set() {
        // Exhaustive: all 25 (from, to) pairs across 5 states, checked
        // against an independently-constructed expected set -- a
        // stronger guarantee than spot-checking individual cases,
        // since it can't silently miss a combination nobody thought
        // to write a named test for.
        use ServiceState::*;
        let all_states = [Stopped, Starting, Running, Stopping, Failed];
        let valid_pairs: HashSet<(ServiceState, ServiceState)> = [
            (Stopped, Starting),
            (Starting, Running),
            (Starting, Failed),
            (Running, Stopping),
            (Running, Failed),
            (Stopping, Stopped),
            (Stopping, Failed),
            (Failed, Starting),
            (Failed, Stopped),
        ]
        .into_iter()
        .collect();

        for &from in &all_states {
            for &to in &all_states {
                let expected = valid_pairs.contains(&(from, to));
                assert_eq!(
                    is_valid_transition(from, to),
                    expected,
                    "transition {:?} -> {:?} should be {}",
                    from,
                    to,
                    expected
                );
            }
        }
    }
}
