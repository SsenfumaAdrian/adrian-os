//! Crash-loop backoff decisions ("restart policy" from Pulse's
//! README).

use std::time::{Duration, Instant};

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
