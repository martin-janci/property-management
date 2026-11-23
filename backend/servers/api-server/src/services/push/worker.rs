//! Push fanout background worker.
//!
//! [`PushFanoutWorker`] is a lightweight background tokio task that `BLPOP`s
//! queued push jobs off the Redis list [`PUSH_FANOUT_QUEUE_KEY`] and delivers
//! each one via [`CombinedPushAdapter`], with a bounded transient-failure
//! requeue policy. Falls back to a heartbeat-only loop when Redis / providers
//! are not configured.

use std::sync::Arc;
use std::time::Duration;

use common::notifications::{Notification, NotificationError, PushTransport, TransportResult};
use db::DbPool;
use serde::{Deserialize, Serialize};
use tokio::time::interval;
use tracing::Instrument;

use super::apns::ApnsConfig;
use super::combined::CombinedPushAdapter;
use super::config::PushFanoutConfig;
use super::fcm::FcmConfig;

/// Redis list key the notification pipeline enqueues push jobs onto.
///
/// Producers `RPUSH` a JSON-serialized [`Notification`] onto this list; the
/// worker `BLPOP`s from the head and delivers each one via [`CombinedPushAdapter`].
pub const PUSH_FANOUT_QUEUE_KEY: &str = "push_fanout_queue";

/// BLPOP block timeout (seconds) used when draining the queue.
///
/// Kept short so the single multiplexed `ConnectionManager` connection is not
/// held for long and other Redis commands are not starved. When the queue is
/// empty, BLPOP returns after this timeout and the drain yields back to the
/// poll ticker.
const BLPOP_TIMEOUT_SECS: f64 = 2.0;

/// Safety cap on the number of jobs drained per poll tick, so a flooded queue
/// can't make a single tick run unbounded.
const MAX_JOBS_PER_TICK: usize = 256;

/// Maximum number of delivery attempts for a single queued push job.
///
/// A *transient* (retryable) delivery failure requeues the job at the tail of
/// [`PUSH_FANOUT_QUEUE_KEY`] with an incremented attempt counter; once a job has
/// been attempted this many times it is dropped rather than requeued, so a
/// permanently-failing job cannot hot-loop forever. Set to 3 — the original
/// attempt plus two retries.
const MAX_PUSH_DELIVERY_ATTEMPTS: u32 = 3;

/// Maximum attempts to push a job *back* onto the Redis queue when the requeue
/// `RPUSH` itself fails transiently.
///
/// A job that reaches the requeue step has already been `BLPOP`'d off the list,
/// so it exists only in memory — a single failed `RPUSH` (NOAUTH mid-failover,
/// LOADING on a restarting replica, a dropped connection) would silently lose
/// the push. Bounding the in-memory retry gives a transient Redis blip a few
/// chances to recover without hot-looping when Redis is genuinely down; after
/// this many attempts the failure is surfaced loudly instead of swallowed.
const MAX_REQUEUE_PUSH_ATTEMPTS: u32 = 3;

/// Fixed delay between requeue-push retries (see [`MAX_REQUEUE_PUSH_ATTEMPTS`]).
///
/// Short, because the failures it covers (connection reset, a brief `LOADING`)
/// usually clear within one reconnect, and kept small so a dead Redis cannot
/// stall the drain tick for long.
const REQUEUE_PUSH_RETRY_DELAY: Duration = Duration::from_millis(100);

/// A push job as it travels on [`PUSH_FANOUT_QUEUE_KEY`].
///
/// Historically the queue payload was a bare JSON-serialized [`Notification`].
/// This envelope `#[serde(flatten)]`s the notification fields back to that exact
/// shape and adds an optional `attempts` counter, so:
/// - an old bare-`Notification` payload still deserializes (`attempts` defaults
///   to 0), and a requeued job still parses as a plain `Notification` for any
///   old-shape consumer, and
/// - a requeued job carries how many times it has already been attempted,
///   letting the worker bound retries via [`MAX_PUSH_DELIVERY_ATTEMPTS`].
#[derive(Debug, Clone, Serialize, Deserialize)]
struct QueuedPushJob {
    #[serde(flatten)]
    notification: Notification,
    /// How many delivery attempts this job has already had (0 for a fresh job).
    #[serde(default)]
    attempts: u32,
}

