//! Combined push adapter — platform-routes FCM + APNs off a single token fetch.
//!
//! [`CombinedPushAdapter`] fetches a user's device tokens once, hands the shared
//! slice to both [`FcmHttpAdapter`](super::fcm::FcmHttpAdapter) and
//! [`ApnsHttpAdapter`](super::apns::ApnsHttpAdapter), and combines their
//! [`ProviderOutcome`]s into an honest pipeline result (Sent / Failed / Skipped).

use async_trait::async_trait;
use common::notifications::{Notification, NotificationError, PushTransport, TransportResult};
use db::{models::DevicePushToken, repositories::DevicePushTokenRepository, DbPool};
use uuid::Uuid;

use super::apns::{ApnsConfig, ApnsHttpAdapter};
use super::fcm::{FcmConfig, FcmHttpAdapter};

// ============================================================================
// CombinedPushAdapter — routes FCM and APNs by platform in one pass
// ============================================================================

/// Internal, richer-than-`Result` outcome of one provider's delivery pass.
///
/// The `PushTransport` trait can only return `Ok(())` / `Err(_)`, which
/// conflates "I actually delivered a push" with "I was configured but had no
/// tokens for my platform". [`CombinedPushAdapter`] needs that distinction to
/// keep delivery metrics honest: if *neither* provider performed a real
/// delivery, the pipeline must record the push as **Skipped**, not **Sent**
/// (the inflated-`sent` lie Issue #484 removed — it re-surfaced for iOS-only
/// users when FCM was configured but APNs was not).
pub(crate) enum ProviderOutcome {
    /// At least one device accepted the push.
    Delivered,
    /// Provider is configured, but the user has no tokens for this platform —
    /// nothing was delivered and nothing failed.
    NoTargets,
    /// Provider credentials are not configured; it attempted nothing.
    NotConfigured,
    /// Provider attempted delivery and every attempt failed.
    Failed(NotificationError),
}

impl ProviderOutcome {
    /// Collapse to the `PushTransport` trait's `Result`, used when a single
    /// adapter is driven directly (not through [`CombinedPushAdapter`]).
    /// `NoTargets` maps to `Ok(())` — a configured provider with no devices for
    /// its platform is not an error on the single-provider path.
    pub(crate) fn into_result(self) -> TransportResult {
        match self {
            ProviderOutcome::Delivered | ProviderOutcome::NoTargets => Ok(()),
            ProviderOutcome::NotConfigured => Err(NotificationError::PushNotConfigured),
            ProviderOutcome::Failed(e) => Err(e),
        }
    }
}

/// A combined push adapter that routes delivery to the correct provider by platform.
///
/// - FCM tokens (platform = `fcm`) are delivered via `FcmHttpAdapter`.
/// - APNs tokens (platform = `apns`) are delivered via `ApnsHttpAdapter`.
///
/// The user's device tokens are fetched **once** here and the shared slice is
/// handed to each provider (which selects its own platform), so a fanout over
/// N users costs N token queries, not 2N.
#[derive(Clone)]
pub struct CombinedPushAdapter {
    pub(crate) fcm: FcmHttpAdapter,
    pub(crate) apns: ApnsHttpAdapter,
    /// The combined adapter owns the single token fetch (see [`CombinedPushAdapter::send`]).
    /// Kept as its own field rather than reaching into `self.fcm.token_repo`, so the
    /// "fetch once" responsibility is expressed on the adapter that owns it and does not
    /// break if `FcmHttpAdapter` is later extracted to another module.
    token_repo: DevicePushTokenRepository,
}

impl CombinedPushAdapter {
    /// Create a combined adapter from the shared pool.
    pub fn from_env(pool: DbPool) -> Self {
        Self {
            fcm: FcmHttpAdapter::from_env(pool.clone()),
            apns: ApnsHttpAdapter::from_env(pool.clone()),
            token_repo: DevicePushTokenRepository::new(pool),
        }
    }

    /// Create a combined adapter with explicit configs (useful in tests).
    pub fn new(pool: DbPool, fcm_config: FcmConfig, apns_config: ApnsConfig) -> Self {
        Self {
            fcm: FcmHttpAdapter::new(pool.clone(), fcm_config),
            apns: ApnsHttpAdapter::new(pool.clone(), apns_config),
            token_repo: DevicePushTokenRepository::new(pool),
        }
    }

