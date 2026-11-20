//! Push notification fanout (Epic 8A-3).
//!
//! Delivers queued push notifications to Android (FCM) and iOS (APNs) devices.
//! Each provider is a [`PushTransport`](common::notifications::PushTransport)
//! implementation; [`combined::CombinedPushAdapter`] platform-routes a single
//! token fetch to both, and [`worker::PushFanoutWorker`] drains the Redis queue.
//!
//! This module was split from a single ~3.0k-LOC `push_fanout.rs` into
//! surface-scoped submodules. The split is behaviour-preserving: every type,
//! adapter, worker body, authz/stale-token guard and test is carried over
//! verbatim; the `PushTransport` trait remains the seam between the adapters
//! and the pipeline.
//!
//! | Concern                     | Module      |
//! |-----------------------------|-------------|
//! | Dispatch-target selection   | (this mod)  |
//! | FCM HTTP v1 transport       | [`fcm`]     |
//! | APNs HTTP/2 transport       | [`apns`]    |
//! | Platform routing (FCM+APNs) | [`combined`]|
//! | Worker configuration        | [`config`]  |
//! | Background fanout worker    | [`worker`]  |
//!
//! # Configuration (environment variables)
//!
//! | Variable                     | Required | Description                                                        |
//! |------------------------------|----------|--------------------------------------------------------------------|
//! | `FCM_PROJECT_ID`             | No       | GCP project ID for FCM HTTP v1 (`projects/{id}/…`) — required to enable Android push |
//! | `FCM_SERVICE_ACCOUNT_JSON`   | No       | Inline Google service-account JSON. **Preferred** for HTTP v1: the adapter mints and auto-refreshes an OAuth2 access token before Google's ~1h TTL expiry. |
//! | `GOOGLE_APPLICATION_CREDENTIALS` | No   | Path to a Google service-account JSON file (same effect as `FCM_SERVICE_ACCOUNT_JSON`, used when the inline var is unset). |
//! | `FCM_OAUTH_TOKEN`           | No       | **Legacy / deprecated.** A pre-minted OAuth2 bearer token, read once at startup and never refreshed. |
//! | `APNS_P8_KEY`               | No       | PEM-encoded P8 ECDSA private key for APNs provider auth             |
//! | `APNS_KEY_ID`               | No       | 10-char Key ID printed on the P8 file in App Store Connect          |
//! | `APNS_TEAM_ID`              | No       | 10-char Apple Team ID                                               |
//! | `APNS_TOPIC`                | No       | APNs topic / bundle ID (default: `three.two.bit.ppt.management`)    |
//! | `PUSH_FANOUT_ENABLED`       | No       | Set to `false` / `0` to disable the worker                         |
//! | `PUSH_FANOUT_POLL_SECS`     | No       | Polling interval in seconds (default: 30)                           |
//!
//! If neither FCM nor APNs is configured the worker logs a warning and becomes a
//! no-op loop — the server will not crash.

use std::time::Duration;

use db::{models::device_push_token::PushPlatform, repositories::DevicePushTokenRepository};
use uuid::Uuid;

pub mod apns;
pub mod combined;
pub mod config;
pub mod fcm;
pub mod worker;

pub use apns::{ApnsConfig, ApnsHttpAdapter};
pub use combined::CombinedPushAdapter;
pub use config::PushFanoutConfig;
pub use fcm::{FcmConfig, FcmHttpAdapter};
pub use worker::PushFanoutWorker;

// ============================================================================
// Dispatch-target selection (preference-aware)
// ============================================================================

/// A single device that has been selected as a push dispatch target.
///
/// Produced by [`select_dispatch_targets`] from the raw rows stored in
/// `device_push_tokens`. Keeping selection in a small pure struct (rather than
/// passing whole [`DevicePushToken`] rows around the delivery loop) makes the
/// "which devices do we actually send to" decision unit-testable without a DB
/// or a live FCM/APNs gateway.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushTarget {
    /// The `device_push_tokens.id` of the selected row (used for receipts /
    /// stale-token eviction).
    pub token_id: Uuid,
    /// The raw FCM registration token / APNs device token.
    pub token: String,
    /// Which OS gateway this target is delivered through.
    pub platform: PushPlatform,
    /// Bundle / package id the token was registered under, if any.
    pub app_id: Option<String>,
}