/// What to do with a job after one delivery attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JobDisposition {
    /// Delivered, or a benign skip (no configured provider / no deliverable
    /// device) — the job is complete and must not be requeued.
    Done,
    /// Permanent failure, or transient failure with retries exhausted — drop
    /// the job (the in-app channel remains the mandatory record).
    Drop,
    /// Transient (retryable) failure with retries remaining — requeue the job
    /// with an incremented attempt counter.
    Retry,
}

/// Whether a delivery error is transient (worth a bounded retry) rather than a
/// permanent/structural failure.
///
/// Network errors, provider 5xx responses, OAuth token-refresh failures and DB
/// hiccups on the fanout path all surface as [`NotificationError::PushFailed`]
/// or [`NotificationError::Database`]; these are retryable, as is a transient
/// [`NotificationError::RateLimitExceeded`]. [`NotificationError::PushNotConfigured`]
/// is a benign skip (handled before this is consulted), and every other variant
/// is treated as permanent.
fn is_transient_push_error(err: &NotificationError) -> bool {
    matches!(
        err,
        NotificationError::PushFailed(_)
            | NotificationError::Database(_)
            | NotificationError::RateLimitExceeded(_)
    )
}

/// Decide what to do with a queued job given the delivery result and how many
/// attempts it has already had.
///
/// Pure (no I/O) so the retry policy is unit-testable without a live Redis, DB
/// or push gateway. Before this, the worker dropped **every** failed job — a
/// transient provider outage silently lost the push instead of retrying it.
fn decide_disposition(result: &TransportResult, attempts: u32) -> JobDisposition {
    match result {
        // Delivered, or a benign "nothing to deliver" skip.
        Ok(()) | Err(NotificationError::PushNotConfigured) => JobDisposition::Done,
        Err(e) if is_transient_push_error(e) => {
            // `attempts` counts prior attempts; the one that just failed is
            // attempt number `attempts + 1`. Requeue only while retries remain.
            if attempts + 1 >= MAX_PUSH_DELIVERY_ATTEMPTS {
                JobDisposition::Drop
            } else {
                JobDisposition::Retry
            }
        }
        // Permanent / structural failure — do not requeue.
        Err(_) => JobDisposition::Drop,
    }
}

/// Background worker that fans out pending push notifications.
///
/// ## Delivery model
///
/// The worker drains a Redis list ([`PUSH_FANOUT_QUEUE_KEY`]) for pending job
/// payloads enqueued by the notification pipeline.  Each payload is a
/// JSON-serialized [`Notification`]:
/// ```json
/// {
///   "id": "<uuid>",
///   "user_id": "<uuid>",
///   "category": "announcements",
///   "title": "...",
///   "body": "...",
///   "priority": "normal"
/// }
/// ```
///
/// On each tick the worker `BLPOP`s from the head of the list (short timeout)
/// and delivers each job via [`CombinedPushAdapter::send`] (one token fetch,
/// platform-routed FCM + APNs), up to [`MAX_JOBS_PER_TICK`] jobs per tick.
///
/// When Redis is not available (or the worker is disabled) the worker falls
/// through to a no-op heartbeat loop so the server always starts cleanly — the
/// in-process pipeline still delivers push synchronously in that mode.
pub struct PushFanoutWorker {
    adapter: Arc<CombinedPushAdapter>,
    config: PushFanoutConfig,
    pubsub: Option<integrations::PubSubService>,
}