    /// Route a *pre-fetched* device-token set to both providers concurrently and
    /// combine their outcomes into a single pipeline result.
    ///
    /// Split out from [`CombinedPushAdapter::send`] (which owns the one DB fetch)
    /// so the routing + honest-metrics decision is unit-testable without a live
    /// DB or gateway: an unconfigured provider and a provider with no targets for
    /// its platform both short-circuit before any network I/O.
    async fn deliver_with_tokens(
        &self,
        user_id: Uuid,
        tokens: &[DevicePushToken],
        notification: &Notification,
    ) -> TransportResult {
        // Run FCM and APNs delivery concurrently off the shared token slice.
        let (fcm_outcome, apns_outcome) = tokio::join!(
            self.fcm.deliver(user_id, tokens, notification),
            self.apns.deliver(user_id, tokens, notification),
        );
        Self::combine(user_id, fcm_outcome, apns_outcome)
    }

    /// Combine the two providers' outcomes into the pipeline result.
    ///
    /// Honest-metrics contract:
    /// - **any** provider `Delivered` → `Ok(())` (Sent); a failure on the other
    ///   side is logged but does not suppress the successful delivery.
    /// - no delivery but **some** provider `Failed` → `Err(PushFailed)` (Failed).
    /// - no delivery and no failure (every provider `NoTargets` / `NotConfigured`)
    ///   → `Err(PushNotConfigured)` so the pipeline records **Skipped**, not Sent.
    fn combine(user_id: Uuid, fcm: ProviderOutcome, apns: ProviderOutcome) -> TransportResult {
        let delivered =
            matches!(fcm, ProviderOutcome::Delivered) || matches!(apns, ProviderOutcome::Delivered);

        let mut failures: Vec<String> = Vec::new();
        if let ProviderOutcome::Failed(e) = &fcm {
            failures.push(format!("FCM: {e}"));
        }
        if let ProviderOutcome::Failed(e) = &apns {
            failures.push(format!("APNs: {e}"));
        }

        if delivered {
            if !failures.is_empty() {
                tracing::warn!(
                    user_id = %user_id,
                    errors = %failures.join("; "),
                    "[8A-3] partial push delivery — one provider delivered, another failed"
                );
            }
            return Ok(());
        }

        if failures.is_empty() {
            // Nothing delivered and nothing failed: every provider was either
            // unconfigured or had no tokens for its platform. Record Skipped
            // (honest) rather than Sent (the iOS-only inflated-`sent` lie).
            tracing::info!(
                user_id = %user_id,
                "[8A-3] push skipped — no configured provider had a deliverable target"
            );
            Err(NotificationError::PushNotConfigured)
        } else {
            let msg = failures.join("; ");
            tracing::warn!(
                user_id = %user_id,
                error = %msg,
                "[8A-3] push delivery failed on all attempted providers"
            );
            Err(NotificationError::PushFailed(msg))
        }
    }
}

