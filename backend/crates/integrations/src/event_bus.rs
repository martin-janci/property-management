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
//! logging. Backoff sleeps are jittered by default ("full jitter": a uniform
//! random delay in `[0, base * 2^(retry-1)]`) so that a shared-downstream blip
//! does not make every subscriber and instance retry in lockstep (thundering
//! herd). The deterministic [`ExponentialBackoff::schedule`] still describes the
//! upper bound of each retry's delay.
//!
//! Pub/sub delivery itself remains at-most-once across the wire; durable,
//! cross-restart redelivery would need an outbox/dead-letter table and is left
//! as a follow-up. The dead-letter [`broadcast`] stream exposed here is the
//! integration point for such durable persistence.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::{broadcast, Semaphore};

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

/// Default cap on the number of messages being dispatched (and possibly sleeping
/// in backoff) concurrently per subscription.
///
/// Dispatch is fanned out across spawned tasks (see
/// [`EventBus::subscribe_with_handler`]) so one slow/retrying message no longer
/// head-of-line-blocks the whole channel; this semaphore bounds how many such
/// tasks run at once to avoid unbounded task growth under sustained failure.
pub const DEFAULT_MAX_IN_FLIGHT: usize = 16;

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
///
/// [`Self::delay_for_retry`] / [`Self::schedule`] return the deterministic
/// (un-jittered) delay so the schedule is easy to reason about and assert on.
/// The actual sleep performed between attempts is jittered when [`Self::jitter`]
/// is `true` (the default) — see [`Self::jittered`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExponentialBackoff {
    /// Delay before the first retry.
    pub base: Duration,
    /// Upper bound on any single delay (caps the exponential growth).
    pub max_delay: Duration,
    /// Maximum number of retries after the initial attempt.
    pub max_retries: u32,
    /// When `true` (default), apply full jitter to each sleep: a uniform random
    /// delay in `[0, delay_for_retry(retry)]` instead of the deterministic
    /// value. Spreads retries out so a recovered downstream is not hammered by
    /// every subscriber at once. Set to `false` for deterministic timing in
    /// tests.
    pub jitter: bool,
}

impl Default for ExponentialBackoff {
    fn default() -> Self {
        Self {
            base: DEFAULT_BASE_DELAY,
            max_delay: DEFAULT_MAX_DELAY,
            max_retries: DEFAULT_MAX_RETRIES,
            jitter: true,
        }
    }
}

impl ExponentialBackoff {
    /// Construct a custom backoff schedule with jitter enabled.
    #[must_use]
    pub fn new(base: Duration, max_delay: Duration, max_retries: u32) -> Self {
        Self {
            base,
            max_delay,
            max_retries,
            jitter: true,
        }
    }

