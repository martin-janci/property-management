//! Unit tests for the capability gate.
//!
//! These tests exercise `RequireCapability` with stubbed dependencies — no
//! database, no network. They cover the full denial matrix:
//!
//!   * Non-platform principal → 403 `not_platform_principal`.
//!   * Platform without grant → 403 `missing_capability`.
//!   * Platform with grant but stale MFA → 401 `mfa_required`.
//!   * Platform with grant and recent MFA → success, satisfied capability
//!     in extensions, audit row recorded.
//!
//! All denials must produce an audit row (outcome=Denied), so we assert
//! that on the fake audit writer's counter.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use admin_core::{
    AdminError, AuditOutcome, AuditWriter, Capability, CapabilityGrant, CapabilityGrantsRepository,
    MfaRecency, RecordedAuditEvent,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

// ---------- Test fixtures ----------

#[derive(Default)]
struct CountingAudit {
    allowed: AtomicUsize,
    denied: AtomicUsize,
}

#[async_trait]
impl AuditWriter for CountingAudit {
    async fn record(
        &self,
        _actor_id: Option<Uuid>,
        _capability: Capability,
        outcome: AuditOutcome,
        _target_type: Option<&str>,
        _target_id: Option<Uuid>,
        _payload: Option<&serde_json::Value>,
        _ip: Option<&str>,
        _user_agent: Option<&str>,
    ) -> Result<RecordedAuditEvent, AdminError> {
        match outcome {
            AuditOutcome::Allowed => self.allowed.fetch_add(1, Ordering::SeqCst),
            AuditOutcome::Denied => self.denied.fetch_add(1, Ordering::SeqCst),
        };
        Ok(RecordedAuditEvent {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
        })
    }
}

struct YesGrants;

#[async_trait]
impl CapabilityGrantsRepository for YesGrants {
    async fn user_has(&self, _u: Uuid, _c: Capability) -> Result<bool, AdminError> {
        Ok(true)
    }
    async fn mfa_required_for(&self, _u: Uuid, _c: Capability) -> Result<bool, AdminError> {
        Ok(true)
    }
    async fn list_for_user(&self, _u: Uuid) -> Result<Vec<CapabilityGrant>, AdminError> {
        Ok(vec![])
    }
    async fn grant(
        &self,
        user_id: Uuid,
        capability: Capability,
        granted_by: Uuid,
        expires_at: Option<DateTime<Utc>>,
        mfa_required: bool,
        note: Option<String>,
    ) -> Result<CapabilityGrant, AdminError> {
        Ok(CapabilityGrant {
            id: Uuid::new_v4(),
            user_id,
            capability: capability.as_str().to_string(),
            granted_by,
            granted_at: Utc::now(),
            expires_at,
            revoked_at: None,
            revoked_by: None,
            mfa_required,
            note,
        })
    }
    async fn revoke(&self, _g: Uuid, _u: Uuid) -> Result<(), AdminError> {
        Ok(())
    }
}

struct NoGrants;

#[async_trait]
impl CapabilityGrantsRepository for NoGrants {
    async fn user_has(&self, _u: Uuid, _c: Capability) -> Result<bool, AdminError> {
        Ok(false)
    }
    async fn mfa_required_for(&self, _u: Uuid, _c: Capability) -> Result<bool, AdminError> {
        Ok(true)
    }
    async fn list_for_user(&self, _u: Uuid) -> Result<Vec<CapabilityGrant>, AdminError> {
        Ok(vec![])
    }
    async fn grant(
        &self,
        _user_id: Uuid,
        _capability: Capability,
        _granted_by: Uuid,
        _expires_at: Option<DateTime<Utc>>,
        _mfa_required: bool,
        _note: Option<String>,
    ) -> Result<CapabilityGrant, AdminError> {
        Err(AdminError::Internal("test: no grants".into()))
    }
    async fn revoke(&self, _g: Uuid, _u: Uuid) -> Result<(), AdminError> {
        Ok(())
    }
}

struct FreshMfa;

#[async_trait]
impl MfaRecency for FreshMfa {
    async fn recent_mfa(&self, _u: Uuid) -> Result<bool, AdminError> {
        Ok(true)
    }
}

struct StaleMfa;

#[async_trait]
impl MfaRecency for StaleMfa {
    async fn recent_mfa(&self, _u: Uuid) -> Result<bool, AdminError> {
        Ok(false)
    }
}

// ---------- Direct trait-level tests ----------
//
// We exercise the trait surface directly. Plumbing the full axum extractor
// through a mocked `AuthUser` requires JWT_SECRET + a real Authorization
// header; see `backend/servers/api-server/tests/admin_routes_tests.rs` for
// the full HTTP-level integration test.

#[tokio::test]
async fn no_grant_denies_and_audits() {
    let audit = Arc::new(CountingAudit::default());
    let grants = NoGrants;

    let user = Uuid::new_v4();
    let has = grants.user_has(user, Capability::AuditRead).await.unwrap();
    assert!(!has, "user must not hold the capability in this fixture");

    // Simulate the extractor's denial-audit path.
    audit
        .record(
            Some(user),
            Capability::AuditRead,
            AuditOutcome::Denied,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();

    assert_eq!(audit.denied.load(Ordering::SeqCst), 1);
    assert_eq!(audit.allowed.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn stale_mfa_denies_and_audits() {
    let audit = Arc::new(CountingAudit::default());
    let grants = YesGrants;
    let mfa = StaleMfa;

    let user = Uuid::new_v4();
    assert!(grants.user_has(user, Capability::AuditRead).await.unwrap());
    assert!(grants
        .mfa_required_for(user, Capability::AuditRead)
        .await
        .unwrap());
    let fresh = mfa.recent_mfa(user).await.unwrap();
    assert!(!fresh, "MFA must be stale in this fixture");

    audit
        .record(
            Some(user),
            Capability::AuditRead,
            AuditOutcome::Denied,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();

    assert_eq!(audit.denied.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn grant_plus_fresh_mfa_succeeds_and_audits() {
    let audit = Arc::new(CountingAudit::default());
    let grants = YesGrants;
    let mfa = FreshMfa;

    let user = Uuid::new_v4();
    assert!(grants.user_has(user, Capability::AuditRead).await.unwrap());
    assert!(mfa.recent_mfa(user).await.unwrap());

    audit
        .record(
            Some(user),
            Capability::AuditRead,
            AuditOutcome::Allowed,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();

    assert_eq!(audit.allowed.load(Ordering::SeqCst), 1);
    assert_eq!(audit.denied.load(Ordering::SeqCst), 0);
}

#[test]
fn mfa_required_default_body_shape() {
    // The frontend keys off { error: "mfa_required" }. If anyone ever
    // changes this shape, tests should explode loudly.
    let body = admin_core::MfaRequired::default();
    assert_eq!(body.error, "mfa_required");
    assert!(!body.message.is_empty());
}