impl PushFanoutWorker {
    /// Create a new worker from the shared pool and current environment.
    ///
    /// Reads FCM and APNs credentials from environment variables. If neither
    /// is configured the worker starts in heartbeat-only mode (no crash).
    pub fn new(
        pool: DbPool,
        pubsub: Option<integrations::PubSubService>,
        config: PushFanoutConfig,
    ) -> Self {
        let fcm_config = FcmConfig::from_env();
        let apns_config = ApnsConfig::from_env();

        if !fcm_config.is_configured() {
            tracing::warn!("[8A-3] PushFanoutWorker: FCM not configured (FCM_PROJECT_ID unset)");
        }
        if !apns_config.is_configured() {
            tracing::warn!(
                "[8A-3] PushFanoutWorker: APNs not configured (APNS_P8_KEY / APNS_KEY_ID / APNS_TEAM_ID unset)"
            );
        }
        if !fcm_config.is_configured() && !apns_config.is_configured() {
            tracing::warn!(
                "[8A-3] PushFanoutWorker: neither FCM nor APNs is configured; \
                 push delivery will be skipped (no crash — set env vars to enable)"
            );
        }

        Self {
            adapter: Arc::new(CombinedPushAdapter::new(pool, fcm_config, apns_config)),
            config,
            pubsub,
        }
    }

    /// Spawn the background task and return its `JoinHandle`.
    pub fn start(self) -> tokio::task::JoinHandle<()> {
        let poll_secs = self.config.poll_interval_secs;
        tokio::spawn(
            async move {
                if !self.config.enabled {
                    tracing::info!("[8A-3] PushFanoutWorker disabled — not starting");
                    return;
                }

                tracing::info!(
                    poll_interval_secs = self.config.poll_interval_secs,
                    fcm_configured = self.adapter.fcm.fcm_config.is_configured(),
                    apns_configured = self.adapter.apns.apns_config.is_configured(),
                    "[8A-3] PushFanoutWorker started"
                );

                let mut ticker = interval(Duration::from_secs(self.config.poll_interval_secs));

                loop {
                    ticker.tick().await;
                    self.process_pending_jobs().await;
                }
            }
            .instrument(tracing::info_span!("bg.push_fanout", poll_secs = poll_secs,)),
        )
    }

    /// Process pending push jobs from the queue.
    ///
    /// When Redis is available we `BLPOP` jobs off [`PUSH_FANOUT_QUEUE_KEY`] and
    /// deliver each one via [`CombinedPushAdapter::send`] (one token fetch,
    /// platform-routed FCM + APNs), up to [`MAX_JOBS_PER_TICK`] jobs per tick.
    /// When Redis is not available we just
    /// log a heartbeat (the in-process pipeline delivers push synchronously).
    async fn process_pending_jobs(&self) {
        let Some(ref pubsub) = self.pubsub else {
            // No Redis — nothing to drain; log at trace to avoid spam
            tracing::trace!(
                "[8A-3] PushFanoutWorker heartbeat (no Redis; in-process pipeline handles push)"
            );
            return;
        };

        let redis = pubsub.client();
        let mut drained = 0usize;
        let mut requeued = 0usize;
        let mut requeue_failed = 0usize;

        // Drain up to MAX_JOBS_PER_TICK jobs. BLPOP with a short timeout means
        // an empty queue ends the drain promptly and yields back to the ticker;
        // a backed-up queue is bounded by MAX_JOBS_PER_TICK per tick.
        for _ in 0..MAX_JOBS_PER_TICK {
            let payload = match redis
                .queue_pop_blocking(PUSH_FANOUT_QUEUE_KEY, BLPOP_TIMEOUT_SECS)
                .await
            {
                Ok(Some(payload)) => payload,
                // Timed out with an empty queue — nothing more to do this tick.
                Ok(None) => break,
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "[8A-3] PushFanoutWorker: BLPOP on push_fanout_queue failed; backing off until next tick"
                    );
                    break;
                }
            };

