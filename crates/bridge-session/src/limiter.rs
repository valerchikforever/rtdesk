//! Online-guessing protection for one-time codes (SR-04).
//! Exponential backoff after each failure; every 5th failure forces a new code.

use std::time::{Duration, Instant};

pub const ROTATE_EVERY: u32 = 5;
pub const MAX_DELAY: Duration = Duration::from_secs(15 * 60);

#[derive(Debug, Default)]
pub struct AuthLimiter {
    failures: u32,
    locked_until: Option<Instant>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum FailureOutcome {
    Continue { retry_after: Duration },
    RotateSecret { retry_after: Duration },
}

impl AuthLimiter {
    pub fn new() -> Self {
        Self::default()
    }

    /// `Err(remaining)` while locked out.
    pub fn check(&self, now: Instant) -> Result<(), Duration> {
        match self.locked_until {
            Some(t) if t > now => Err(t - now),
            _ => Ok(()),
        }
    }

    pub fn record_failure(&mut self, now: Instant) -> FailureOutcome {
        self.failures = self.failures.saturating_add(1);
        let exp = self.failures.saturating_sub(1).min(20);
        let delay = Duration::from_secs(1u64 << exp).min(MAX_DELAY);
        self.locked_until = Some(now + delay);
        if self.failures.is_multiple_of(ROTATE_EVERY) {
            FailureOutcome::RotateSecret { retry_after: delay }
        } else {
            FailureOutcome::Continue { retry_after: delay }
        }
    }

    pub fn record_success(&mut self) {
        self.failures = 0;
        self.locked_until = None;
    }

    pub fn failures(&self) -> u32 {
        self.failures
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_and_rotation() {
        let mut l = AuthLimiter::new();
        let t0 = Instant::now();
        assert!(l.check(t0).is_ok());
        assert_eq!(l.record_failure(t0), FailureOutcome::Continue { retry_after: Duration::from_secs(1) });
        assert!(l.check(t0).is_err());
        assert!(l.check(t0 + Duration::from_secs(2)).is_ok());
        for _ in 0..3 {
            l.record_failure(t0);
        }
        assert!(matches!(l.record_failure(t0), FailureOutcome::RotateSecret { .. }));
        for _ in 0..40 {
            l.record_failure(t0);
        }
        assert_eq!(l.check(t0), Err(MAX_DELAY));
        l.record_success();
        assert!(l.check(t0).is_ok());
    }
}
