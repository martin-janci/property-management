//! Saved-search alert email/push **transport drainer** — BIT-139 (Epic 16,
//! issue #983 follow-up).
//!
//! The saved-search matching engine ([`super::SavedSearchAlertWorker`]) enqueues
//! `search_alert_queue` rows and the portal surfaces them as an in-app feed
//! (`GET /api/v1/saved-searches/alerts`). Until now that in-app pull was the
//! *only* delivery channel — reality-server had no email/push transport.
//!
//! This background worker drains the same queue out-of-band: for each alert it
//! has not yet delivered it dispatches an email to the owning user and a push
//! notification to each of their registered devices (`device_push_tokens`), then
//! records delivery in `notified_at` (migration 00195). That column is tracked
//! independently of `status` — which belongs to the in-app read channel — so the
//! drainer never re-sends on every poll and never clobbers the unread badge.
//!
//! **First cut (this change):** the transports are pluggable behind the
//! [`AlertEmailTransport`] / [`AlertPushTransport`] traits, defaulting to
//! logging stubs ([`LogEmailTransport`] / [`LogPushTransport`]). reality-server
//! has no real email service wired yet, so the stubs let the full drain →
//! device-token fan-out → mark-delivered pipeline land and be exercised by tests
//! now; a real SMTP/FCM/APNs adapter can be injected later via
//! [`SearchAlertDrainerWorker::with_transports`] without touching the loop.
//!
//! **Cross-tenant note:** the drainer runs service-role (no `app.current_user_id`
//! on its connections). `search_alert_queue` / `portal_saved_searches` / `users`
//! are not RLS-gated, and `device_push_tokens` exposes a service-role SELECT
//! policy. Every row carries its *own* owner's contact details and we fan tokens
//! out strictly by that row's `user_id`, so one user's alert can never be
//! delivered to another user's address or device.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use db::models::user::Locale;
use db::repositories::{DevicePushTokenRepository, RealityPortalRepository};
use db::DbPool;
use tokio::time::interval;
use tracing::Instrument;

/// A composed alert notification, ready to hand to a transport.
#[derive(Debug, Clone)]
pub struct AlertNotification {
    pub subject: String,
    pub body: String,
    /// Recipient locale (`sk` | `cs` | `de` | `en`) the copy was rendered in.
    pub locale: String,
}

impl AlertNotification {
    /// Compose a fully localized subject + body for the recipient's `locale`,
    /// given how many new listings matched (`count`, always ≥ 1 when an alert is
    /// enqueued) and the saved-search `name`.
    ///
    /// `locale` is the recipient's carried locale string (`u.locale`), which may
    /// be absent — [`Locale::parse`] normalizes it and falls back to English for
    /// an absent or unrecognized value. Every supported language (sk, cs, de, en)
    /// is covered, with correct count-based noun/verb forms.
    pub fn localized(locale: Option<&str>, count: usize, name: &str) -> Self {
        let loc = Locale::parse(locale.unwrap_or_default());
        let (subject, body) = compose_alert_copy(&loc, count, name);
        Self {
            subject,
            body,
            locale: loc.as_str().to_string(),
        }
    }
}

/// Render the localized (subject, body) pair for one saved-search alert.
///
/// Slovak and Czech use three count buckets (1 / 2–4 / 5+); German and English
/// use two (1 / many). Verb and pronoun agreement follow the same bucketing.
fn compose_alert_copy(locale: &Locale, count: usize, name: &str) -> (String, String) {
    // Slavic plural category for `count`: 0 = singular, 1 = paucal (2–4),
    // 2 = plural/genitive (0, 5+). Only ever called with count ≥ 1.
    let slavic_bucket = |n: usize| -> usize {
        match n {
            1 => 0,
            2..=4 => 1,
            _ => 2,
        }
    };

    match locale {
        Locale::Slovak => {
            let noun = ["nový inzerát", "nové inzeráty", "nových inzerátov"][slavic_bucket(count)];
            let matches_verb = if count == 1 {
                "zodpovedá"
            } else {
                "zodpovedajú"
            };
            let pronoun = if count == 1 { "ho" } else { "ich" };
            (
                format!("{count} {noun} {matches_verb} vášmu uloženému hľadaniu „{name}“"),
                format!(
                    "Vaše uložené hľadanie „{name}“ má {count} {noun}. \
                     Otvorte aplikáciu a zobrazte si {pronoun}."
                ),
            )
        }
        Locale::Czech => {
            let noun = ["nový inzerát", "nové inzeráty", "nových inzerátů"][slavic_bucket(count)];
            let matches_verb = if (2..=4).contains(&count) {
                "odpovídají"
            } else {
                "odpovídá"
            };
            let pronoun = if count == 1 { "jej" } else { "je" };
            (
                format!("{count} {noun} {matches_verb} vašemu uloženému hledání „{name}“"),
                format!(
                    "Vaše uložené hledání „{name}“ má {count} {noun}. \
                     Otevřete aplikaci a zobrazte si {pronoun}."
                ),
            )
        }
        Locale::German => {
            let (noun, matches_verb, pronoun) = if count == 1 {
                ("neues Inserat", "passt", "es")
            } else {
                ("neue Inserate", "passen", "sie")
            };
            (
                format!("{count} {noun} {matches_verb} zu Ihrer gespeicherten Suche „{name}“"),
                format!(
                    "Ihre gespeicherte Suche „{name}“ hat {count} neue passende \
                     {inserat}. Öffnen Sie die App, um {pronoun} anzusehen.",
                    inserat = if count == 1 { "Inserat" } else { "Inserate" },
                ),
            )
        }
        Locale::English => {
            let noun = if count == 1 {
                "new listing"
            } else {
                "new listings"
            };
            let matches_verb = if count == 1 { "matches" } else { "match" };
            let pronoun = if count == 1 { "it" } else { "them" };
            (
                format!("{count} {noun} {matches_verb} your saved search \"{name}\""),
                format!(
                    "Your saved search \"{name}\" has {count} new matching {listing}. \
                     Open the app to view {pronoun}.",
                    listing = if count == 1 { "listing" } else { "listings" },
                ),
            )
        }
    }
}

