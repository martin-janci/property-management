//! Unit tests for the impersonation service.
//!
//! These tests exercise behavior that does not require Postgres (no real
//! `start_impersonation` round-trip). For full DB-level coverage see the
//! integration tests in `backend/servers/api-server/tests/`.

use admin_core::{
    AdminError, AuditOutcome, AuditWriter, Capability, ImpersonationService, ImpersonationToken,
    IssuedImpersonationToken, RecordedAuditEvent, IMPERSONATION_TTL,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Default)]
struct CountingAudit {
    impersonate_events: AtomicUsize,
    last: Mutex<Option<(Option<Uuid>, Option<Uuid>)>>,
}

#[async_trait]
impl AuditWriter for CountingAudit {
    async fn record(
        &self,
        actor_id: Option<Uuid>,
        capability: Capability,
        _outcome: AuditOutcome,
        _target_type: Option<&str>,
        target_id: Option<Uuid>,
        _payload: Option<&serde_json::Value>,
        _ip: Option<&str>,
        _user_agent: Option<&str>,
    ) -> Result<RecordedAuditEvent, AdminError> {
        if matches!(capability, Capability::UsersImpersonate) {
            self.impersonate_events.fetch_add(1, Ordering::SeqCst);
            *self.last.lock().unwrap() = Some((actor_id, target_id));
        }
        Ok(RecordedAuditEvent {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
        })
    }
}

#[derive(Default, Clone)]
struct InMemoryImpersonation {
    tokens: Arc<Mutex<Vec<ImpersonationToken>>>,
    audit: Arc<CountingAudit>,
}

#[async_trait]
impl ImpersonationService for InMemoryImpersonation {
    async fn start_impersonation(
        &self,
        actor: Uuid,
        target: Uuid,
        capability: Capability,
    ) -> Result<IssuedImpersonationToken, AdminError> {
        if actor == target {
            return Err(AdminError::Internal("no_self_impersonation".into()));
        }
        let now = Utc::now();
        let record = ImpersonationToken {
            id: Uuid::new_v4(),
            token_hash: format!("h_{}", Uuid::new_v4()),
            actor_id: actor,
            target_user_id: target,
            justifying_capability: capability.as_str().to_string(),
            issued_at: now,
            expires_at: now + IMPERSONATION_TTL,
            ended_at: None,
        };
        self.tokens.lock().unwrap().push(record.clone());
        self.audit
            .record(
                Some(actor),
                Capability::UsersImpersonate,
                AuditOutcome::Allowed,
                Some("user"),
                Some(target),
                None,
                None,
                None,
            )
            .await?;
        Ok(IssuedImpersonationToken {
            record,
            plain_token: format!("plain_{}", Uuid::new_v4()),
        })
    }

    async fn end_impersonation(&self, token_id: Uuid, _actor: Uuid) -> Result<(), AdminError> {
        let mut tokens = self.tokens.lock().unwrap();
        for t in tokens.iter_mut() {
            if t.id == token_id && t.ended_at.is_none() {
                t.ended_at = Some(Utc::now());
            }
        }
        Ok(())
    }

    async fn resolve(&self, plain_token: &str) -> Result<Option<ImpersonationToken>, AdminError> {
        // The in-memory impl doesn't actually hash; for the resolve test
        // below we use a different path.
        let _ = plain_token;
        Ok(None)
    }
}

#[tokio::test]
async fn start_impersonation_writes_audit_linking_actor_to_target() {
    let svc = InMemoryImpersonation::default();
    let actor = Uuid::new_v4();
    let target = Uuid::new_v4();

    let issued = svc
        .start_impersonation(actor, target, Capability::UsersImpersonate)
        .await
        .expect("start_impersonation should succeed");

    assert_eq!(issued.record.actor_id, actor);
    assert_eq!(issued.record.target_user_id, target);
    assert!(!issued.plain_token.is_empty());

    // Audit row links impersonator → target.
    assert_eq!(svc.audit.impersonate_events.load(Ordering::SeqCst), 1);
    let (audited_actor, audited_target) = svc.audit.last.lock().unwrap().unwrap();
    assert_eq!(audited_actor, Some(actor));
    assert_eq!(audited_target, Some(target));
}

#[tokio::test]
async fn no_self_impersonation() {
    let svc = InMemoryImpersonation::default();
    let me = Uuid::new_v4();
    let res = svc
        .start_impersonation(me, me, Capability::UsersImpersonate)
        .await;
    assert!(matches!(res, Err(AdminError::Internal(_))));
    // No audit row written for the rejected attempt at this layer (the
    // capability gate is what audits denials).
    assert_eq!(svc.audit.impersonate_events.load(Ordering::SeqCst), 0);
}

#[test]
fn token_ttl_is_15_minutes() {
    assert_eq!(IMPERSONATION_TTL.num_minutes(), 15);
}

#[tokio::test]
async fn end_impersonation_marks_token_ended() {
    let svc = InMemoryImpersonation::default();
    let issued = svc
        .start_impersonation(Uuid::new_v4(), Uuid::new_v4(), Capability::UsersImpersonate)
        .await
        .unwrap();
    let id = issued.record.id;

    svc.end_impersonation(id, issued.record.actor_id)
        .await
        .unwrap();

    let tokens = svc.tokens.lock().unwrap();
    let row = tokens.iter().find(|t| t.id == id).unwrap();
    assert!(row.ended_at.is_some());
}

#[test]
fn impersonation_token_serializes_for_api_response() {
    // The handler returns ImpersonationToken via JSON. Confirm it has the
    // shape the frontend expects: id, target, expires_at, etc.
    let now = Utc::now();
    let t = ImpersonationToken {
        id: Uuid::new_v4(),
        token_hash: "h".into(),
        actor_id: Uuid::new_v4(),
        target_user_id: Uuid::new_v4(),
        justifying_capability: Capability::UsersImpersonate.as_str().into(),
        issued_at: now,
        expires_at: now + IMPERSONATION_TTL,
        ended_at: None,
    };
    let json = serde_json::to_string(&t).unwrap();
    for required in [
        "id",
        "target_user_id",
        "expires_at",
        "justifying_capability",
    ] {
        assert!(
            json.contains(required),
            "missing field {} in {}",
            required,
            json
        );
    }
}

#[allow(dead_code)]
fn _datetime_check(_: DateTime<Utc>) {} // silence unused import warning if Utc is not used elsewhere