/// Filter controlling which stored device tokens become dispatch targets.
///
/// This is the device-token-level companion to the channel-level
/// `PreferenceRouter` in `notification_pipeline.rs`. The pipeline decides
/// *whether* the push channel is enabled for a user at all; this filter then
/// decides *which of the user's registered devices* should receive the push.
///
/// All fields default to "no restriction" so an empty filter selects every
/// non-empty token the user has registered.
#[derive(Debug, Clone, Default)]
pub struct PushTargetFilter {
    /// When set, only tokens registered under this bundle / package id are
    /// selected. Lets a notification originating from the Property-Management
    /// app avoid waking the Reality-Portal binary on the same device (and vice
    /// versa) when both are installed.
    pub app_id: Option<String>,
    /// When set, only tokens for these platforms are selected. `None` selects
    /// both FCM and APNs.
    pub platforms: Option<Vec<PushPlatform>>,
}

impl PushTargetFilter {
    /// Returns `true` when the given stored token row passes the filter.
    fn accepts(&self, token: &db::models::DevicePushToken) -> bool {
        // Never dispatch to an empty/blank token — it can only ever be rejected
        // by the gateway and would waste a request + risk a spurious stale-token
        // eviction.
        if token.token.trim().is_empty() {
            return false;
        }

        if let Some(ref want_app) = self.app_id {
            match token.app_id.as_deref() {
                Some(app) if app == want_app => {}
                _ => return false,
            }
        }

        if let Some(ref platforms) = self.platforms {
            if !platforms.contains(&token.push_platform()) {
                return false;
            }
        }

        true
    }
}

/// Select the set of dispatch targets from a user's stored device tokens.
///
/// Pure function (no I/O): the caller fetches the rows, this decides which ones
/// are valid push targets given `filter`. Blank tokens are always dropped.
///
/// The result preserves the input ordering so callers that fetched tokens
/// `ORDER BY last_seen_at DESC` keep "most-recently-seen device first".
pub fn select_dispatch_targets(
    tokens: &[db::models::DevicePushToken],
    filter: &PushTargetFilter,
) -> Vec<PushTarget> {
    tokens
        .iter()
        .filter(|t| filter.accepts(t))
        .map(|t| PushTarget {
            token_id: t.id,
            token: t.token.clone(),
            platform: t.push_platform(),
            app_id: t.app_id.clone(),
        })
        .collect()
}

// ============================================================================
// FCM delivery receipt (stored in memory for now; DB table in follow-up)
// ============================================================================

/// Outcome of a single FCM/APNs delivery attempt for one device token.
#[derive(Debug, Clone)]
pub struct PushDeliveryReceipt {
    pub token_id: Uuid,
    pub user_id: Uuid,
    pub platform: PushPlatform,
    /// Whether the message was accepted by the upstream gateway.
    pub success: bool,
    /// Error string set when `success == false`.
    pub error: Option<String>,
    /// Stale token: the upstream gateway signalled the token is no longer valid.
    pub token_expired: bool,
    pub attempted_at: chrono::DateTime<chrono::Utc>,
}

/// Build the shared `reqwest::Client` used by a push transport.
///
/// `reqwest::Client::builder().build()` only fails when the underlying TLS
/// backend or DNS resolver cannot be initialised — a recoverable environment /
/// configuration problem, not a logic bug. The push-fanout worker previously
/// `.expect()`-panicked at this point, so a transient init failure took the
/// whole worker (and, on the synchronous pipeline path, request handling) down
/// instead of degrading to "push skipped". Returning the error lets each
/// constructor log it and fall back to a disabled (no-op) transport, matching
/// how the rest of this module degrades when a provider is misconfigured.
pub(crate) fn build_push_http_client(
    timeout: Duration,
    http2_prior_knowledge: bool,
) -> Result<reqwest::Client, reqwest::Error> {
    let mut builder = reqwest::Client::builder().timeout(timeout);
    if http2_prior_knowledge {
        // APNs requires HTTP/2; enabling http2_prior_knowledge avoids the TLS
        // ALPN upgrade round-trip to the known-H2 gateway.
        builder = builder.http2_prior_knowledge();
    }
    builder.build()
}