/// Email side of the drainer's transport. The default is a logging stub; a real
/// SMTP-backed adapter can be injected via
/// [`SearchAlertDrainerWorker::with_transports`].
#[async_trait]
pub trait AlertEmailTransport: Send + Sync {
    /// Deliver `notification` to `to_email`. `Err` is a transient failure: the
    /// drainer records an attempt and retries on a later poll.
    async fn send_email(
        &self,
        to_email: &str,
        to_name: &str,
        notification: &AlertNotification,
    ) -> Result<(), String>;
}

/// Push side of the drainer's transport (one call per device token).
#[async_trait]
pub trait AlertPushTransport: Send + Sync {
    async fn send_push(
        &self,
        token: &str,
        platform: &str,
        notification: &AlertNotification,
    ) -> Result<(), String>;
}

/// Logging email stub — records what *would* be sent. Used until a real email
/// service is wired into reality-server.
pub struct LogEmailTransport;

#[async_trait]
impl AlertEmailTransport for LogEmailTransport {
    async fn send_email(
        &self,
        to_email: &str,
        _to_name: &str,
        notification: &AlertNotification,
    ) -> Result<(), String> {
        tracing::info!(
            target: "bg.search_alert_drainer",
            to = %to_email,
            subject = %notification.subject,
            "[BIT-139] (stub) email alert dispatched"
        );
        Ok(())
    }
}

/// Logging push stub — records what *would* be sent per device token.
pub struct LogPushTransport;

#[async_trait]
impl AlertPushTransport for LogPushTransport {
    async fn send_push(
        &self,
        token: &str,
        platform: &str,
        notification: &AlertNotification,
    ) -> Result<(), String> {
        // Never log the full token (a credential): a short prefix is enough to
        // correlate without leaking it into logs.
        let token_prefix: String = token.chars().take(8).collect();
        tracing::info!(
            target: "bg.search_alert_drainer",
            platform = %platform,
            token_prefix = %token_prefix,
            subject = %notification.subject,
            "[BIT-139] (stub) push alert dispatched"
        );
        Ok(())
    }
}

/// Configuration for the saved-search alert transport drainer.
#[derive(Debug, Clone)]
pub struct SearchAlertDrainerConfig {
    pub enabled: bool,
    pub poll_interval_secs: u64,
    /// Max alerts drained per poll.
    pub batch_size: i64,
    /// Give up retrying a row after this many failed delivery attempts.
    pub max_attempts: i32,
}

impl Default for SearchAlertDrainerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            poll_interval_secs: 60,
            batch_size: 100,
            max_attempts: 5,
        }
    }
}

