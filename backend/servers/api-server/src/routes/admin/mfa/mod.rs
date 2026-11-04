//! Admin MFA enrollment + recovery-code routes. Phase 6 (B6).
//!
//! These endpoints are for *platform principals* only. They do NOT go through
//! the `RequireCapability` gate because enrollment itself is a precondition to
//! that gate — you cannot satisfy MFA if you have never enrolled.
//!
//! Endpoints:
//!   * `POST /admin/mfa/enroll/start`   — generate TOTP secret, return QR URI
//!   * `POST /admin/mfa/enroll/verify`  — confirm code, activate, issue recovery codes
//!   * `POST /admin/mfa/verify`         — TOTP step-up for enrolled users (opens recency window)
//!   * `POST /admin/mfa/recovery/use`   — consume a recovery code (lockout fallback)
//!   * `POST /admin/mfa/disable`        — clear enrollment (requires fresh MFA or recovery code)

mod disable;
mod enroll;
mod recovery;
mod verify;

pub use disable::disable_mfa;
pub use enroll::{start_enroll, verify_enroll};
pub use recovery::use_recovery;
pub use verify::verify_step_up;

use std::sync::Arc;

use axum::{routing::post, Router};
use uuid::Uuid;

use admin_core::{AuditOutcome, AuditWriter, Capability};

use crate::state::AppState;

/// Sub-router mounted at `/api/v1/admin/mfa`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/enroll/start", post(start_enroll))
        .route("/enroll/verify", post(verify_enroll))
        .route("/verify", post(verify_step_up))
        .route("/recovery/use", post(use_recovery))
        .route("/disable", post(disable_mfa))
}

/// Best-effort audit write shared by all MFA security-event handlers
/// (enroll / disable / verify / recovery).
///
/// Centralising the write keeps every MFA handler on the same contract:
/// the audit row is best-effort (an audit-store outage must never brick the
/// MFA path itself) but a failure is **surfaced at WARN rather than silently
/// dropped**. Mirrors how the sibling admin security-event handlers
/// (`users.rs`, `capabilities.rs`, `impersonation.rs`) treat their audit
/// writes — `if let Err(e) = audit.record(..) { tracing::warn!(..) }` — so a
/// DB failure that loses an MFA enroll/disable/verify/recovery record is at
/// least observable instead of being swallowed by a bare `let _ = …`.
pub(super) async fn audit_mfa_event(
    audit: &Arc<dyn AuditWriter>,
    actor: Uuid,
    outcome: AuditOutcome,
    target_type: &str,
    target_id: Uuid,
    payload: Option<&serde_json::Value>,
) {
    if let Err(e) = audit
        .record(
            Some(actor),
            Capability::UsersWrite,
            outcome,
            Some(target_type),
            Some(target_id),
            payload,
            None,
            None,
        )
        .await
    {
        tracing::warn!(
            error = %e,
            actor = %actor,
            target = %target_type,
            "MFA security-event audit write failed",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::{Arc, Mutex};

    use admin_core::{AdminError, AuditOutcome, AuditWriter, Capability, RecordedAuditEvent};
    use tracing::Level;
    use tracing_subscriber::layer::{Context, SubscriberExt};
    use tracing_subscriber::Layer;
    use uuid::Uuid;

    /// An `AuditWriter` whose `record` always fails, simulating an
    /// audit-store (DB) outage.
    struct FailingAuditWriter;

    #[async_trait::async_trait]
    impl AuditWriter for FailingAuditWriter {
        async fn record(
            &self,
            _actor_id: Option<Uuid>,
            _capability: Capability,
            _outcome: AuditOutcome,
            _target_type: Option<&str>,
            _target_id: Option<Uuid>,
            _payload: Option<&serde_json::Value>,
            _ip: Option<&str>,
            _user_agent: Option<&str>,
        ) -> Result<RecordedAuditEvent, AdminError> {
            Err(AdminError::Internal("audit store down".into()))
        }
    }

    /// Minimal tracing layer that records the level of every event emitted
    /// while it is the default subscriber.
    #[derive(Clone, Default)]
    struct LevelCapture(Arc<Mutex<Vec<Level>>>);

    impl<S: tracing::Subscriber> Layer<S> for LevelCapture {
        fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
            self.0.lock().unwrap().push(*event.metadata().level());
        }
    }

    /// Regression test for the "fire-and-forget MFA audit" finding
    /// (`code-review-api-handlers-mfa-audit-discard`).
    ///
    /// Before the fix the MFA handlers wrote their security-event audit rows
    /// with a bare `let _ = audit.record(..).await;`, so a DB failure was
    /// silently discarded — no error return, no log. This asserts that when
    /// the audit store fails, `audit_mfa_event` surfaces the failure at WARN
    /// instead of dropping it. It FAILS against the old silent-drop body
    /// (no event is emitted) and PASSES once the failure is logged.
    #[tokio::test]
    async fn audit_write_failure_is_logged_not_silently_dropped() {
        let capture = LevelCapture::default();
        let events = capture.0.clone();

        let writer: Arc<dyn AuditWriter> = Arc::new(FailingAuditWriter);
        let actor = Uuid::new_v4();

        {
            let subscriber = tracing_subscriber::registry().with(capture);
            let _guard = tracing::subscriber::set_default(subscriber);

            audit_mfa_event(
                &writer,
                actor,
                AuditOutcome::Allowed,
                "mfa_test_event",
                actor,
                None,
            )
            .await;
        }

        let levels = events.lock().unwrap();
        assert!(
            levels.contains(&Level::WARN),
            "a failed MFA audit write must emit a WARN event, not be silently \
             dropped — captured events: {levels:?}"
        );
    }
}