    /// Construct a custom backoff schedule with jitter disabled (deterministic
    /// sleeps equal to [`Self::delay_for_retry`]). Primarily for tests.
    #[must_use]
    pub fn without_jitter(base: Duration, max_delay: Duration, max_retries: u32) -> Self {
        Self {
            base,
            max_delay,
            max_retries,
            jitter: false,
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

    /// Apply jitter to a deterministic backoff delay.
    ///
    /// With [`Self::jitter`] disabled this returns `base` unchanged. With jitter
    /// enabled it returns a uniform random delay in `[0, base]` ("full jitter"),
    /// so concurrent retriers spread their wake-ups out instead of synchronizing
    /// on a recovering downstream. Applied at sleep time only, so the
    /// deterministic [`Self::schedule`] (used by tests) is unaffected.
    #[must_use]
    pub fn jittered(&self, base: Duration) -> Duration {
        if !self.jitter || base.is_zero() {
            return base;
        }
        let max_nanos = u64::try_from(base.as_nanos()).unwrap_or(u64::MAX);
        Duration::from_nanos(rand::random_range(0..=max_nanos))
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
    max_in_flight: usize,
}

impl EventBus {
    /// Create an event bus with the default backoff schedule.
    #[must_use]
    pub fn new(pubsub: PubSubService) -> Self {
        Self {
            pubsub,
            backoff: ExponentialBackoff::default(),
            max_in_flight: DEFAULT_MAX_IN_FLIGHT,
        }
    }

    /// Create an event bus with a custom backoff schedule.
    #[must_use]
    pub fn with_backoff(pubsub: PubSubService, backoff: ExponentialBackoff) -> Self {
        Self {
            pubsub,
            backoff,
            max_in_flight: DEFAULT_MAX_IN_FLIGHT,
        }
    }

    /// Set the maximum number of messages dispatched concurrently per
    /// subscription (see [`DEFAULT_MAX_IN_FLIGHT`]). A value of `0` is treated
    /// as `1` (at least one in-flight dispatch).
    #[must_use]
    pub fn with_max_in_flight(mut self, max_in_flight: usize) -> Self {
        self.max_in_flight = max_in_flight.max(1);
        self
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
    /// observe or durably persist events that exhausted their retries.
    ///
    /// Dispatch is **concurrent, not strictly ordered** (#1765/#1792): each
    /// received message is handled in its own spawned task, bounded by a
    /// semaphore of [`Self::with_max_in_flight`] permits (default
    /// [`DEFAULT_MAX_IN_FLIGHT`]). This means a single slow or retrying message
    /// no longer head-of-line-blocks every later message on the channel while it
    /// sleeps through its backoff. The trade-off is that handlers may observe
    /// messages out of publish order; handlers are already required to be
    /// idempotent, and if strict per-key ordering is later needed it should be
    /// achieved by partitioning on an aggregate id rather than serializing the
    /// whole channel.
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
        let channel_name = channel.to_string();
        let (dlq_tx, dlq_rx) = broadcast::channel::<DeadLetter>(100);
        // Shared, ref-counted so each per-message dispatch task can hold it.
        let handler = Arc::new(handler);
        let semaphore = Arc::new(Semaphore::new(self.max_in_flight));

        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(message) => {
                        // Concurrent dispatch (#1765/#1792): handle each message
                        // in its own task so a slow/retrying handler does not
                        // stall later messages. Bounded by `semaphore` to cap
                        // in-flight work. `acquire_owned` only errors if the
                        // semaphore is closed, which we never do.
                        let Ok(permit) = semaphore.clone().acquire_owned().await else {
                            break;
                        };
                        let handler = handler.clone();
                        let dlq_tx = dlq_tx.clone();
                        tokio::spawn(async move {
                            dispatch_with_retry(handler.as_ref(), &message, backoff, &dlq_tx).await;
                            drop(permit);
                        });
                    }
                    // SECURITY/RELIABILITY (#1792): a `while let Ok(..)` exits on
                    // BOTH `Lagged` and `Closed`, so the first broadcast-buffer
                    // overflow silently and permanently terminated the
                    // subscription — defeating the at-least-once guarantee with
                    // no log, no dead-letter, no recovery. Handle `Lagged`
                    // explicitly: surface it loudly + as a synthetic dead-letter
                    // for monitoring, then KEEP the subscription alive.
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        tracing::error!(
                            channel = %channel_name,
                            skipped,
                            "Event subscription lagged; broadcast buffer overflowed and \
                             dropped messages (at-least-once violated for the gap)"
                        );
                        let _ = dlq_tx.send(DeadLetter {
                            message: PubSubMessage::new(
                                &channel_name,
                                "event_bus.lagged",
                                serde_json::json!({ "skipped_messages": skipped }),
                            ),
                            attempts: 0,
                            last_error: format!(
                                "subscription lagged: {skipped} message(s) dropped by \
                                 broadcast buffer overflow"
                            ),
                        });
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        tracing::debug!(
                            channel = %channel_name,
                            "Event subscription channel closed; subscriber task exiting"
                        );
                        break;
                    }
                }
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
                let scheduled = backoff.delay_for_retry(retry).unwrap_or(backoff.max_delay);
                // Full jitter (#1765/#1792): randomize the actual sleep within
                // `[0, scheduled]` so a shared-downstream blip does not make
                // every subscriber and instance retry in lockstep.
                let delay = backoff.jittered(scheduled);
                tracing::warn!(
                    channel = %message.channel,
                    event_type = %message.event_type,
                    message_id = %message.id,
                    retry,
                    scheduled_ms = scheduled.as_millis() as u64,
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
        assert!(
            dlq_rx.try_recv().is_err(),
            "no dead-letter on eventual success"
        );
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

    // ---- #1765 / #1792: jitter ----

    #[test]
    fn test_jittered_disabled_returns_base_unchanged() {
        let backoff =
            ExponentialBackoff::without_jitter(Duration::from_secs(1), Duration::from_secs(30), 3);
        assert!(!backoff.jitter);
        // No randomization: the sleep equals the deterministic schedule entry.
        let base = Duration::from_secs(4);
        for _ in 0..100 {
            assert_eq!(backoff.jittered(base), base);
        }
    }

    #[test]
    fn test_jittered_enabled_stays_within_bounds_and_varies() {
        let backoff = ExponentialBackoff::default();
        assert!(backoff.jitter);
        let base = Duration::from_secs(4);

        let mut seen = std::collections::HashSet::new();
        let mut saw_below_base = false;
        for _ in 0..1_000 {
            let d = backoff.jittered(base);
            // Full jitter: always within [0, base].
            assert!(d <= base, "jittered delay {d:?} exceeded base {base:?}");
            if d < base {
                saw_below_base = true;
            }
            seen.insert(d.as_nanos());
        }
        // Randomized: not a constant, and (almost surely) not always exactly base.
        assert!(
            seen.len() > 1,
            "jitter produced a single constant value across 1000 samples"
        );
        assert!(
            saw_below_base,
            "jitter never produced a value below base — not actually jittering"
        );

        // Zero base is a no-op even with jitter enabled (avoids a 0..=0 sample).
        assert_eq!(backoff.jittered(Duration::ZERO), Duration::ZERO);
    }

    // ---- #1765 / #1792: head-of-line blocking ----

    /// Handler that blocks forever on messages whose event_type is "slow"
    /// (until `release` is notified) and immediately reports completion of any
    /// "fast" message via `fast_done`. Lets a test prove that a stuck message
    /// does NOT prevent a later message from being processed.
    struct GatedHandler {
        release: Arc<tokio::sync::Notify>,
        fast_done: tokio::sync::mpsc::UnboundedSender<()>,
    }

    #[async_trait]
    impl EventHandler for GatedHandler {
        async fn handle(&self, message: &PubSubMessage) -> Result<(), EventHandlerError> {
            if message.event_type == "slow" {
                // Block until the test explicitly releases us.
                self.release.notified().await;
            } else {
                let _ = self.fast_done.send(());
            }
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_concurrent_dispatch_does_not_head_of_line_block() {
        // Shared in-memory broker so a separate publisher instance's messages
        // are not self-filtered by the subscriber instance.
        let broker = Arc::new(crate::redis::InMemoryBroker::new());
        let publisher = PubSubService::with_broker(broker.clone());
        let bus = EventBus::new(PubSubService::with_broker(broker.clone()));

        let release = Arc::new(tokio::sync::Notify::new());
        let (fast_tx, mut fast_rx) = tokio::sync::mpsc::unbounded_channel();
        let handler = GatedHandler {
            release: release.clone(),
            fast_done: fast_tx,
        };

        let _dlq = bus
            .subscribe_with_handler("hol-test", handler)
            .await
            .expect("subscribe");

        // Give the in-memory subscribe forwarder task a moment to wire up before
        // publishing (broadcast has no replay for messages sent pre-subscribe).
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Publish the blocking message FIRST, then the fast one.
        publisher
            .publish(
                "hol-test",
                PubSubMessage::new("hol-test", "slow", serde_json::json!({})),
            )
            .await
            .expect("publish slow");
        publisher
            .publish(
                "hol-test",
                PubSubMessage::new("hol-test", "fast", serde_json::json!({})),
            )
            .await
            .expect("publish fast");

        // The fast message must complete WHILE the slow one is still blocked.
        // On the old sequential loop the slow dispatch blocks the recv loop, so
        // the fast handler never runs and this times out.
        let got_fast = tokio::time::timeout(Duration::from_secs(5), fast_rx.recv()).await;
        assert!(
            matches!(got_fast, Ok(Some(()))),
            "fast message was not processed while a prior message was blocked — \
             dispatch is head-of-line blocking"
        );

        // Unblock the slow handler so the spawned task can finish cleanly.
        release.notify_one();
    }
}
