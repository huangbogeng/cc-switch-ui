use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    failure_threshold: u32,
    reset_timeout: Duration,
    state: CircuitState,
    consecutive_failures: u32,
    opened_at: Option<Instant>,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, reset_timeout: Duration) -> Self {
        Self {
            failure_threshold: failure_threshold.max(1),
            reset_timeout,
            state: CircuitState::Closed,
            consecutive_failures: 0,
            opened_at: None,
        }
    }

    pub fn state(&self) -> CircuitState {
        self.state
    }

    pub fn consecutive_failures(&self) -> u32 {
        self.consecutive_failures
    }

    pub fn allow_request(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::HalfOpen => true,
            CircuitState::Open => {
                let elapsed = self
                    .opened_at
                    .map(|opened_at| opened_at.elapsed())
                    .unwrap_or_default();
                if elapsed >= self.reset_timeout {
                    self.state = CircuitState::HalfOpen;
                    true
                } else {
                    false
                }
            }
        }
    }

    pub fn record_success(&mut self) {
        self.consecutive_failures = 0;
        self.state = CircuitState::Closed;
        self.opened_at = None;
    }

    pub fn record_failure(&mut self) {
        match self.state {
            CircuitState::HalfOpen => {
                self.state = CircuitState::Open;
                self.opened_at = Some(Instant::now());
                self.consecutive_failures = self.failure_threshold;
            }
            CircuitState::Closed | CircuitState::Open => {
                self.consecutive_failures = self.consecutive_failures.saturating_add(1);
                if self.consecutive_failures >= self.failure_threshold {
                    self.state = CircuitState::Open;
                    self.opened_at = Some(Instant::now());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_after_reaching_failure_threshold() {
        let mut breaker = CircuitBreaker::new(2, Duration::from_secs(30));
        assert_eq!(breaker.state(), CircuitState::Closed);

        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Closed);

        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);
        assert!(!breaker.allow_request());
    }

    #[test]
    fn open_transitions_to_half_open_after_timeout() {
        let mut breaker = CircuitBreaker::new(1, Duration::from_millis(0));
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);

        assert!(breaker.allow_request());
        assert_eq!(breaker.state(), CircuitState::HalfOpen);
    }

    #[test]
    fn half_open_success_closes_breaker() {
        let mut breaker = CircuitBreaker::new(1, Duration::from_millis(0));
        breaker.record_failure();
        assert!(breaker.allow_request());
        assert_eq!(breaker.state(), CircuitState::HalfOpen);

        breaker.record_success();
        assert_eq!(breaker.state(), CircuitState::Closed);
        assert!(breaker.allow_request());
    }

    #[test]
    fn half_open_failure_reopens_breaker() {
        let mut breaker = CircuitBreaker::new(1, Duration::from_millis(0));
        breaker.record_failure();
        assert!(breaker.allow_request());
        assert_eq!(breaker.state(), CircuitState::HalfOpen);

        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);
    }
}
