//! Pulse: system initialization and service management.
//!
//! This first slice covers exactly one of the README's listed
//! responsibilities -- "boot service graph" and "dependency
//! resolution" -- and deliberately not the rest ("service lifecycle
//! management", "restart policy", "health supervision"). Those all
//! need a real running service to manage, which needs real process
//! execution -- something adrian-kernel doesn't have yet either (no
//! context switching, no bare-metal target this can actually boot
//! on). What's genuinely buildable and verifiable right now is the
//! part that doesn't need any of that: given a set of services and
//! what each depends on, compute a valid order to start them in, or
//! detect that no valid order exists.
//!
//! Not a no_std crate: Pulse is meant to run in userspace once a real
//! bridge exists, not inside the kernel itself, so there's no reason
//! to forgo std here the way adrian-kernel has to.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

/// A service's static definition: its name and what it depends on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceManifest {
    pub name: String,
    pub dependencies: Vec<String>,
}

impl ServiceManifest {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            dependencies: Vec::new(),
        }
    }

    /// Builder-style: `ServiceManifest::new("x").depends_on("a").depends_on("b")`.
    pub fn depends_on(mut self, dependency: impl Into<String>) -> Self {
        self.dependencies.push(dependency.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolutionError {
    /// A service depends on a name that isn't in the manifest set --
    /// reported before any ordering is attempted, since this is
    /// almost always a typo or a missing manifest, not a genuine
    /// cycle, and deserves a clearer error than treating it as one.
    UnknownDependency { service: String, missing: String },
    /// The dependency graph has a cycle: these services can never
    /// have a valid start order. Carries whichever service names were
    /// still unresolved when nothing further could proceed, sorted --
    /// enough to act on without claiming more precision (like the
    /// exact cycle path) than a straightforward check actually
    /// provides.
    Cycle(Vec<String>),
}

/// Compute a valid start order: every service appears after
/// everything it depends on. Kahn's algorithm -- repeatedly pull out
/// every service with no unresolved dependencies left, in
/// deterministic name-sorted order so the result doesn't depend on
/// hash-map iteration order, which Rust deliberately randomizes.
pub fn resolve_start_order(services: &[ServiceManifest]) -> Result<Vec<String>, ResolutionError> {
    let names: HashSet<&str> = services.iter().map(|s| s.name.as_str()).collect();

    for service in services {
        for dep in &service.dependencies {
            if !names.contains(dep.as_str()) {
                return Err(ResolutionError::UnknownDependency {
                    service: service.name.clone(),
                    missing: dep.clone(),
                });
            }
        }
    }

    let mut remaining: HashMap<&str, &ServiceManifest> =
        services.iter().map(|s| (s.name.as_str(), s)).collect();
    let mut resolved: HashSet<String> = HashSet::new();
    let mut order: Vec<String> = Vec::new();

    while !remaining.is_empty() {
        let mut ready: Vec<&str> = remaining
            .values()
            .filter(|s| s.dependencies.iter().all(|d| resolved.contains(d)))
            .map(|s| s.name.as_str())
            .collect();
        ready.sort();

        if ready.is_empty() {
            let mut stuck: Vec<String> = remaining.keys().map(|s| s.to_string()).collect();
            stuck.sort();
            return Err(ResolutionError::Cycle(stuck));
        }

        for name in ready {
            resolved.insert(name.to_string());
            order.push(name.to_string());
            remaining.remove(name);
        }
    }

    Ok(order)
}

/// A policy for whether to restart a crashed service, based on its
/// recent failure history. Doesn't touch any real process, and
/// doesn't track history itself either -- it only answers "should
/// another restart be attempted", given a record of when past
/// failures happened. Actually tracking that history for a real
/// running service, and spawning the replacement process, both need
/// real process execution and a real supervisor loop, neither of
/// which exists yet -- this is deliberately just the decision, kept
/// separate and stateless so it's fully testable on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RestartPolicy {
    /// Maximum failures allowed within `window` before giving up.
    pub max_failures: u32,
    pub window: Duration,
}

impl RestartPolicy {
    pub const fn new(max_failures: u32, window: Duration) -> Self {
        Self { max_failures, window }
    }

    /// Whether another restart should be attempted, given `now` and
    /// the timestamps of past failures (any order). Only failures
    /// within `window` of `now` count against the budget -- a service
    /// that's been stable for a while gets a fresh allowance rather
    /// than being penalized forever for failures from long ago.
    pub fn should_restart(&self, now: Instant, failure_times: &[Instant]) -> bool {
        let recent_failures = failure_times
            .iter()
            .filter(|&&failure| now.duration_since(failure) <= self.window)
            .count();
        (recent_failures as u32) < self.max_failures
    }
}

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
/// caveat as security.rs's is_authorized. Notable modeling choices,
/// stated rather than left implicit:
///
/// - `Stopping -> Failed` is allowed: a stop can time out and need a
///   force-kill, which doesn't count as a clean stop.
/// - `Failed -> Starting` is allowed (a restart attempt) *and*
///   `Failed -> Stopped` is allowed (giving up rather than retrying)
///   -- RestartPolicy::should_restart is what actually decides which
///   of those two should happen for a real failure, this only
///   confirms both are legal outcomes to land in.
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

/// The last of Pulse's five listed responsibilities to get real logic
/// -- boot graph, dependency resolution, restart policy, and
/// lifecycle transitions are all real already; this is health
/// supervision's turn. Same "decision, not execution" split as the
/// rest: doesn't send or receive any real heartbeat, since that needs
/// a real running service to report one. Only judges recency against
/// two thresholds, given a last-heartbeat time handed to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// Heartbeat received within the expected interval.
    Healthy,
    /// Heartbeat is late -- past the expected interval but not yet
    /// past the hard timeout. Worth watching, not yet worth acting on.
    Degraded,
    /// Heartbeat is past the hard timeout: the service should be
    /// considered down.
    Unhealthy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HealthPolicy {
    /// Beyond this, a late heartbeat is worth flagging but not yet
    /// treating as down.
    pub expected_interval: Duration,
    /// Beyond this, the service is considered down.
    pub timeout: Duration,
}

impl HealthPolicy {
    /// `None` if `timeout` isn't strictly longer than
    /// `expected_interval` -- a timeout that doesn't sit past the
    /// warning point makes the Degraded tier unreachable, which is
    /// almost certainly a misconfiguration rather than an intentional
    /// two-tier-collapsed policy, so this is rejected outright rather
    /// than silently accepted and producing a policy that can never
    /// actually report Degraded.
    pub fn new(expected_interval: Duration, timeout: Duration) -> Option<Self> {
        if timeout <= expected_interval {
            return None;
        }
        Some(Self { expected_interval, timeout })
    }

    pub fn assess(&self, now: Instant, last_heartbeat: Instant) -> HealthStatus {
        let elapsed = now.duration_since(last_heartbeat);
        if elapsed <= self.expected_interval {
            HealthStatus::Healthy
        } else if elapsed <= self.timeout {
            HealthStatus::Degraded
        } else {
            HealthStatus::Unhealthy
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn service(name: &str, deps: &[&str]) -> ServiceManifest {
        let mut manifest = ServiceManifest::new(name);
        for dep in deps {
            manifest = manifest.depends_on(*dep);
        }
        manifest
    }

    fn position(order: &[String], name: &str) -> usize {
        order.iter().position(|s| s.as_str() == name).unwrap()
    }

    #[test]
    fn empty_service_list_resolves_to_empty_order() {
        assert_eq!(resolve_start_order(&[]), Ok(vec![]));
    }

    #[test]
    fn independent_services_all_resolve_in_name_order() {
        let services = vec![service("c", &[]), service("a", &[]), service("b", &[])];
        assert_eq!(
            resolve_start_order(&services),
            Ok(vec!["a".to_string(), "b".to_string(), "c".to_string()])
        );
    }

    #[test]
    fn linear_chain_resolves_in_dependency_order() {
        let services = vec![service("c", &["b"]), service("a", &[]), service("b", &["a"])];
        assert_eq!(
            resolve_start_order(&services),
            Ok(vec!["a".to_string(), "b".to_string(), "c".to_string()])
        );
    }

    #[test]
    fn diamond_dependency_resolves_with_both_branches_before_the_join() {
        let services = vec![
            service("a", &[]),
            service("d", &["b", "c"]),
            service("b", &["a"]),
            service("c", &["a"]),
        ];
        let order = resolve_start_order(&services).unwrap();

        assert!(position(&order, "a") < position(&order, "b"));
        assert!(position(&order, "a") < position(&order, "c"));
        assert!(position(&order, "b") < position(&order, "d"));
        assert!(position(&order, "c") < position(&order, "d"));
    }

    #[test]
    fn direct_cycle_is_detected() {
        let services = vec![service("a", &["b"]), service("b", &["a"])];
        assert_eq!(
            resolve_start_order(&services),
            Err(ResolutionError::Cycle(vec!["a".to_string(), "b".to_string()]))
        );
    }

    #[test]
    fn self_dependency_is_detected_as_a_cycle() {
        let services = vec![service("a", &["a"])];
        assert_eq!(
            resolve_start_order(&services),
            Err(ResolutionError::Cycle(vec!["a".to_string()]))
        );
    }

    #[test]
    fn longer_cycle_is_detected() {
        // a -> b -> c -> a
        let services = vec![service("a", &["b"]), service("b", &["c"]), service("c", &["a"])];
        assert!(matches!(
            resolve_start_order(&services),
            Err(ResolutionError::Cycle(_))
        ));
    }

    #[test]
    fn dependency_on_an_unknown_service_is_reported_clearly() {
        let services = vec![service("a", &["ghost"])];
        assert_eq!(
            resolve_start_order(&services),
            Err(ResolutionError::UnknownDependency {
                service: "a".to_string(),
                missing: "ghost".to_string(),
            })
        );
    }

    #[test]
    fn unknown_dependency_is_reported_even_if_a_cycle_also_exists_elsewhere() {
        // Validation runs before cycle detection, so this reports the
        // missing dependency rather than attempting to resolve order
        // and reporting a cycle instead.
        let services = vec![service("a", &["b", "ghost"]), service("b", &["a"])];
        assert_eq!(
            resolve_start_order(&services),
            Err(ResolutionError::UnknownDependency {
                service: "a".to_string(),
                missing: "ghost".to_string(),
            })
        );
    }

    #[test]
    fn builder_accumulates_multiple_dependencies_in_order_added() {
        let manifest = ServiceManifest::new("x").depends_on("a").depends_on("b");
        assert_eq!(manifest.dependencies, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn restart_allowed_with_no_failure_history() {
        let policy = RestartPolicy::new(3, Duration::from_secs(60));
        assert!(policy.should_restart(Instant::now(), &[]));
    }

    #[test]
    fn restart_allowed_when_under_the_limit() {
        let policy = RestartPolicy::new(3, Duration::from_secs(60));
        let now = Instant::now();
        let failures = vec![now - Duration::from_secs(10), now - Duration::from_secs(20)];
        assert!(policy.should_restart(now, &failures));
    }

    #[test]
    fn restart_denied_once_the_limit_is_reached() {
        let policy = RestartPolicy::new(2, Duration::from_secs(60));
        let now = Instant::now();
        let failures = vec![now - Duration::from_secs(10), now - Duration::from_secs(20)];
        // Exactly at the limit -- the next failure would be the third
        // within the window, so this restart should already be denied.
        assert!(!policy.should_restart(now, &failures));
    }

    #[test]
    fn restart_denied_well_past_the_limit() {
        let policy = RestartPolicy::new(1, Duration::from_secs(60));
        let now = Instant::now();
        let failures = vec![
            now - Duration::from_secs(5),
            now - Duration::from_secs(10),
            now - Duration::from_secs(15),
        ];
        assert!(!policy.should_restart(now, &failures));
    }

    #[test]
    fn failures_outside_the_window_do_not_count() {
        let policy = RestartPolicy::new(1, Duration::from_secs(60));
        let now = Instant::now();
        // Both failures are older than the 60s window, so neither
        // counts against the budget -- a service stable for the last
        // minute gets a fresh allowance.
        let failures = vec![now - Duration::from_secs(120), now - Duration::from_secs(90)];
        assert!(policy.should_restart(now, &failures));
    }

    #[test]
    fn zero_max_failures_means_never_restart() {
        // A service explicitly configured to never auto-restart --
        // denied even with a completely empty failure history.
        let policy = RestartPolicy::new(0, Duration::from_secs(60));
        assert!(!policy.should_restart(Instant::now(), &[]));
    }

    #[test]
    fn failure_exactly_at_the_window_boundary_still_counts() {
        // duration_since <= window, not strictly less than -- a
        // failure exactly window-old is still within budget, not
        // treated as already expired.
        let policy = RestartPolicy::new(1, Duration::from_secs(60));
        let now = Instant::now();
        let failures = vec![now - Duration::from_secs(60)];
        assert!(!policy.should_restart(now, &failures));
    }

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
    fn health_policy_rejects_timeout_equal_to_expected_interval() {
        assert!(HealthPolicy::new(Duration::from_secs(30), Duration::from_secs(30)).is_none());
    }

    #[test]
    fn health_policy_rejects_timeout_shorter_than_expected_interval() {
        assert!(HealthPolicy::new(Duration::from_secs(30), Duration::from_secs(10)).is_none());
    }

    #[test]
    fn health_policy_accepts_a_properly_ordered_pair() {
        assert!(HealthPolicy::new(Duration::from_secs(10), Duration::from_secs(30)).is_some());
    }

    #[test]
    fn fresh_heartbeat_is_healthy() {
        let policy = HealthPolicy::new(Duration::from_secs(10), Duration::from_secs(30)).unwrap();
        let now = Instant::now();
        assert_eq!(policy.assess(now, now), HealthStatus::Healthy);
    }

    #[test]
    fn heartbeat_exactly_at_the_expected_interval_is_still_healthy() {
        let policy = HealthPolicy::new(Duration::from_secs(10), Duration::from_secs(30)).unwrap();
        let now = Instant::now();
        let last_heartbeat = now - Duration::from_secs(10);
        assert_eq!(policy.assess(now, last_heartbeat), HealthStatus::Healthy);
    }

    #[test]
    fn heartbeat_just_past_the_expected_interval_is_degraded() {
        let policy = HealthPolicy::new(Duration::from_secs(10), Duration::from_secs(30)).unwrap();
        let now = Instant::now();
        let last_heartbeat = now - Duration::from_secs(11);
        assert_eq!(policy.assess(now, last_heartbeat), HealthStatus::Degraded);
    }

    #[test]
    fn heartbeat_exactly_at_the_timeout_is_still_degraded_not_unhealthy() {
        let policy = HealthPolicy::new(Duration::from_secs(10), Duration::from_secs(30)).unwrap();
        let now = Instant::now();
        let last_heartbeat = now - Duration::from_secs(30);
        assert_eq!(policy.assess(now, last_heartbeat), HealthStatus::Degraded);
    }

    #[test]
    fn heartbeat_past_the_timeout_is_unhealthy() {
        let policy = HealthPolicy::new(Duration::from_secs(10), Duration::from_secs(30)).unwrap();
        let now = Instant::now();
        let last_heartbeat = now - Duration::from_secs(31);
        assert_eq!(policy.assess(now, last_heartbeat), HealthStatus::Unhealthy);
    }
}