#[async_trait]
impl PushTransport for CombinedPushAdapter {
    async fn send(&self, user_id: Uuid, notification: &Notification) -> TransportResult {
        // Fetch the user's device tokens ONCE. Previously `FcmHttpAdapter` and
        // `ApnsHttpAdapter` each ran `get_tokens_for_user` independently — 2N
        // identical queries on the `dispatch_to_users` fanout hot path. Fetch
        // here and hand both providers the shared slice.
        let tokens = match self.token_repo.get_tokens_for_user(user_id).await {
            Ok(t) => t,
            Err(e) => {
                // Keep the raw DB error (which can carry connection-string
                // host/port/credentials) in the internal log only — never in the
                // returned error, which is persisted verbatim to the delivery
                // record via `into_failed(e.to_string())`.
                tracing::error!(
                    user_id = %user_id,
                    error = %e,
                    "[8A-3] Combined: failed to fetch device tokens for push delivery"
                );
                return Err(NotificationError::PushFailed(
                    "failed to fetch device tokens".to_string(),
                ));
            }
        };
        self.deliver_with_tokens(user_id, &tokens, notification)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::push::stored_token;

    // ------------------------------------------------------------------
    // Issue #2301: CombinedPushAdapter routing + honest-metrics guards
    // ------------------------------------------------------------------

    /// FCM configured (project id present), APNs left unconfigured.
    fn fcm_only_configured() -> (FcmConfig, ApnsConfig) {
        (
            FcmConfig {
                project_id: Some("test-project".to_string()),
                oauth_token: None,
                fcm_base_url: None,
            },
            ApnsConfig {
                p8_key_pem: None,
                key_id: None,
                team_id: None,
                topic: "three.two.bit.ppt.management".to_string(),
                apns_base_url: None,
            },
        )
    }

    /// APNs configured, FCM left unconfigured. (The APNs P8 is invalid so JWT
    /// minting would fail — but the tests below never reach minting because the
    /// user has no APNs targets, so no network I/O happens.)
    fn apns_only_configured() -> (FcmConfig, ApnsConfig) {
        (
            FcmConfig {
                project_id: None,
                oauth_token: None,
                fcm_base_url: None,
            },
            ApnsConfig {
                p8_key_pem: Some("--- not a real key ---".to_string()),
                key_id: Some("ABCDE12345".to_string()),
                team_id: Some("TEAM123456".to_string()),
                topic: "three.two.bit.ppt.management".to_string(),
                apns_base_url: None,
            },
        )
    }

    /// A lazy pool pointed at an unreachable address — never touched by the
    /// tests below (they call `deliver_with_tokens`, which takes tokens directly
    /// and never queries the DB).
    fn unused_pool() -> DbPool {
        sqlx::PgPool::connect_lazy("postgres://never-used:never@127.0.0.1:1/never")
            .expect("connect_lazy builds without connecting")
    }

    /// A lazy pool with a short acquire timeout pointed at an unreachable
    /// address: the first query fails fast with a connection error, so `send()`'s
    /// device-token fetch exercises the DB-error path without a live Postgres.
    fn failing_pool() -> DbPool {
        sqlx::postgres::PgPoolOptions::new()
            .acquire_timeout(std::time::Duration::from_millis(250))
            .connect_lazy("postgres://never-used:never@127.0.0.1:1/never")
            .expect("connect_lazy builds without connecting")
    }

    fn test_notification() -> Notification {
        Notification::new(
            Uuid::new_v4(),
            common::notifications::NotificationCategory::Announcements,
            "Building update",
            "The lift will be serviced tomorrow.",
        )
    }

    /// Honest-metrics regression guard (Issue #2301, finding 2).
    ///
    /// FCM is configured but APNs is **not**, and the user is iOS-only (only APNs
    /// tokens). Nothing is actually delivered — FCM has no FCM targets, APNs is
    /// unconfigured — so `CombinedPushAdapter` must report `PushNotConfigured`
    /// (the pipeline records **Skipped**), NOT `Ok(())` (**Sent**).
    ///
    /// Before this fix `(Ok(()), Err(PushNotConfigured))` mapped to `Ok(())`,
    /// re-introducing the inflated-`sent` lie (Issue #484) for every iOS-only
    /// user. This exercises the real `CombinedPushAdapter` routing + combine path
    /// (no DB, no gateway: both providers short-circuit before any I/O).
    #[tokio::test]
    async fn ios_only_user_with_fcm_only_configured_is_skipped_not_sent() {
        let (fcm, apns) = fcm_only_configured();
        let adapter = CombinedPushAdapter::new(unused_pool(), fcm, apns);
        let tokens = vec![stored_token("apns-device", "apns", None)];

        let result = adapter
            .deliver_with_tokens(Uuid::new_v4(), &tokens, &test_notification())
            .await;

        assert!(
            matches!(result, Err(NotificationError::PushNotConfigured)),
            "iOS-only user, only FCM configured → nothing delivered → must record \
             Skipped (PushNotConfigured), got {result:?}"
        );
    }

    /// Symmetric honest-metrics guard: APNs configured, FCM not, Android-only
    /// user (only FCM tokens). Nothing delivered → Skipped, not Sent.
    #[tokio::test]
    async fn android_only_user_with_apns_only_configured_is_skipped_not_sent() {
        let (fcm, apns) = apns_only_configured();
        let adapter = CombinedPushAdapter::new(unused_pool(), fcm, apns);
        let tokens = vec![stored_token("fcm-device", "fcm", None)];

        let result = adapter
            .deliver_with_tokens(Uuid::new_v4(), &tokens, &test_notification())
            .await;

        assert!(
            matches!(result, Err(NotificationError::PushNotConfigured)),
            "Android-only user, only APNs configured → nothing delivered → must \
             record Skipped (PushNotConfigured), got {result:?}"
        );
    }

    /// `combine` pins the honest-metrics decision table directly (no I/O).
    #[test]
    fn combine_records_skipped_when_nothing_delivered_and_nothing_failed() {
        let uid = Uuid::new_v4();
        for (fcm, apns) in [
            // iOS-only user, FCM configured but no FCM targets, APNs not configured.
            (ProviderOutcome::NoTargets, ProviderOutcome::NotConfigured),
            // Android-only symmetric.
            (ProviderOutcome::NotConfigured, ProviderOutcome::NoTargets),
            // Neither provider configured.
            (
                ProviderOutcome::NotConfigured,
                ProviderOutcome::NotConfigured,
            ),
            // Both configured, user has zero registered devices.
            (ProviderOutcome::NoTargets, ProviderOutcome::NoTargets),
        ] {
            assert!(
                matches!(
                    CombinedPushAdapter::combine(uid, fcm, apns),
                    Err(NotificationError::PushNotConfigured)
                ),
                "no real delivery and no failure must record Skipped"
            );
        }
    }

    #[test]
    fn combine_reports_sent_when_any_provider_delivered() {
        let uid = Uuid::new_v4();
        assert!(CombinedPushAdapter::combine(
            uid,
            ProviderOutcome::Delivered,
            ProviderOutcome::NotConfigured,
        )
        .is_ok());
        assert!(CombinedPushAdapter::combine(
            uid,
            ProviderOutcome::NotConfigured,
            ProviderOutcome::Delivered,
        )
        .is_ok());
        // A delivery on one side masks a failure on the other (partial success).
        assert!(CombinedPushAdapter::combine(
            uid,
            ProviderOutcome::Delivered,
            ProviderOutcome::Failed(NotificationError::PushFailed("apns down".into())),
        )
        .is_ok());
    }

    /// Security regression (code-review `push-db-err-leak`): when the single
    /// device-token fetch fails, the push error path must carry a FIXED,
    /// non-leaking message — never the raw `sqlx::Error` display, which can
    /// expose the DB host/port/credentials from the connection string. The raw
    /// error stays in the internal `tracing::error!` log only.
    #[tokio::test]
    async fn combined_db_fetch_error_message_does_not_leak_raw_db_error() {
        let (fcm, apns) = fcm_only_configured();
        let adapter = CombinedPushAdapter::new(failing_pool(), fcm, apns);

        let result = adapter.send(Uuid::new_v4(), &test_notification()).await;

        match result {
            Err(NotificationError::PushFailed(msg)) => {
                assert_eq!(
                    msg, "failed to fetch device tokens",
                    "DB fetch failure must map to a fixed non-leaking message, got {msg:?}"
                );
            }
            other => panic!("expected PushFailed(non-leaking message), got {other:?}"),
        }
    }

    #[test]
    fn combine_reports_failed_when_an_attempt_failed_and_nothing_delivered() {
        let uid = Uuid::new_v4();
        // APNs attempted and failed while FCM had no targets — honest Failed,
        // NOT Sent (the old `(Ok, Err)` → `Ok` arm would have lied here too).
        assert!(matches!(
            CombinedPushAdapter::combine(
                uid,
                ProviderOutcome::NoTargets,
                ProviderOutcome::Failed(NotificationError::PushFailed("apns down".into())),
            ),
            Err(NotificationError::PushFailed(_))
        ));
        // Both providers attempted and failed.
        assert!(matches!(
            CombinedPushAdapter::combine(
                uid,
                ProviderOutcome::Failed(NotificationError::PushFailed("fcm".into())),
                ProviderOutcome::Failed(NotificationError::PushFailed("apns".into())),
            ),
            Err(NotificationError::PushFailed(_))
        ));
    }
}