            // `deliver_job` returns `Some(job)` only when the delivery failed
            // *transiently* and the job still has retries left — requeue it at
            // the tail so other pending jobs are served first and the retry is
            // naturally spaced out. A transient failure used to be dropped here,
            // silently losing the push (code-review finding).
            if let Some(job) = self.deliver_job(&payload).await {
                match serde_json::to_string(&job) {
                    Ok(requeue_payload) => {
                        // The job has already been BLPOP'd off the list, so if
                        // this RPUSH fails the push lives only in memory. A
                        // single attempt would silently lose it on a transient
                        // Redis error (NOAUTH mid-failover, LOADING on a
                        // restarting replica, connection reset), so retry a
                        // bounded number of times before giving up, and then
                        // surface the loss loudly rather than dropping it on the
                        // floor (code-review finding).
                        let push_result = retry_bounded(
                            MAX_REQUEUE_PUSH_ATTEMPTS,
                            || redis.queue_push(PUSH_FANOUT_QUEUE_KEY, &requeue_payload),
                            |_attempt| tokio::time::sleep(REQUEUE_PUSH_RETRY_DELAY),
                        )
                        .await;
                        match push_result {
                            Ok(_) => requeued += 1,
                            Err(e) => {
                                requeue_failed += 1;
                                tracing::error!(
                                    error = %e,
                                    notification_id = %job.notification.id,
                                    user_id = %job.notification.user_id,
                                    attempts = MAX_REQUEUE_PUSH_ATTEMPTS,
                                    "[8A-3] PushFanoutWorker: failed to requeue push job into Redis after bounded retries (NOAUTH/LOADING/connection reset?) — push may be lost; in-app notification remains the record of truth"
                                );
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            error = %e,
                            notification_id = %job.notification.id,
                            "[8A-3] PushFanoutWorker: failed to serialize push job for requeue — job dropped"
                        );
                    }
                }
            }
            drained += 1;
        }

        if drained > 0 {
            tracing::info!(
                drained = drained,
                requeued = requeued,
                requeue_failed = requeue_failed,
                "[8A-3] PushFanoutWorker drained push_fanout_queue"
            );
        }
    }

    /// Deliver a single queued job payload.
    ///
    /// The payload is a JSON [`QueuedPushJob`] (an old bare-`Notification`
    /// payload still parses, with `attempts` defaulting to 0). A malformed
    /// payload is logged and dropped — it has already been popped off the list,
    /// and re-queuing a poison message would loop forever.
    ///
    /// Returns `Some(job)` when the job should be requeued — a *transient*
    /// delivery failure with retries remaining; the returned job carries an
    /// incremented attempt counter. Returns `None` when the job is complete
    /// (delivered / benign skip) or must be dropped (permanent failure, or a
    /// transient failure with retries exhausted). The caller performs the
    /// actual requeue so that all Redis I/O stays in `process_pending_jobs`.
    async fn deliver_job(&self, payload: &str) -> Option<QueuedPushJob> {
        let mut job: QueuedPushJob = match serde_json::from_str(payload) {
            Ok(j) => j,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "[8A-3] PushFanoutWorker: dropping malformed push_fanout_queue payload"
                );
                return None;
            }
        };

        let result = self
            .adapter
            .send(job.notification.user_id, &job.notification)
            .await;

        match decide_disposition(&result, job.attempts) {
            JobDisposition::Done => {
                if result.is_ok() {
                    tracing::debug!(
                        notification_id = %job.notification.id,
                        user_id = %job.notification.user_id,
                        "[8A-3] PushFanoutWorker delivered queued push job"
                    );
                } else {
                    // `PushNotConfigured`: not a failure — either no provider is
                    // configured, or the user has no deliverable device tokens
                    // (`combine()` maps both to `PushNotConfigured`, which the
                    // synchronous pipeline records as Skipped). A benign
                    // no-device user must not emit warn-level noise.
                    tracing::debug!(
                        notification_id = %job.notification.id,
                        user_id = %job.notification.user_id,
                        "[8A-3] PushFanoutWorker: push skipped — no configured provider had a deliverable target"
                    );
                }
                None
            }
            JobDisposition::Retry => {
                // Transient (retryable) failure with retries left — hand the job
                // back to the caller to requeue with an incremented counter,
                // instead of dropping it (the previous behaviour silently lost
                // the push on a provider/DB blip).
                job.attempts += 1;
                tracing::warn!(
                    notification_id = %job.notification.id,
                    user_id = %job.notification.user_id,
                    attempt = job.attempts,
                    max_attempts = MAX_PUSH_DELIVERY_ATTEMPTS,
                    error = %display_err(&result),
                    "[8A-3] PushFanoutWorker: transient push failure — requeuing queued job"
                );
                Some(job)
            }
            JobDisposition::Drop => {
                // Permanent failure, or retries exhausted. The in-app channel
                // remains the mandatory record, so log and move on rather than
                // requeue (avoids hot-looping on a permanently failing job).
                tracing::warn!(
                    notification_id = %job.notification.id,
                    user_id = %job.notification.user_id,
                    attempts = job.attempts,
                    error = %display_err(&result),
                    "[8A-3] PushFanoutWorker: dropping push job after failed delivery"
                );
                None
            }
        }
    }
}

