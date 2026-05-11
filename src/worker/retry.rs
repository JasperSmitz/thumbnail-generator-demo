use chrono::{DateTime, Duration, Utc};

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    max_attempts: u32,
}

impl RetryPolicy {
    pub fn new(max_attempts: u32) -> Self {
        Self { max_attempts }
    }

    pub fn next_retry_at(
        &self,
        attempts_after_failure: u32,
        now: DateTime<Utc>,
    ) -> Option<DateTime<Utc>> {
        if attempts_after_failure >= self.max_attempts {
            return None;
        }

        Some(now + Duration::seconds(self.backoff_seconds(attempts_after_failure)))
    }

    pub fn backoff_seconds(&self, attempts_after_failure: u32) -> i64 {
        match attempts_after_failure {
            0 => 0,
            1 => 10,
            2 => 30,
            3 => 120,
            4 => 600,
            _ => 1800,
        }
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::worker::retry::RetryPolicy;

    #[test]
    fn zero_attempts_has_no_delay() {
        let policy = RetryPolicy::new(3);

        assert_eq!(policy.backoff_seconds(0), 0);
    }

    #[test]
    fn retry_policy_uses_expected_backoff_table() {
        let policy = RetryPolicy::new(10);

        assert_eq!(policy.backoff_seconds(1), 10);
        assert_eq!(policy.backoff_seconds(2), 30);
        assert_eq!(policy.backoff_seconds(3), 120);
        assert_eq!(policy.backoff_seconds(4), 600);
        assert_eq!(policy.backoff_seconds(5), 1800);
        assert_eq!(policy.backoff_seconds(99), 1800);
    }

    #[test]
    fn retry_policy_allows_retry_before_max_attempts() {
        let now = Utc::now();
        let policy = RetryPolicy::new(3);

        let next_retry_at = policy.next_retry_at(2, now);

        assert_eq!(
            next_retry_at.map(|value| (value - now).num_seconds()),
            Some(30)
        );
    }

    #[test]
    fn first_failure_retries_after_10_seconds() {
        let now = Utc::now();
        let policy = RetryPolicy::new(3);

        let next_retry_at = policy.next_retry_at(1, now);

        assert_eq!(
            next_retry_at.map(|value| (value - now).num_seconds()),
            Some(10)
        );
    }

    #[test]
    fn second_failure_retries_after_30_seconds() {
        let now = Utc::now();
        let policy = RetryPolicy::new(3);

        let next_retry_at = policy.next_retry_at(2, now);

        assert_eq!(
            next_retry_at.map(|value| (value - now).num_seconds()),
            Some(30)
        );
    }

    #[test]
    fn fourth_failure_retries_after_600_seconds_when_still_allowed() {
        let now = Utc::now();
        let policy = RetryPolicy::new(10);

        let next_retry_at = policy.next_retry_at(4, now);

        assert_eq!(
            next_retry_at.map(|value| (value - now).num_seconds()),
            Some(600)
        );
    }

    #[test]
    fn fifth_and_later_failures_use_long_backoff_when_still_allowed() {
        let now = Utc::now();
        let policy = RetryPolicy::new(10);

        let next_retry_at = policy.next_retry_at(5, now);

        assert_eq!(
            next_retry_at.map(|value| (value - now).num_seconds()),
            Some(1800)
        );
    }

    #[test]
    fn retry_policy_returns_none_when_max_attempts_reached() {
        let now = Utc::now();
        let policy = RetryPolicy::new(3);

        let next_retry_at = policy.next_retry_at(3, now);

        assert_eq!(next_retry_at, None);
    }

    #[test]
    fn max_attempts_prevents_retry_even_if_backoff_exists() {
        let now = Utc::now();
        let policy = RetryPolicy::new(5);

        let next_retry_at = policy.next_retry_at(5, now);

        assert_eq!(next_retry_at, None);
    }
}
