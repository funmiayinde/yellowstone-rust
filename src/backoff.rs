use std::time::Duration;

/// Reconnection delay strategy.
///
/// Implement this trait to add custom backoff behaviour.
pub trait BackoffStrategy: Send + Sync + std::fmt::Debug {
    /// Return the delay before attempt number `attempt` (0-indexed).
    fn next_delay(&self, attempt: u32) -> Duration;

    /// Reset internal state after a successful connection.
    fn reset(&self);
}

// ── Exponential backoff with full jitter ──────────────────────────────────
// AWS "Full Jitter" variant: delay = rand(0, min(cap, base * 2^attempt))
// Produces the best load distribution across reconnect storms.

/// Exponential backoff with full jitter. The default reconnection strategy.
#[derive(Debug, Clone)]
pub struct ExponentialBackoff {
    /// Base delay. Default: 100ms
    pub base: Duration,
    /// Maximum delay cap. Default: 30s
    pub max: Duration,
    /// Additional random jitter ceiling. Default: 500ms
    pub jitter_max: Duration,
}

impl Default for ExponentialBackoff {
    fn default() -> Self {
        Self {
            base:       Duration::from_millis(100),
            max:        Duration::from_secs(30),
            jitter_max: Duration::from_millis(500),
        }
    }
}

impl ExponentialBackoff {
    pub fn new(base: Duration, max: Duration, jitter_max: Duration) -> Self {
        Self { base, max, jitter_max }
    }
}

impl BackoffStrategy for ExponentialBackoff {
    fn next_delay(&self, attempt: u32) -> Duration {
        use rand::Rng;
        let exp    = self.base.saturating_mul(2u32.saturating_pow(attempt));
        let capped = exp.min(self.max);
        let jitter = rand::thread_rng().gen_range(Duration::ZERO..=self.jitter_max);
        capped + jitter
    }

    fn reset(&self) {}
}

// ── Constant backoff ──────────────────────────────────────────────────────

/// Fixed delay between every reconnect attempt. Useful in controlled environments.
#[derive(Debug, Clone)]
pub struct ConstantBackoff {
    pub delay: Duration,
}

impl ConstantBackoff {
    pub fn new(delay: Duration) -> Self { Self { delay } }
}

impl BackoffStrategy for ConstantBackoff {
    fn next_delay(&self, _attempt: u32) -> Duration { self.delay }
    fn reset(&self) {}
}

// ── Immediate reconnect (tests only) ─────────────────────────────────────

/// No delay — reconnect immediately.  Only suitable for unit tests.
#[derive(Debug, Clone, Default)]
pub struct ImmediateReconnect;

impl BackoffStrategy for ImmediateReconnect {
    fn next_delay(&self, _attempt: u32) -> Duration { Duration::ZERO }
    fn reset(&self) {}
}
