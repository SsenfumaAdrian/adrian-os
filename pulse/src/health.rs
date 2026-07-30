//! Heartbeat-recency-based health assessment ("health supervision"
//! from Pulse's README).

use std::time::{Duration, Instant};

/// A policy for judging service health from heartbeat recency.
/// Doesn't send or receive any real heartbeat -- that needs a real
/// running service to report one -- only judges recency against two
/// thresholds, given a last-heartbeat time handed to it. Genuinely
/// distinct in shape from restart::RestartPolicy despite both being
/// Duration-window logic: RestartPolicy judges frequency across a
/// history of past events, this judges recency of a single most-
/// recent signal.
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
