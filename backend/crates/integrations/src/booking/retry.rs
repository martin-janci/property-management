//! Retry configuration and push-outcome types for the Booking.com client
//! (AC-5 rate/availability push).
//!
//! Extracted verbatim from `booking/mod.rs` as a pure module split — no
//! behaviour change. The bounded exponential-backoff policy
//! ([`BookingRetryConfig`]), the combined per-stream push result
//! ([`PushOutcome`]) and the retryable-status predicate
//! ([`is_retryable_status`]) live here; the [`super::BookingClient`] that drives
//! them stays in the parent module.

/// Retry policy for OTA rate & availability *push* operations.
///
/// Booking.com's Supply XML endpoint is occasionally transient-fail under
/// load (HTTP 429/5xx) and the OTA push messages are idempotent (each carries
/// an absolute `Start`/`End`/`InvTypeCode` application control rather than a
/// delta), so re-sending the same document is safe. This struct controls the
/// bounded exponential-backoff retry loop used by [`super::BookingClient::push_rates`]
/// and [`super::BookingClient::push_availability`].
///
/// This is intentionally a small, push-specific policy distinct from
/// [`crate::connector::RetryConfig`], which drives the generic [`crate::connector::Connector`]
/// request/response abstraction. The Booking client talks to `reqwest`
/// directly (it must hand-roll OTA XML bodies + Basic auth), so it carries its
/// own lightweight policy rather than routing through the generic connector.
#[derive(Debug, Clone)]
pub struct BookingRetryConfig {
    /// Total number of attempts (1 = no retry). Must be >= 1.
    pub max_attempts: u32,
    /// Delay before the first retry, in milliseconds.
    pub initial_delay_ms: u64,
    /// Upper bound on any single backoff delay, in milliseconds.
    pub max_delay_ms: u64,
    /// Multiplier applied to the delay after each failed attempt.
    pub backoff_multiplier: u32,
}

impl Default for BookingRetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 500,
            max_delay_ms: 8_000,
            backoff_multiplier: 2,
        }
    }
}

impl BookingRetryConfig {
    /// A no-retry policy (single attempt) — useful in tests to keep them fast.
    pub fn no_retry() -> Self {
        Self {
            max_attempts: 1,
            initial_delay_ms: 0,
            max_delay_ms: 0,
            backoff_multiplier: 1,
        }
    }

    /// Return a copy of this policy with `max_attempts` overridden, keeping the
    /// other (delay/backoff) parameters. Handy for zero-delay multi-attempt
    /// retry configs in tests (`no_retry().clone_with_attempts(3)`).
    pub fn clone_with_attempts(&self, max_attempts: u32) -> Self {
        Self {
            max_attempts: max_attempts.max(1),
            ..self.clone()
        }
    }

    /// Backoff delay (ms) before the retry that follows `attempt` (0-based:
    /// `attempt = 0` is the delay after the first attempt failed). Capped at
    /// [`Self::max_delay_ms`].
    pub fn delay_for_attempt(&self, attempt: u32) -> u64 {
        let factor = self.backoff_multiplier.saturating_pow(attempt);
        self.initial_delay_ms
            .saturating_mul(factor as u64)
            .min(self.max_delay_ms)
    }

    /// Effective delay (ms) to wait before the retry that follows the failed
    /// attempt indexed by `prev_attempt` (0-based, same convention as
    /// [`Self::delay_for_attempt`]), combining our exponential backoff with an
    /// optional server-supplied `Retry-After` hint.
    ///
    /// `retry_after_ms` is the hint already converted to milliseconds and
    /// capped by the caller. The larger of the two values wins: we never retry
    /// sooner than the server explicitly asked for (`Retry-After`), and never
    /// sooner than our own backoff schedule would allow. When no hint is
    /// present this is exactly [`Self::delay_for_attempt`].
    ///
    /// Extracted from the inline logic in
    /// [`super::BookingClient::post_ota_with_retry`] so the delay computation is
    /// unit-testable without spinning up a mock server and real timers.
    pub fn effective_delay_ms(&self, prev_attempt: u32, retry_after_ms: Option<u64>) -> u64 {
        let base = self.delay_for_attempt(prev_attempt);
        match retry_after_ms {
            Some(hint) => hint.max(base),
            None => base,
        }
    }
}

/// Outcome of a combined availability + rate *push* sync to Booking.com
/// (AC-5).
///
/// The two OTA streams — `OTA_HotelAvailNotifRQ` (availability) and
/// `OTA_HotelRateAmountNotifRQ` (rates) — are pushed independently and can
/// succeed or fail independently. Collapsing them into a single
/// `Result<(), _>` loses that distinction: a caller could not tell whether an
/// error meant "nothing was applied" or "availability landed but rates were
/// rejected", leaving the channel in a half-synced state with no signal.
///
/// [`super::BookingClient::push_availability_and_rates`] returns this struct so the
/// caller sees exactly which stream applied, how many messages each carried,
/// and the per-stream error text when one failed. Each stream still runs
/// through [`super::BookingClient::post_ota_with_retry`], so transient failures are
/// retried with the same bounded backoff + idempotency-token semantics as the
/// single-stream pushes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PushOutcome {
    /// Number of availability messages sent (0 when none were supplied).
    pub availability_pushed: usize,
    /// Number of rate messages sent (0 when none were supplied).
    pub rates_pushed: usize,
    /// `Some(error)` if the availability push was attempted and failed;
    /// `None` if it succeeded or was skipped (no availability updates).
    pub availability_error: Option<String>,
    /// `Some(error)` if the rate push was attempted and failed; `None` if it
    /// succeeded or was skipped (no rate updates).
    pub rates_error: Option<String>,
}

impl PushOutcome {
    /// True when neither stream reported an error (a fully-applied or no-op
    /// sync).
    pub fn is_success(&self) -> bool {
        self.availability_error.is_none() && self.rates_error.is_none()
    }

    /// True when exactly one of the two streams failed while the other
    /// applied — the channel is now in a half-synced state the caller must
    /// reconcile.
    pub fn is_partial(&self) -> bool {
        self.availability_error.is_some() != self.rates_error.is_some()
    }
}

/// Whether an HTTP status code returned by the Booking.com endpoint is worth
/// retrying. Transient server-side / throttling failures are retryable;
/// client errors (4xx other than 408/429) are not — they indicate a bad
/// message and will fail identically on retry.
pub(super) fn is_retryable_status(status: u16) -> bool {
    matches!(status, 408 | 425 | 429 | 500 | 502 | 503 | 504)
}
