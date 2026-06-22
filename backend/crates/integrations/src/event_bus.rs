//! Event bus with at-least-once handler dispatch and exponential-backoff retry.
//!
//! Story 2B.1 (GH epic #969 gap 2, BIT-178).
//!
//! The Redis pub/sub layer in [`crate::redis`] is fire-and-forget: a subscriber
//! that fails to process a message simply drops it. Story 2B.1 AC3 requires
//! failed events to be retried with exponential backoff before being given up
//! on. This module wraps [`PubSubService`] with a subscriber dispatch loop that,
//! on handler error, retries with exponential backoff (1s, 2s, 4s by default) up
//! to a capped number of retries, then dead-letters the event with structured
//! logging.
//!
//! Pub/sub delivery itself remains at-most-once across the wire; durable,
//! cross-restart redelivery would need an outbox/dead-letter table and is left
//! as a follow-up. The dead-letter [`broadcast`] stream exposed here is the
//! integration point for such durable persistence.

use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::broadcast;

use crate::redis::{CacheError, PubSubMessage, PubSubService};

/// Default number of retries (after the initial attempt) before dead-lettering.
///
/// With the default base delay this yields the 1s, 2s, 4s schedule from the
/// Story 2B.1 acceptance criteria.
pub const DEFAULT_MAX_RETRIES: u32 = 3;

/// Default base delay used for the first retry.
pub const DEFAULT_BASE_DELAY: Duration = Duration::from_secs(1);

/// Default upper bound on any single backoff delay.
pub const DEFAULT_MAX_DELAY: Duration = Duration::from_secs(30);

/// Error returned by an [`EventHandler`] when it fails to process a message.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct EventHandlerError(pub String);

impl EventHandlerError {
    /// Build a handler error from any displayable message.
    pub fn new(msg: impl Into<String>) -> Self {
        Self(msg.into())
    }
}

/// A consumer of pub/sub events.
///
/// Returning `Err` triggers the retry-with-backoff policy; once retries are
/// exhausted the event is dead-lettered.
#[async_trait]
pub trait EventHandler: Send + Sync {
    /// Process a single message. May be called multiple times for the same
    /// message when earlier attempts fail, so handlers should be idempotent.
    async fn handle(&self, message: &PubSubMessage) -> Result<(), EventHandlerError>;
}

/// Exponential backoff schedule: `base * 2^(retry-1)`, capped at `max_delay`,
/// allowing up to `max_retries` retries after the initial attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExponentialBackoff {
    /// Delay before the first retry.
    pub base: Duration,
    /// Upper bound on any single delay (caps the exponential growth).
    pub max_delay: Duration,
    /// Maximum number of retries after the initial attempt.
    pub max_retries: u32,
}

impl Default for ExponentialBackoff {
    fn default() -> Self {
        Self {
            base: DEFAULT_BASE_DELAY,
            max_delay: DEFAULT_MAX_DELAY,
            max_retries: DEFAULT_MAX_RETRIES,
        }
    }
}

impl ExponentialBackoff {
    /// Construct a custom backoff schedule.
    #[must_use]
    pub fn new(base: Duration, max_delay: Duration, max_retries: u32) -> Self {
        Self {
            base,
            max_delay,
            max_retries,
        }
    }

    /// Delay to wait before retry number `retry` (1-based).
    ///
    /// Returns `None` when `retry` is `0` or exceeds [`Self::max_retries`], i.e.
    /// when no further retry is permitted. Delay is `base * 2^(retry-1)`, capped
    /// at [`Self::max_delay`]; arithmetic overflow saturates to `max_delay`.
    #[must_use]
    pub fn delay_for_retry(&self, retry: u32) -> Option<Duration> {
        if retry == 0 || retry > self.max_retries {
            return None;
        }
        let factor = 2u64.checked_pow(retry - 1);
        let delay = match factor.and_then(|f| self.base.checked_mul_u64(f)) {
            Some(d) => d.min(self.max_delay),
            None => self.max_delay,
        };
        Some(delay)
    }

    /// The full delay schedule, one entry per retry, in order.
    #[must_use]
    pub fn schedule(&self) -> Vec<Duration> {
        (1..=self.max_retries)
            .filter_map(|r| self.delay_for_retry(r))
            .collect()
    }
}