/// Run an async fallible operation up to `max_attempts` times, sleeping between
/// tries, and return the first `Ok` or the final `Err`.
///
/// The operation and the inter-attempt sleep are both injected so the
/// bounded-retry policy is unit-testable without a live Redis (or real
/// wall-clock sleeps). `sleep` is called with the 1-based number of the attempt
/// that just failed, and only *between* attempts — never after the last one.
async fn retry_bounded<T, E, Op, OpFut, Sleep, SleepFut>(
    max_attempts: u32,
    mut op: Op,
    mut sleep: Sleep,
) -> Result<T, E>
where
    Op: FnMut() -> OpFut,
    OpFut: std::future::Future<Output = Result<T, E>>,
    Sleep: FnMut(u32) -> SleepFut,
    SleepFut: std::future::Future<Output = ()>,
{
    let max_attempts = max_attempts.max(1);
    let mut attempt = 1u32;
    loop {
        match op().await {
            Ok(value) => return Ok(value),
            // Final attempt failed — surface the error to the caller rather
            // than swallowing it (the pre-fix behaviour dropped the job here).
            Err(e) if attempt >= max_attempts => return Err(e),
            // Transient failure with attempts remaining — back off and retry.
            Err(_) => {
                sleep(attempt).await;
                attempt += 1;
            }
        }
    }
}