impl SearchAlertDrainerConfig {
    /// Build from environment:
    /// - `SEARCH_ALERT_DRAINER_ENABLED` (default `true`)
    /// - `SEARCH_ALERT_DRAINER_INTERVAL_SECS` (default `60`)
    /// - `SEARCH_ALERT_DRAINER_BATCH_SIZE` (default `100`)
    /// - `SEARCH_ALERT_DRAINER_MAX_ATTEMPTS` (default `5`)
    pub fn from_env() -> Self {
        let default = Self::default();
        let enabled = std::env::var("SEARCH_ALERT_DRAINER_ENABLED")
            .map(|v| v != "false" && v != "0")
            .unwrap_or(default.enabled);
        let poll_interval_secs = std::env::var("SEARCH_ALERT_DRAINER_INTERVAL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default.poll_interval_secs);
        let batch_size = std::env::var("SEARCH_ALERT_DRAINER_BATCH_SIZE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default.batch_size);
        let max_attempts = std::env::var("SEARCH_ALERT_DRAINER_MAX_ATTEMPTS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default.max_attempts);
        Self {
            enabled,
            poll_interval_secs,
            batch_size,
            max_attempts,
        }
    }

    /// Poll cadence as a `Duration`, guaranteed non-zero.
    ///
    /// `tokio::time::interval` panics on a zero period, so a misconfigured
    /// `SEARCH_ALERT_DRAINER_INTERVAL_SECS=0` (which [`from_env`] parses as a
    /// valid `0`) must never reach it. Floor at 1s.
    ///
    /// [`from_env`]: SearchAlertDrainerConfig::from_env
    fn poll_interval(&self) -> Duration {
        Duration::from_secs(self.poll_interval_secs.max(1))
    }
}

/// Background worker that drains undelivered `search_alert_queue` rows to email
/// and push transports.
pub struct SearchAlertDrainerWorker {
    repo: RealityPortalRepository,
    push_tokens: DevicePushTokenRepository,
    email: Arc<dyn AlertEmailTransport>,
    push: Arc<dyn AlertPushTransport>,
    config: SearchAlertDrainerConfig,
}

impl SearchAlertDrainerWorker {
    /// Construct with the default logging stubs (no real email/push service).
    pub fn new(db: DbPool, config: SearchAlertDrainerConfig) -> Self {
        Self::with_transports(
            db,
            config,
            Arc::new(LogEmailTransport),
            Arc::new(LogPushTransport),
        )
    }

    /// Construct with explicit transports — used by tests and, later, by
    /// production wiring once real SMTP/FCM/APNs adapters exist.
    pub fn with_transports(
        db: DbPool,
        config: SearchAlertDrainerConfig,
        email: Arc<dyn AlertEmailTransport>,
        push: Arc<dyn AlertPushTransport>,
    ) -> Self {
        Self {
            repo: RealityPortalRepository::new(db.clone()),
            push_tokens: DevicePushTokenRepository::new(db),
            email,
            push,
            config,
        }
    }

    /// Spawn the background task and return its `JoinHandle`.
    pub fn start(self) -> tokio::task::JoinHandle<()> {
        let poll_secs = self.config.poll_interval_secs;
        tokio::spawn(
            async move {
                if !self.config.enabled {
                    tracing::info!("[BIT-139] SearchAlertDrainerWorker disabled — not starting");
                    return;
                }
                tracing::info!(
                    poll_interval_secs = self.config.poll_interval_secs,
                    batch_size = self.config.batch_size,
                    "[BIT-139] SearchAlertDrainerWorker started"
                );
                let mut ticker = interval(self.config.poll_interval());
                loop {
                    ticker.tick().await;
                    self.run_once().await;
                }
            }
            .instrument(tracing::info_span!(
                "bg.search_alert_drainer",
                poll_secs = poll_secs
            )),
        )
    }

    /// One drain pass. Returns the number of alerts marked delivered (handy for
    /// tests; ignored by the loop).
    pub async fn run_once(&self) -> usize {
        let alerts = match self
            .repo
            .list_undelivered_search_alerts(self.config.batch_size, self.config.max_attempts)
            .await
        {
            Ok(a) => a,
            Err(e) => {
                tracing::error!(error = %e, "[BIT-139] failed to list undelivered alerts");
                return 0;
            }
        };

        let mut delivered = 0usize;
        for alert in alerts {
            let count = alert.matching_listing_ids.len();
            let notification = AlertNotification::localized(
                alert.recipient_locale.as_deref(),
                count,
                &alert.saved_search_name,
            );

            // An alert counts as delivered if *any* channel succeeds. Email is
            // the primary channel; push is best-effort fan-out across devices.
            let mut any_ok = false;

            match self
                .email
                .send_email(&alert.recipient_email, &alert.recipient_name, &notification)
                .await
            {
                Ok(()) => any_ok = true,
                Err(e) => tracing::warn!(
                    alert_id = %alert.id, error = %e,
                    "[BIT-139] email transport failed"
                ),
            }

            // Push fan-out: strictly this row's owner's tokens (service-role read).
            match self.push_tokens.get_tokens_for_user(alert.user_id).await {
                Ok(tokens) => {
                    for tok in &tokens {
                        match self
                            .push
                            .send_push(&tok.token, &tok.platform, &notification)
                            .await
                        {
                            Ok(()) => any_ok = true,
                            Err(e) => tracing::warn!(
                                alert_id = %alert.id, error = %e,
                                "[BIT-139] push transport failed for a device"
                            ),
                        }
                    }
                }
                Err(e) => tracing::warn!(
                    alert_id = %alert.id, error = %e,
                    "[BIT-139] failed to load device tokens"
                ),
            }

            if any_ok {
                if let Err(e) = self.repo.mark_search_alert_notified(alert.id).await {
                    tracing::error!(alert_id = %alert.id, error = %e, "[BIT-139] mark-notified failed");
                } else {
                    delivered += 1;
                }
            } else if let Err(e) = self.repo.record_search_alert_notify_failure(alert.id).await {
                tracing::error!(alert_id = %alert.id, error = %e, "[BIT-139] record-failure failed");
            }
        }

        if delivered > 0 {
            tracing::info!(delivered, "[BIT-139] saved-search alerts delivered");
        }
        delivered
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poll_interval_is_floored_to_avoid_zero_period_panic() {
        // A zero period would panic inside `tokio::time::interval`; the config
        // must floor it so a `SEARCH_ALERT_DRAINER_INTERVAL_SECS=0` misconfig
        // never crashes the worker task.
        let cfg = SearchAlertDrainerConfig {
            poll_interval_secs: 0,
            ..SearchAlertDrainerConfig::default()
        };
        assert_eq!(cfg.poll_interval(), Duration::from_secs(1));

        // A valid value passes through unchanged.
        let cfg = SearchAlertDrainerConfig {
            poll_interval_secs: 45,
            ..SearchAlertDrainerConfig::default()
        };
        assert_eq!(cfg.poll_interval(), Duration::from_secs(45));
    }

    #[test]
    fn absent_or_unknown_locale_falls_back_to_english() {
        for loc in [None, Some(""), Some("xx"), Some("fr")] {
            let n = AlertNotification::localized(loc, 3, "Byty BA");
            assert_eq!(n.locale, "en", "locale {loc:?} should fall back to en");
            assert!(n.subject.contains("new listings match"), "{}", n.subject);
        }
    }

    #[test]
    fn each_supported_language_renders_its_own_copy() {
        // (locale-in, normalized code, a substring unique to that language)
        let cases = [
            ("sk", "sk", "uloženému hľadaniu"),
            ("cs", "cs", "uloženému hledání"),
            ("de", "de", "gespeicherten Suche"),
            ("en", "en", "saved search"),
        ];
        for (input, code, needle) in cases {
            let n = AlertNotification::localized(Some(input), 2, "Test");
            assert_eq!(n.locale, code);
            assert!(
                n.subject.contains(needle),
                "{code} subject missing {needle:?}: {}",
                n.subject
            );
            assert!(!n.body.is_empty());
            // Never leak English copy into a non-English notification.
            if code != "en" {
                assert!(
                    !n.subject.contains("saved search") && !n.body.contains("Open the app"),
                    "{code} copy still English: {} / {}",
                    n.subject,
                    n.body
                );
            }
        }
    }

    #[test]
    fn count_based_plural_forms_are_selected() {
        // English: singular verb/noun at 1, plural otherwise.
        let one = AlertNotification::localized(Some("en"), 1, "S");
        assert!(
            one.subject.contains("1 new listing matches"),
            "{}",
            one.subject
        );
        assert!(one.body.contains("view it."), "{}", one.body);
        let many = AlertNotification::localized(Some("en"), 5, "S");
        assert!(
            many.subject.contains("5 new listings match"),
            "{}",
            many.subject
        );
        assert!(many.body.contains("view them."), "{}", many.body);

        // Slovak three-bucket noun declension: 1 / 2–4 / 5+.
        assert!(AlertNotification::localized(Some("sk"), 1, "S")
            .subject
            .contains("nový inzerát"));
        assert!(AlertNotification::localized(Some("sk"), 3, "S")
            .subject
            .contains("nové inzeráty"));
        assert!(AlertNotification::localized(Some("sk"), 9, "S")
            .subject
            .contains("nových inzerátov"));

        // Czech three-bucket noun declension: 1 / 2–4 / 5+.
        assert!(AlertNotification::localized(Some("cs"), 1, "S")
            .subject
            .contains("nový inzerát"));
        assert!(AlertNotification::localized(Some("cs"), 4, "S")
            .subject
            .contains("nové inzeráty"));
        assert!(AlertNotification::localized(Some("cs"), 12, "S")
            .subject
            .contains("nových inzerátů"));

        // German two-bucket.
        assert!(AlertNotification::localized(Some("de"), 1, "S")
            .subject
            .contains("neues Inserat passt"));
        assert!(AlertNotification::localized(Some("de"), 6, "S")
            .subject
            .contains("neue Inserate passen"));
    }
}