/// Small extension so we can multiply a `Duration` by a `u64` factor with
/// saturation rather than panicking on overflow (std only offers `* u32`).
trait CheckedMulU64 {
    fn checked_mul_u64(self, factor: u64) -> Option<Duration>;
}

impl CheckedMulU64 for Duration {
    fn checked_mul_u64(self, factor: u64) -> Option<Duration> {
        self.as_nanos()
            .checked_mul(u128::from(factor))
            .and_then(|nanos| u64::try_from(nanos).ok())
            .map(Duration::from_nanos)
    }
}

/// An event that exhausted its retries and was given up on.
#[derive(Clone, Debug)]
pub struct DeadLetter {
    /// The message that could not be processed.
    pub message: PubSubMessage,
    /// Total number of handler attempts made (initial + retries).
    pub attempts: u32,
    /// The error from the final attempt.
    pub last_error: String,
}

/// Wraps a [`PubSubService`] with retrying, dead-lettering event dispatch.
#[derive(Clone)]
pub struct EventBus {
    pubsub: PubSubService,
    backoff: ExponentialBackoff,
}

impl EventBus {
    /// Create an event bus with the default backoff schedule.
    #[must_use]
    pub fn new(pubsub: PubSubService) -> Self {
        Self {
            pubsub,
            backoff: ExponentialBackoff::default(),
        }
    }

    /// Create an event bus with a custom backoff schedule.
    #[must_use]
    pub fn with_backoff(pubsub: PubSubService, backoff: ExponentialBackoff) -> Self {
        Self { pubsub, backoff }
    }

    /// Access the underlying pub/sub service (for publishing).
    #[must_use]
    pub fn pubsub(&self) -> &PubSubService {
        &self.pubsub
    }

    /// Subscribe to `channel` and dispatch each message to `handler`, retrying
    /// failed handling with exponential backoff before dead-lettering.
    ///
    /// Returns a [`broadcast::Receiver`] of [`DeadLetter`]s so callers can
    /// observe or durably persist events that exhausted their retries. Messages
    /// are processed sequentially: a message that is being retried delays
    /// subsequent messages on the same subscription, preserving order.
    pub async fn subscribe_with_handler<H>(
        &self,
        channel: &str,
        handler: H,
    ) -> Result<broadcast::Receiver<DeadLetter>, CacheError>
    where
        H: EventHandler + 'static,
    {
        let mut rx = self.pubsub.subscribe(channel).await?;
        let backoff = self.backoff;
        let (dlq_tx, dlq_rx) = broadcast::channel::<DeadLetter>(100);

        tokio::spawn(async move {
            while let Ok(message) = rx.recv().await {
                dispatch_with_retry(&handler, &message, backoff, &dlq_tx).await;
            }
        });

        Ok(dlq_rx)
    }
}

