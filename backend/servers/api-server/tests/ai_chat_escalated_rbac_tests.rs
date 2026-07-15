//! Regression tests for the RBAC gate on `GET /api/v1/ai/chat/escalated`
//! (issue #2317, follow-up to #2279/#2289).
//!
//! `list_escalated` returns the full content of ALL escalated AI-chat messages
//! org-wide via `list_escalated_messages(org_id)` — a deliberately cross-user
//! read (escalation review). AI chat sessions are per-user private within an
//! org (#2279), so before #2317 any authenticated org member — including a
//! plain resident — could read other members' escalated messages, partially
//! reopening the confidentiality exposure #2289 closed.
//!
//! The fix adds an `rls.role().is_manager()` check inside the handler (the
//! same pattern as the report-schedule handlers, see
//! `report_schedule_rbac_tests.rs`).
//!
//! # What these tests verify
//!
//! 1. **RBAC denial** — `list_escalated_as_resident_is_rejected`: a user
//!    authenticated with a *real* JWT whose DB-backed membership role is
//!    `resident` gets past `AuthUser` and `ValidatedTenantExtractor`, reaches
//!    the handler, and is rejected by the `rls.role().is_manager()` branch
//!    with a strict `403 FORBIDDEN` — proving the RBAC predicate fires, not
//!    the outer JWT gate.
//! 2. **Manager allow** — `list_escalated_as_manager_succeeds`: the same
//!    request from a `manager`-role member returns `200 OK` with the seeded
//!    escalated message, proving the gate does not lock out the legitimate
//!    escalation-review path.
//! 3. **JWT gate** — `list_escalated_without_auth_is_rejected`: no
//!    Authorization header at all is rejected with 401/403 before any handler
//!    logic runs.

#![allow(dead_code)]

mod common;

use axum::http::StatusCode;
use sqlx::PgPool;
use uuid::Uuid;

use common::{create_authenticated_user, seed_membership, seed_org, TestApp, TestUser};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// Look up a user id by email (the user is created by
/// `create_authenticated_user` via `POST /api/v1/auth/register`).
async fn user_id_for(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("fetch user id")
}

/// Seed a session owned by `user_id` with one escalated message; returns the
/// message id.
async fn seed_escalated_message(pool: &PgPool, org_id: Uuid, user_id: Uuid) -> Uuid {
    let session = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO ai_chat_sessions (organization_id, user_id, title)
        VALUES ($1, $2, 'escalation rbac session') RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed session");

    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO ai_chat_messages (session_id, role, content, escalated, escalation_reason)
        VALUES ($1, 'assistant', 'sensitive escalated answer', TRUE, 'needs human')
        RETURNING id
        "#,
    )
    .bind(session)
    .fetch_one(pool)
    .await
    .expect("seed escalated message")
}

// ---------------------------------------------------------------------------
// Test 1 — #2317: an authenticated Resident gets 403 from the RBAC check
// ---------------------------------------------------------------------------

/// A user with a real Bearer JWT whose DB-backed membership role is
/// `resident` requests the org-wide escalated-message list. The request
/// passes `AuthUser` and `ValidatedTenantExtractor` and reaches the handler;
/// the handler's `if !rls.role().is_manager()` branch fires and returns a
/// strict `403 FORBIDDEN` with the `FORBIDDEN` error code — the escalated
/// content of other members' private sessions is never returned.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_escalated_as_resident_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org = seed_org(&pool, "esc-rbac").await;

    // Another member's escalated message exists in the org — this is exactly
    // the content the resident must NOT be able to read.
    let victim = seed_user_raw(&pool, "esc-victim@ai-esc-rbac.test").await;
    let _message = seed_escalated_message(&pool, org, victim).await;

    // Register + login a real user, then attach them to `org` as a resident.
    let resident = TestUser::new();
    let (access_token, _) = create_authenticated_user(&app, &resident).await;
    let resident_id = user_id_for(&pool, &resident.email).await;
    seed_membership(&pool, org, resident_id, "resident").await;

    let response = app
        .execute(
            app.get("/api/v1/ai/chat/escalated")
                .bearer(&access_token)
                .tenant(org)
                .build(),
        )
        .await;

    // The RBAC check inside the handler must fire — not the outer JWT gate.
    assert_eq!(
        response.status,
        StatusCode::FORBIDDEN,
        "#2317: authenticated resident must be rejected by the RBAC check \
         (`rls.role().is_manager()`) with 403, got {} body={}",
        response.status,
        response.text(),
    );

    let body: serde_json::Value = response.json_value();
    let code = body
        .get("code")
        .and_then(|c| c.as_str())
        .unwrap_or_default();
    assert_eq!(
        code, "FORBIDDEN",
        "#2317: 403 response must carry the FORBIDDEN code, body={}",
        body
    );
}

// ---------------------------------------------------------------------------
// Test 2 — #2317: a Manager gets 200 with the escalated messages
// ---------------------------------------------------------------------------

/// The legitimate escalation-review path: a `manager`-role member requests
/// the escalated list and receives `200 OK` including the seeded escalated
/// message from another member's session (escalation review is deliberately
/// cross-user — that is why the endpoint is role-gated rather than
/// owner-scoped).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_escalated_as_manager_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org = seed_org(&pool, "esc-mgr").await;

    let member = seed_user_raw(&pool, "esc-member@ai-esc-rbac.test").await;
    let message = seed_escalated_message(&pool, org, member).await;

    let manager = TestUser::new();
    let (access_token, _) = create_authenticated_user(&app, &manager).await;
    let manager_id = user_id_for(&pool, &manager.email).await;
    seed_membership(&pool, org, manager_id, "manager").await;

    let response = app
        .execute(
            app.get("/api/v1/ai/chat/escalated")
                .bearer(&access_token)
                .tenant(org)
                .build(),
        )
        .await;

    assert_eq!(
        response.status,
        StatusCode::OK,
        "#2317: a manager must be able to review escalated messages, got {} body={}",
        response.status,
        response.text(),
    );

    let body: serde_json::Value = response.json_value();
    let messages = body
        .get("messages")
        .and_then(|m| m.as_array())
        .expect("response has a messages array");
    assert!(
        messages
            .iter()
            .any(|m| m.get("id").and_then(|id| id.as_str()) == Some(message.to_string().as_str())),
        "#2317: the manager's escalated list must include the seeded escalated \
         message, body={}",
        body
    );
}

// ---------------------------------------------------------------------------
// Test 3 — no auth header at all → 401/403 from the auth gate
// ---------------------------------------------------------------------------

/// Unauthenticated request must be rejected by the auth layer before any
/// handler logic runs (same contract as `ai_auth_tests.rs`).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_escalated_without_auth_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let response = app
        .execute(app.get("/api/v1/ai/chat/escalated").build())
        .await;

    assert!(
        response.status == StatusCode::UNAUTHORIZED || response.status == StatusCode::FORBIDDEN,
        "unauthenticated request must be rejected with 401/403, got {}",
        response.status,
    );
}

/// Seed a bare user row (no auth flow needed — the row only exists so the
/// escalated message has a real owner distinct from the caller).
async fn seed_user_raw(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'Esc RBAC User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}
