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

/// A service supervisor instance managing a single service's state,
/// restart backoff, and health tracking.
#[derive(Debug, Clone)]
pub struct ServiceSupervisor {
    name: String,
    state: ServiceState,
    restart_policy: crate::restart::RestartPolicy,
    health_policy: crate::health::HealthPolicy,
}

impl ServiceSupervisor {
    pub fn new(
        name: impl Into<String>,
        restart_policy: crate::restart::RestartPolicy,
        health_policy: crate::health::HealthPolicy,
    ) -> Self {
        Self {
            name: name.into(),
            state: ServiceState::Stopped,
            restart_policy,
            health_policy,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn state(&self) -> ServiceState {
        self.state
    }

    pub fn restart_policy(&self) -> &crate::restart::RestartPolicy {
        &self.restart_policy
    }

    pub fn health_policy(&self) -> &crate::health::HealthPolicy {
        &self.health_policy
    }

    /// Attempt to transition to `target_state`. Returns `true` if transition was legal and applied.
    pub fn transition_to(&mut self, target_state: ServiceState) -> bool {
        if is_valid_transition(self.state, target_state) {
            self.state = target_state;
            true
        } else {
            false
        }
    }

    /// Record a crash event timestamp `now` with past `failure_history`. Updates restart policy tracking and
    /// transitions to `Starting` (if restart is allowed) or `Stopped` (if backoff limit reached).
    pub fn handle_failure(
        &mut self,
        now: std::time::Instant,
        failure_history: &[std::time::Instant],
    ) -> ServiceState {
        if !self.transition_to(ServiceState::Failed) {
            return self.state;
        }

        if self.restart_policy.should_restart(now, failure_history) {
            self.transition_to(ServiceState::Starting);
        } else {
            self.transition_to(ServiceState::Stopped);
        }

        self.state
    }

    /// Evaluate current health recency at timestamp `now` against `last_heartbeat`.
    pub fn evaluate_health(
        &self,
        now: std::time::Instant,
        last_heartbeat: std::time::Instant,
    ) -> crate::health::HealthStatus {
        self.health_policy.assess(now, last_heartbeat)
    }
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

    #[test]
    fn supervisor_tracks_lifecycle_and_restart_policy() {
        use std::time::{Duration, Instant};

        let restart = crate::restart::RestartPolicy::new(2, Duration::from_secs(100));
        let health = crate::health::HealthPolicy::new(Duration::from_secs(10), Duration::from_secs(30)).unwrap();
        let mut supervisor = ServiceSupervisor::new("test-service", restart, health);

        assert_eq!(supervisor.name(), "test-service");
        assert_eq!(supervisor.state(), ServiceState::Stopped);

        assert!(supervisor.transition_to(ServiceState::Starting));
        assert!(supervisor.transition_to(ServiceState::Running));

        let now = Instant::now();
        let healthy_hb = now - Duration::from_secs(5);
        let degraded_hb = now - Duration::from_secs(15);
        let unhealthy_hb = now - Duration::from_secs(35);

        // Evaluate health recency
        assert_eq!(supervisor.evaluate_health(now, healthy_hb), crate::health::HealthStatus::Healthy);
        assert_eq!(supervisor.evaluate_health(now, degraded_hb), crate::health::HealthStatus::Degraded);
        assert_eq!(supervisor.evaluate_health(now, unhealthy_hb), crate::health::HealthStatus::Unhealthy);

        let t1 = now - Duration::from_secs(20);
        let t2 = now - Duration::from_secs(10);
        let t3 = now;

        let mut failure_history = Vec::new();

        // First crash -> restarts -> state becomes Starting
        failure_history.push(t1);
        assert_eq!(supervisor.handle_failure(t1, &failure_history), ServiceState::Starting);
        assert!(supervisor.transition_to(ServiceState::Running));

        // Second crash -> restarts -> state becomes Starting
        failure_history.push(t2);
        assert_eq!(supervisor.handle_failure(t2, &failure_history), ServiceState::Starting);
        assert!(supervisor.transition_to(ServiceState::Running));

        // Third crash -> (exceeding 2 crashes in window) -> backoff exhausted -> state becomes Stopped
        failure_history.push(t3);
        assert_eq!(supervisor.handle_failure(t3, &failure_history), ServiceState::Stopped);
    }
}