/// Dispatch a single message to `handler`, retrying on failure per `backoff`.
///
/// On exhaustion the message is dead-lettered (logged and sent to `dlq`). This
/// is the unit-testable core of the dispatch loop.
async fn dispatch_with_retry<H: EventHandler + ?Sized>(
    handler: &H,
    message: &PubSubMessage,
    backoff: ExponentialBackoff,
    dlq: &broadcast::Sender<DeadLetter>,
) {
    let mut retry = 0u32;
    loop {
        match handler.handle(message).await {
            Ok(()) => return,
            Err(err) => {
                if retry >= backoff.max_retries {
                    let attempts = retry + 1;
                    tracing::error!(
                        channel = %message.channel,
                        event_type = %message.event_type,
                        message_id = %message.id,
                        attempts,
                        error = %err,
                        "Event handler failed after exhausting retries; dead-lettering"
                    );
                    let _ = dlq.send(DeadLetter {
                        message: message.clone(),
                        attempts,
                        last_error: err.to_string(),
                    });
                    return;
                }

                retry += 1;
                // Safe: retry is in 1..=max_retries here.
                let delay = backoff.delay_for_retry(retry).unwrap_or(backoff.max_delay);
                tracing::warn!(
                    channel = %message.channel,
                    event_type = %message.event_type,
                    message_id = %message.id,
                    retry,
                    delay_ms = delay.as_millis() as u64,
                    error = %err,
                    "Event handler failed; retrying after backoff"
                );
                tokio::time::sleep(delay).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    fn test_message() -> PubSubMessage {
        PubSubMessage::new("test-channel", "test.event", serde_json::json!({"k": "v"}))
    }

    /// Handler that fails its first `fail_until` attempts, then succeeds.
    struct FlakyHandler {
        attempts: Arc<AtomicU32>,
        fail_until: u32,
    }

    #[async_trait]
    impl EventHandler for FlakyHandler {
        async fn handle(&self, _message: &PubSubMessage) -> Result<(), EventHandlerError> {
            let n = self.attempts.fetch_add(1, Ordering::SeqCst) + 1;
            if n <= self.fail_until {
                Err(EventHandlerError::new(format!("boom on attempt {n}")))
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn test_backoff_default_schedule_is_1_2_4_seconds() {
        let backoff = ExponentialBackoff::default();
        assert_eq!(
            backoff.schedule(),
            vec![
                Duration::from_secs(1),
                Duration::from_secs(2),
                Duration::from_secs(4),
            ]
        );
        // No retry past max_retries.
        assert_eq!(backoff.delay_for_retry(0), None);
        assert_eq!(backoff.delay_for_retry(4), None);
    }

    #[test]
    fn test_backoff_caps_at_max_delay() {
        let backoff = ExponentialBackoff::new(Duration::from_secs(1), Duration::from_secs(3), 5);
        // 1s, 2s, then capped at 3s, 3s, 3s.
        assert_eq!(
            backoff.schedule(),
            vec![
                Duration::from_secs(1),
                Duration::from_secs(2),
                Duration::from_secs(3),
                Duration::from_secs(3),
                Duration::from_secs(3),
            ]
        );
    }

    #[test]
    fn test_backoff_zero_retries_has_empty_schedule() {
        let backoff = ExponentialBackoff::new(Duration::from_secs(1), Duration::from_secs(30), 0);
        assert!(backoff.schedule().is_empty());
        assert_eq!(backoff.delay_for_retry(1), None);
    }

    #[tokio::test]
    async fn test_dispatch_succeeds_first_try_no_dead_letter() {
        let attempts = Arc::new(AtomicU32::new(0));
        let handler = FlakyHandler {
            attempts: attempts.clone(),
            fail_until: 0,
        };
        let (dlq_tx, mut dlq_rx) = broadcast::channel::<DeadLetter>(8);
        let backoff = ExponentialBackoff::new(Duration::ZERO, Duration::ZERO, 3);

        dispatch_with_retry(&handler, &test_message(), backoff, &dlq_tx).await;

        assert_eq!(attempts.load(Ordering::SeqCst), 1);
        assert!(dlq_rx.try_recv().is_err(), "no dead-letter expected");
    }

    #[tokio::test]
    async fn test_dispatch_retries_then_succeeds() {
        let attempts = Arc::new(AtomicU32::new(0));
        let handler = FlakyHandler {
            attempts: attempts.clone(),
            fail_until: 2, // fail attempts 1 and 2, succeed on 3
        };
        let (dlq_tx, mut dlq_rx) = broadcast::channel::<DeadLetter>(8);
        // Zero delays so the test runs instantly.
        let backoff = ExponentialBackoff::new(Duration::ZERO, Duration::ZERO, 3);

        dispatch_with_retry(&handler, &test_message(), backoff, &dlq_tx).await;

        assert_eq!(attempts.load(Ordering::SeqCst), 3);
        assert!(dlq_rx.try_recv().is_err(), "no dead-letter on eventual success");
    }

    #[tokio::test]
    async fn test_dispatch_dead_letters_after_max_retries() {
        let attempts = Arc::new(AtomicU32::new(0));
        let handler = FlakyHandler {
            attempts: attempts.clone(),
            fail_until: u32::MAX, // always fail
        };
        let (dlq_tx, mut dlq_rx) = broadcast::channel::<DeadLetter>(8);
        let backoff = ExponentialBackoff::new(Duration::ZERO, Duration::ZERO, 3);

        dispatch_with_retry(&handler, &test_message(), backoff, &dlq_tx).await;

        // Initial attempt + 3 retries = 4 invocations.
        assert_eq!(attempts.load(Ordering::SeqCst), 4);
        let dead = dlq_rx.try_recv().expect("expected a dead-letter");
        assert_eq!(dead.attempts, 4);
        assert_eq!(dead.message.event_type, "test.event");
        assert!(dead.last_error.contains("boom"));
    }
}