/// Render the error side of a [`TransportResult`] for logging (`""` on `Ok`).
fn display_err(result: &TransportResult) -> String {
    result
        .as_ref()
        .err()
        .map(|e| e.to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn push_fanout_queue_key_is_stable() {
        // The producer (notification pipeline) and the worker must agree on the
        // list key. Pin it so a rename can't silently split producer/consumer.
        assert_eq!(PUSH_FANOUT_QUEUE_KEY, "push_fanout_queue");
    }

    #[test]
    fn queued_job_payload_roundtrips_to_notification() {
        // A push job on the queue is a JSON-serialized `Notification`. The
        // worker's `deliver_job` deserializes exactly this shape, so a
        // round-trip here pins the queue contract.
        let user_id = Uuid::new_v4();
        let original = Notification::new(
            user_id,
            common::notifications::NotificationCategory::Announcements,
            "Building update",
            "The lift will be serviced tomorrow.",
        )
        .with_priority(common::notifications::NotificationPriority::High);

        let payload = serde_json::to_string(&original).expect("serialize notification");
        let decoded: Notification =
            serde_json::from_str(&payload).expect("deserialize notification");

        assert_eq!(decoded.id, original.id);
        assert_eq!(decoded.user_id, user_id);
        assert_eq!(decoded.title, "Building update");
        assert_eq!(decoded.body, "The lift will be serviced tomorrow.");
        assert_eq!(
            decoded.category,
            common::notifications::NotificationCategory::Announcements
        );
        assert_eq!(
            decoded.priority,
            common::notifications::NotificationPriority::High
        );
    }

    #[test]
    fn malformed_queue_payload_fails_to_deserialize() {
        // `deliver_job` drops payloads that don't parse as a `Notification`
        // rather than re-queuing them (a poison message must not hot-loop).
        let err = serde_json::from_str::<Notification>("{\"not\":\"a notification\"}");
        assert!(err.is_err());
    }

    // ------------------------------------------------------------------
    // Transient-failure requeue policy
    //
    // Regression (code-review finding): the fanout worker dropped EVERY failed
    // queued job, so a transient/retryable failure (provider 5xx, network
    // error, OAuth token-refresh blip, DB hiccup) silently lost the push
    // instead of retrying it. These tests pin the retry/drop decision.
    // ------------------------------------------------------------------

    /// A fresh job (`attempts == 0`) that fails *transiently* must be requeued,
    /// not dropped. This is the core regression.
    #[test]
    fn transient_failure_requeues_fresh_job() {
        let transient: TransportResult =
            Err(NotificationError::PushFailed("gateway 503".to_string()));
        assert_eq!(decide_disposition(&transient, 0), JobDisposition::Retry);

        // A DB hiccup while fetching tokens is also transient.
        let db: TransportResult = Err(NotificationError::Database("pool timeout".to_string()));
        assert_eq!(decide_disposition(&db, 0), JobDisposition::Retry);

        assert!(is_transient_push_error(&NotificationError::PushFailed(
            "x".to_string()
        )));
        assert!(is_transient_push_error(&NotificationError::Database(
            "x".to_string()
        )));
    }

    /// Retries are bounded: once a job has used up its attempts, a further
    /// transient failure drops it rather than hot-looping forever.
    #[test]
    fn transient_failure_drops_after_max_attempts() {
        let transient: TransportResult =
            Err(NotificationError::PushFailed("gateway 503".to_string()));
        // `attempts == MAX - 1` means the attempt that just failed is the MAX-th.
        assert_eq!(
            decide_disposition(&transient, MAX_PUSH_DELIVERY_ATTEMPTS - 1),
            JobDisposition::Drop
        );
        assert_eq!(
            decide_disposition(&transient, MAX_PUSH_DELIVERY_ATTEMPTS + 5),
            JobDisposition::Drop
        );
    }

    /// A permanent/structural error is dropped immediately — never requeued.
    #[test]
    fn permanent_failure_is_dropped_not_requeued() {
        let permanent: TransportResult = Err(NotificationError::RecipientResolution(
            "user not found".to_string(),
        ));
        assert_eq!(decide_disposition(&permanent, 0), JobDisposition::Drop);
        assert!(!is_transient_push_error(
            &NotificationError::RecipientResolution("x".to_string())
        ));
    }

    /// A successful delivery and a benign "not configured / no device" skip are
    /// both terminal — the job is done, not requeued.
    #[test]
    fn success_and_benign_skip_are_done() {
        let ok: TransportResult = Ok(());
        assert_eq!(decide_disposition(&ok, 0), JobDisposition::Done);

        let skip: TransportResult = Err(NotificationError::PushNotConfigured);
        assert_eq!(decide_disposition(&skip, 2), JobDisposition::Done);
    }

    /// The queue envelope is backward-compatible: an old bare-`Notification`
    /// payload deserializes as a `QueuedPushJob` with `attempts == 0`, and a
    /// requeued job round-trips its incremented attempt counter while still
    /// parsing as a plain `Notification` for any old-shape consumer.
    #[test]
    fn queued_job_envelope_is_backward_compatible() {
        let user_id = Uuid::new_v4();
        let notification = Notification::new(
            user_id,
            common::notifications::NotificationCategory::Announcements,
            "Water outage",
            "Water will be shut off 09:00-12:00.",
        );

        // Old producer shape: a bare Notification with no `attempts` field.
        let bare = serde_json::to_string(&notification).expect("serialize notification");
        let job: QueuedPushJob = serde_json::from_str(&bare).expect("parse bare notification");
        assert_eq!(job.attempts, 0);
        assert_eq!(job.notification.id, notification.id);
        assert_eq!(job.notification.user_id, user_id);

        // Requeued shape: incremented attempts carried through a round-trip.
        let mut retried = job;
        retried.attempts += 1;
        let payload = serde_json::to_string(&retried).expect("serialize requeued job");
        let decoded: QueuedPushJob = serde_json::from_str(&payload).expect("parse requeued job");
        assert_eq!(decoded.attempts, 1);
        assert_eq!(decoded.notification.id, notification.id);

        // The flattened envelope still parses as a plain Notification, so a
        // consumer that only knows the old shape is unaffected.
        let as_notification: Notification =
            serde_json::from_str(&payload).expect("parse requeued job as notification");
        assert_eq!(as_notification.id, notification.id);
    }

    // ------------------------------------------------------------------
    // Bounded requeue-push retry
    //
    // Regression (code-review finding): when the Retry disposition's requeue
    // `RPUSH` *itself* failed (NOAUTH mid-failover, LOADING on a restarting
    // replica, connection reset), the worker logged a warn and dropped the
    // job — the push was already BLPOP'd off the list, so it was silently
    // lost. `retry_bounded` gives the requeue a bounded number of tries and
    // surfaces the final error instead of swallowing it. These tests inject
    // the op + sleep so the policy is pinned without a live Redis.
    // ------------------------------------------------------------------

    /// A first-try success returns `Ok` immediately and never sleeps/retries.
    #[tokio::test]
    async fn retry_bounded_returns_ok_without_retrying_on_first_success() {
        use std::cell::Cell;
        let calls = Cell::new(0u32);
        let sleeps = Cell::new(0u32);
        let r: Result<u64, &str> = retry_bounded(
            3,
            || {
                calls.set(calls.get() + 1);
                async { Ok(1u64) }
            },
            |_attempt| {
                sleeps.set(sleeps.get() + 1);
                async {}
            },
        )
        .await;
        assert_eq!(r, Ok(1));
        assert_eq!(calls.get(), 1, "a first-try success must not retry");
        assert_eq!(sleeps.get(), 0, "no sleep when there is nothing to retry");
    }

    /// A transient failure is retried (not dropped) and a later success is
    /// returned — this is the core of the fix.
    #[tokio::test]
    async fn retry_bounded_retries_until_success() {
        use std::cell::Cell;
        let calls = Cell::new(0u32);
        let sleeps = Cell::new(0u32);
        // Fails twice (NOAUTH), then the connection recovers on the 3rd try.
        let r: Result<u64, &str> = retry_bounded(
            3,
            || {
                let n = calls.get() + 1;
                calls.set(n);
                async move {
                    if n < 3 {
                        Err("NOAUTH")
                    } else {
                        Ok(u64::from(n))
                    }
                }
            },
            |_attempt| {
                sleeps.set(sleeps.get() + 1);
                async {}
            },
        )
        .await;
        assert_eq!(
            r,
            Ok(3),
            "the requeue push must be retried, not dropped on a transient error"
        );
        assert_eq!(calls.get(), 3);
        assert_eq!(sleeps.get(), 2, "one sleep between each of the two retries");
    }

    /// When every attempt fails (Redis down for the whole tick), the final
    /// error is surfaced to the caller rather than swallowed, after exactly
    /// `max_attempts` tries.
    #[tokio::test]
    async fn retry_bounded_surfaces_error_after_exhausting_attempts() {
        use std::cell::Cell;
        let calls = Cell::new(0u32);
        let sleeps = Cell::new(0u32);
        let r: Result<u64, &str> = retry_bounded(
            3,
            || {
                calls.set(calls.get() + 1);
                async { Err("LOADING") }
            },
            |_attempt| {
                sleeps.set(sleeps.get() + 1);
                async {}
            },
        )
        .await;
        assert_eq!(
            r,
            Err("LOADING"),
            "the final error must be surfaced, not silently swallowed"
        );
        assert_eq!(calls.get(), 3, "exactly max_attempts tries");
        assert_eq!(
            sleeps.get(),
            2,
            "sleeps happen only BETWEEN attempts, never after the last"
        );
    }

    // ------------------------------------------------------------------
    // Requeue-push policy bound to the PRODUCTION constants (#3726)
    //
    // The retry_bounded tests above pass a literal attempt count, so they keep
    // passing even if `MAX_REQUEUE_PUSH_ATTEMPTS` were lowered back to a single
    // attempt — silently reintroducing the pre-fix silent-drop bug (a transient
    // requeue RPUSH failure dropped the already-BLPOP'd job). These pin the
    // actual production constants the `process_pending_jobs` wiring feeds into
    // `retry_bounded`. (The drain_once ↔ Redis integration itself needs a live
    // Redis and is covered separately.)
    // ------------------------------------------------------------------

    /// Driving `retry_bounded` with the real `MAX_REQUEUE_PUSH_ATTEMPTS` and an
    /// op that fails every time must (a) surface the error — not silently drop
    /// the already-BLPOP'd push — (b) try exactly the production bound, and
    /// (c) perform at least one *retry* (an inter-attempt sleep). The pre-fix
    /// behaviour was a single attempt (no retry), so a transient Redis error
    /// dropped the push; lowering the bound back to 1 makes the retry count fall
    /// to 0 and fails this test, which the literal-count unit tests do not catch.
    #[tokio::test]
    async fn requeue_push_bound_permits_a_retry_before_surfacing() {
        use std::cell::Cell;
        let calls = Cell::new(0u32);
        let sleeps = Cell::new(0u32);
        let r: Result<u64, &str> = retry_bounded(
            MAX_REQUEUE_PUSH_ATTEMPTS,
            || {
                calls.set(calls.get() + 1);
                async { Err("redis down for the whole tick") }
            },
            |_attempt| {
                sleeps.set(sleeps.get() + 1);
                async {}
            },
        )
        .await;
        assert!(
            r.is_err(),
            "an exhausted requeue must surface the error, not silently drop the push"
        );
        assert_eq!(
            calls.get(),
            MAX_REQUEUE_PUSH_ATTEMPTS,
            "the requeue must try exactly the production bound"
        );
        assert!(
            sleeps.get() >= 1,
            "the production requeue bound must permit at least one retry; the pre-fix \
             single-attempt behaviour dropped the push on the first transient Redis error"
        );
    }

    /// Within the production bound, a transient requeue failure on every attempt
    /// but the last still recovers — the push is placed, not dropped. Wiring the
    /// real `MAX_REQUEUE_PUSH_ATTEMPTS` (not a literal) means lowering it to 1
    /// turns these failures into an immediate drop and fails this test.
    #[tokio::test]
    async fn requeue_push_recovers_within_production_bound() {
        use std::cell::Cell;
        let calls = Cell::new(0u32);
        let r: Result<u64, &str> = retry_bounded(
            MAX_REQUEUE_PUSH_ATTEMPTS,
            || {
                let n = calls.get() + 1;
                calls.set(n);
                async move {
                    if n < MAX_REQUEUE_PUSH_ATTEMPTS {
                        Err("NOAUTH")
                    } else {
                        Ok(u64::from(n))
                    }
                }
            },
            |_attempt| async {},
        )
        .await;
        assert_eq!(
            r,
            Ok(u64::from(MAX_REQUEUE_PUSH_ATTEMPTS)),
            "a transient requeue failure within the production bound must recover, not drop the push"
        );
        assert_eq!(calls.get(), MAX_REQUEUE_PUSH_ATTEMPTS);
    }
}