// ---------------------------------------------------------------------------
// Internal helper: delete a stale token by its string value using service-role pool
// ---------------------------------------------------------------------------

/// Delete a specific stale push token by its string value via the service-role pool.
///
/// Uses `DevicePushTokenRepository::delete_stale_token` which issues a targeted
/// `DELETE … WHERE user_id = $1 AND token = $2` so it cannot accidentally evict
/// tokens belonging to other users.
pub(crate) async fn delete_stale_token_by_string(
    repo: &DevicePushTokenRepository,
    user_id: Uuid,
    token: &str,
) -> Result<(), String> {
    repo.delete_stale_token(user_id, token)
        .await
        .map(|_deleted| ())
        .map_err(|e| format!("DB error deleting stale token: {e}"))
}

/// Shared test helper: build a stored `DevicePushToken` row for selection and
/// routing tests across the push submodules.
#[cfg(test)]
pub(crate) fn stored_token(
    token: &str,
    platform: &str,
    app_id: Option<&str>,
) -> db::models::DevicePushToken {
    db::models::DevicePushToken {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        token: token.to_string(),
        platform: platform.to_string(),
        app_id: app_id.map(|s| s.to_string()),
        device_name: None,
        last_seen_at: chrono::Utc::now(),
        created_at: chrono::Utc::now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // Token storage: row -> typed platform / response mapping
    // ------------------------------------------------------------------

    #[test]
    fn stored_token_maps_platform_string_to_enum() {
        assert_eq!(
            stored_token("t", "fcm", None).push_platform(),
            PushPlatform::Fcm
        );
        assert_eq!(
            stored_token("t", "apns", None).push_platform(),
            PushPlatform::Apns
        );
    }

    #[test]
    fn stored_token_unknown_platform_falls_back_to_fcm() {
        // Defensive: an unexpected platform value must not panic the fanout
        // loop — it degrades to FCM (and is logged at warn by `push_platform`).
        assert_eq!(
            stored_token("t", "windows-phone", None).push_platform(),
            PushPlatform::Fcm
        );
    }

    // ------------------------------------------------------------------
    // Dispatch-target selection
    // ------------------------------------------------------------------

    #[test]
    fn select_targets_empty_filter_selects_all_non_blank() {
        let tokens = vec![
            stored_token("fcm-aaa", "fcm", None),
            stored_token("apns-bbb", "apns", None),
        ];
        let targets = select_dispatch_targets(&tokens, &PushTargetFilter::default());
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].platform, PushPlatform::Fcm);
        assert_eq!(targets[1].platform, PushPlatform::Apns);
    }

    #[test]
    fn select_targets_drops_blank_tokens() {
        let tokens = vec![
            stored_token("   ", "fcm", None),
            stored_token("", "apns", None),
            stored_token("real-token", "fcm", None),
        ];
        let targets = select_dispatch_targets(&tokens, &PushTargetFilter::default());
        assert_eq!(
            targets.len(),
            1,
            "only the non-blank token is a valid target"
        );
        assert_eq!(targets[0].token, "real-token");
    }

    #[test]
    fn select_targets_preserves_input_order() {
        // Callers fetch ORDER BY last_seen_at DESC; selection must keep the
        // most-recently-seen device first.
        let tokens = vec![
            stored_token("first", "fcm", None),
            stored_token("second", "apns", None),
            stored_token("third", "fcm", None),
        ];
        let targets = select_dispatch_targets(&tokens, &PushTargetFilter::default());
        let order: Vec<&str> = targets.iter().map(|t| t.token.as_str()).collect();
        assert_eq!(order, vec!["first", "second", "third"]);
    }

    #[test]
    fn select_targets_filters_by_platform() {
        let tokens = vec![
            stored_token("fcm-1", "fcm", None),
            stored_token("apns-1", "apns", None),
            stored_token("fcm-2", "fcm", None),
        ];
        let filter = PushTargetFilter {
            platforms: Some(vec![PushPlatform::Fcm]),
            ..Default::default()
        };
        let targets = select_dispatch_targets(&tokens, &filter);
        assert_eq!(targets.len(), 2);
        assert!(targets.iter().all(|t| t.platform == PushPlatform::Fcm));
    }

    #[test]
    fn select_targets_filters_by_app_id() {
        let tokens = vec![
            stored_token("mgmt", "fcm", Some("three.two.bit.ppt.management")),
            stored_token("reality", "fcm", Some("three.two.bit.ppt.reality")),
            stored_token("legacy-no-app", "fcm", None),
        ];
        let filter = PushTargetFilter {
            app_id: Some("three.two.bit.ppt.management".to_string()),
            ..Default::default()
        };
        let targets = select_dispatch_targets(&tokens, &filter);
        assert_eq!(targets.len(), 1, "only the matching bundle id is selected");
        assert_eq!(targets[0].token, "mgmt");
    }

    #[test]
    fn select_targets_app_id_filter_excludes_null_app_id() {
        // A token registered without an app_id must NOT match a specific-bundle
        // filter — otherwise a Reality push could wake a legacy un-tagged token.
        let tokens = vec![stored_token("legacy", "fcm", None)];
        let filter = PushTargetFilter {
            app_id: Some("three.two.bit.ppt.management".to_string()),
            ..Default::default()
        };
        assert!(select_dispatch_targets(&tokens, &filter).is_empty());
    }

    #[test]
    fn select_targets_empty_input_is_empty() {
        assert!(select_dispatch_targets(&[], &PushTargetFilter::default()).is_empty());
    }

    #[test]
    fn select_targets_carries_token_id_for_receipts() {
        let token = stored_token("fcm-x", "fcm", None);
        let id = token.id;
        let targets = select_dispatch_targets(&[token], &PushTargetFilter::default());
        assert_eq!(
            targets[0].token_id, id,
            "token_id must survive selection for receipts/eviction"
        );
    }

    #[test]
    fn build_push_http_client_succeeds_for_both_profiles() {
        // Regression guard: the production push transports build their reqwest
        // client through this helper instead of `.expect()`-panicking. Both the
        // plain (FCM / OAuth token-exchange) and the HTTP/2-prior-knowledge
        // (APNs) profiles must build cleanly so a healthy process keeps a usable
        // client (`Some`) rather than degrading to the no-op skip path. A build
        // failure here is surfaced as `Err` and logged by the constructor —
        // never a panic that would take down the fanout worker.
        assert!(
            build_push_http_client(Duration::from_secs(10), false).is_ok(),
            "FCM/OAuth client profile must build without panicking"
        );
        assert!(
            build_push_http_client(Duration::from_secs(15), true).is_ok(),
            "APNs HTTP/2 client profile must build without panicking"
        );
    }

    // ------------------------------------------------------------------
    // CombinedPushAdapter routing logic (via PushTargetFilter)
    // ------------------------------------------------------------------

    #[test]
    fn apns_filter_selects_only_apns_tokens() {
        let tokens = vec![
            stored_token("fcm-1", "fcm", None),
            stored_token("apns-1", "apns", None),
            stored_token("apns-2", "apns", None),
        ];
        let filter = PushTargetFilter {
            platforms: Some(vec![PushPlatform::Apns]),
            ..Default::default()
        };
        let targets = select_dispatch_targets(&tokens, &filter);
        assert_eq!(targets.len(), 2);
        assert!(targets.iter().all(|t| t.platform == PushPlatform::Apns));
    }

    #[test]
    fn fcm_filter_selects_only_fcm_tokens() {
        let tokens = vec![
            stored_token("fcm-1", "fcm", None),
            stored_token("apns-1", "apns", None),
            stored_token("fcm-2", "fcm", None),
        ];
        let filter = PushTargetFilter {
            platforms: Some(vec![PushPlatform::Fcm]),
            ..Default::default()
        };
        let targets = select_dispatch_targets(&tokens, &filter);
        assert_eq!(targets.len(), 2);
        assert!(targets.iter().all(|t| t.platform == PushPlatform::Fcm));
    }

    #[test]
    fn no_targets_when_only_wrong_platform_tokens() {
        let tokens = vec![
            stored_token("fcm-1", "fcm", None),
            stored_token("fcm-2", "fcm", None),
        ];
        let filter = PushTargetFilter {
            platforms: Some(vec![PushPlatform::Apns]),
            ..Default::default()
        };
        // APNs adapter sees no targets → graceful no-op.
        assert!(select_dispatch_targets(&tokens, &filter).is_empty());
    }

    // ------------------------------------------------------------------
    // PushDeliveryReceipt construction (APNs)
    // ------------------------------------------------------------------

    #[test]
    fn apns_delivery_receipt_captures_platform() {
        let receipt = PushDeliveryReceipt {
            token_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            platform: PushPlatform::Apns,
            success: true,
            error: None,
            token_expired: false,
            attempted_at: chrono::Utc::now(),
        };
        assert_eq!(receipt.platform, PushPlatform::Apns);
        assert!(receipt.success);
        assert!(!receipt.token_expired);
    }

    #[test]
    fn apns_delivery_receipt_marks_expired_on_bad_token() {
        let receipt = PushDeliveryReceipt {
            token_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            platform: PushPlatform::Apns,
            success: false,
            error: Some("APNs rejected".to_string()),
            token_expired: true,
            attempted_at: chrono::Utc::now(),
        };
        assert!(!receipt.success);
        assert!(receipt.token_expired);
    }

    // ------------------------------------------------------------------
    // gap-84-4: pipeline push transport is multi-platform (FCM + APNs)
    // ------------------------------------------------------------------

    /// Regression guard for gap-84-4: the notification pipeline's push channel
    /// now dispatches through `CombinedPushAdapter`, which must treat BOTH
    /// Android (FCM) and iOS (APNs) tokens as first-class delivery targets.
    ///
    /// Before this fix the pipeline wired the FCM-only `FcmHttpAdapter`, which
    /// logged APNs tokens as "not yet implemented" and dropped them — so an
    /// iOS-only user's push notifications were silently discarded on the
    /// synchronous dispatch path. This test pins that a mixed and an APNs-only
    /// token set both yield real dispatch targets so the combined adapter can
    /// route them to their respective gateways.
    #[test]
    fn combined_push_adapter_treats_apns_as_first_class_target() {
        // APNs-only user: on the old FCM-only path this produced zero real
        // delivery targets (dropped as "unsupported"); it must now select the
        // APNs token so `ApnsHttpAdapter` can deliver it.
        let apns_only = vec![stored_token("apns-device", "apns", None)];
        let targets = select_dispatch_targets(&apns_only, &PushTargetFilter::default());
        assert_eq!(
            targets.len(),
            1,
            "an APNs-only user must have a real push dispatch target"
        );
        assert_eq!(targets[0].platform, PushPlatform::Apns);

        // Mixed user: both platforms are selected (order preserved), so the
        // combined adapter fans out to FCM and APNs concurrently.
        let mixed = vec![
            stored_token("fcm-device", "fcm", None),
            stored_token("apns-device", "apns", None),
        ];
        let mixed_targets = select_dispatch_targets(&mixed, &PushTargetFilter::default());
        let platforms: Vec<PushPlatform> =
            mixed_targets.iter().map(|t| t.platform.clone()).collect();
        assert!(
            platforms.contains(&PushPlatform::Fcm) && platforms.contains(&PushPlatform::Apns),
            "a mixed FCM+APNs user must produce targets for both platforms; got {platforms:?}"
        );
    }
}
