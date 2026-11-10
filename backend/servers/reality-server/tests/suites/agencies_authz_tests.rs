//! Handler-level auth tests for agencies endpoints.
//!
//! Drives the real Axum router (routes::agencies) end-to-end via `oneshot`.
//!
//! Public endpoints (no RequestPrincipal): list_agencies, get_agency,
//! get_agency_by_slug — return non-401 without a token.
//!
//! Protected endpoints (RequestPrincipal): create_agency, update_agency,
//! list_members, create_invitation, accept_invitation — return 401 without a
//! token and non-401 with a seeded user token.

use crate::common::{make_app_state, mint_token, send, send_json};
use axum::{http::Method, Router};
use reality_server::routes;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

fn agencies_router(pool: PgPool) -> Router {
    let state = make_app_state(pool);
    Router::new()
        .nest("/api/v1/agencies", routes::agencies::router())
        .with_state(state)
}

async fn seed_user(pool: &PgPool, tag: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'hash', $2, 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(format!("{tag}@agencies-authz.test"))
    .bind(format!("AgenciesAuthz {tag}"))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_user({tag}): {e}"))
}

/// Seed a `platform`-kind user so the `RequestPrincipal` extractor clears the
/// tenant gate on the host-less test router and the request reaches the
/// handler's own membership check (mirrors the happy-path suite).
async fn seed_platform_user(pool: &PgPool, tag: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'hash', $2, 'active', NOW(), 'platform')
        RETURNING id
        "#,
    )
    .bind(format!("{tag}@agencies-authz.test"))
    .bind(format!("AgenciesAuthz {tag}"))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_platform_user({tag}): {e}"))
}

async fn seed_agency(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO reality_agencies (name, slug, email)
        VALUES ($1, $2, $3)
        RETURNING id
        "#,
    )
    .bind(format!("Agency {slug}"))
    .bind(format!("agency-{slug}"))
    .bind(format!("{slug}@agencies-authz.test"))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_agency({slug}): {e}"))
}

async fn seed_membership(pool: &PgPool, agency_id: Uuid, user_id: Uuid) {
    sqlx::query(
        "INSERT INTO reality_agency_members (agency_id, user_id, role) VALUES ($1, $2, 'owner')",
    )
    .bind(agency_id)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("seed_membership failed");
}

/// Seed a pending, unexpired invitation bound to `email`, returning its token.
async fn seed_invitation(
    pool: &PgPool,
    agency_id: Uuid,
    invited_by: Uuid,
    email: &str,
    token: &str,
) {
    sqlx::query(
        r#"
        INSERT INTO reality_agency_invitations
            (agency_id, email, role, invited_by, token, expires_at)
        VALUES ($1, $2, 'realtor', $3, $4, NOW() + INTERVAL '7 days')
        "#,
    )
    .bind(agency_id)
    .bind(email)
    .bind(invited_by)
    .bind(token)
    .execute(pool)
    .await
    .expect("seed_invitation failed");
}

async fn membership_count(pool: &PgPool, agency_id: Uuid, user_id: Uuid) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM reality_agency_members WHERE agency_id = $1 AND user_id = $2",
    )
    .bind(agency_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("membership_count failed")
}

// ── list_agencies (public) ────────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_agencies_unauthenticated_returns_non_401(pool: PgPool) {
    let app = agencies_router(pool);
    let status = send(&app, Method::GET, "/api/v1/agencies", None).await;
    assert_ne!(
        status, 401,
        "list_agencies must not require auth (public directory)"
    );
}

// ── get_agency (public) ───────────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_agency_unauthenticated_returns_non_401(pool: PgPool) {
    let id = Uuid::new_v4();
    let app = agencies_router(pool);
    let status = send(&app, Method::GET, &format!("/api/v1/agencies/{id}"), None).await;
    assert_ne!(
        status, 401,
        "get_agency must not require auth (may return 404 for unknown id)"
    );
}

// ── get_agency_by_slug (public) ───────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_agency_by_slug_unauthenticated_returns_non_401(pool: PgPool) {
    let app = agencies_router(pool);
    let status = send(
        &app,
        Method::GET,
        "/api/v1/agencies/by-slug/unknown-slug",
        None,
    )
    .await;
    assert_ne!(
        status, 401,
        "get_agency_by_slug must not require auth (may return 404)"
    );
}

// ── list_members (protected) ──────────────────────────────────────────────────

// SECURITY regression (members-idor audit): the member roster is agency-internal
// PII. `list_members` had NO auth extractor and NO membership gate, so anyone on
// the internet could enumerate any agency's members. An unauthenticated request
// must now be rejected with 401 (auth required), not served the roster.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_members_unauthenticated_returns_401(pool: PgPool) {
    let id = Uuid::new_v4();
    let app = agencies_router(pool);
    let status = send(
        &app,
        Method::GET,
        &format!("/api/v1/agencies/{id}/members"),
        None,
    )
    .await;
    assert_eq!(
        status, 401,
        "list_members must require auth (member roster is agency-internal PII), got {status}"
    );
}

// SECURITY regression (members-idor audit): an authenticated user who is NOT a
// member of the target agency must NOT be able to enumerate its members — the
// cross-agency IDOR. Uses a `platform` caller so the request clears the
// `RequestPrincipal` extractor and the 403 can only come from the handler's own
// `check_agency_membership` gate.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_members_non_member_returns_403(pool: PgPool) {
    let outsider = seed_platform_user(&pool, "members-outsider").await;
    let agency_id = seed_agency(&pool, "members-403").await;
    // NB: no membership seeded for `outsider` in this agency.
    let token = mint_token(outsider);
    let app = agencies_router(pool);
    let status = send(
        &app,
        Method::GET,
        &format!("/api/v1/agencies/{agency_id}/members"),
        Some(&token),
    )
    .await;
    assert_eq!(
        status, 403,
        "a non-member must NOT be able to enumerate an agency's members, got {status}"
    );
}

// Companion: the gate must not over-reject — an active member of the agency is
// allowed past the membership check and receives the roster (200).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_members_member_returns_200(pool: PgPool) {
    let member = seed_platform_user(&pool, "members-member").await;
    let agency_id = seed_agency(&pool, "members-ok").await;
    seed_membership(&pool, agency_id, member).await;
    let token = mint_token(member);
    let app = agencies_router(pool);
    let status = send(
        &app,
        Method::GET,
        &format!("/api/v1/agencies/{agency_id}/members"),
        Some(&token),
    )
    .await;
    assert_eq!(
        status, 200,
        "an active member must be able to list the agency's members, got {status}"
    );
}

// ── create_agency (protected) ─────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_agency_unauthenticated_returns_401(pool: PgPool) {
    let app = agencies_router(pool);
    let status = send_json(
        &app,
        Method::POST,
        "/api/v1/agencies",
        None,
        json!({"name": "Test Agency", "slug": "test-agency"}),
    )
    .await;
    assert_eq!(status, 401, "create_agency must return 401 without auth");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_agency_authenticated_returns_non_401(pool: PgPool) {
    let user = seed_user(&pool, "create-agency").await;
    let token = mint_token(user);
    let app = agencies_router(pool);
    let status = send_json(
        &app,
        Method::POST,
        "/api/v1/agencies",
        Some(&token),
        json!({"name": "Test Agency", "slug": "test-agency"}),
    )
    .await;
    assert_ne!(
        status, 401,
        "authenticated create_agency must not return 401"
    );
}

// ── update_agency (protected) ─────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_agency_unauthenticated_returns_401(pool: PgPool) {
    let id = Uuid::new_v4();
    let app = agencies_router(pool);
    let status = send_json(
        &app,
        Method::PUT,
        &format!("/api/v1/agencies/{id}"),
        None,
        json!({"name": "Updated"}),
    )
    .await;
    assert_eq!(status, 401, "update_agency must return 401 without auth");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_agency_authenticated_unknown_returns_non_401(pool: PgPool) {
    let user = seed_user(&pool, "update-agency").await;
    let token = mint_token(user);
    let id = Uuid::new_v4();
    let app = agencies_router(pool);
    let status = send_json(
        &app,
        Method::PUT,
        &format!("/api/v1/agencies/{id}"),
        Some(&token),
        json!({"name": "Updated"}),
    )
    .await;
    assert_ne!(
        status, 401,
        "authenticated update_agency must not return 401"
    );
}

// ── create_invitation (protected) ────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_invitation_unauthenticated_returns_401(pool: PgPool) {
    let id = Uuid::new_v4();
    let app = agencies_router(pool);
    let status = send_json(
        &app,
        Method::POST,
        &format!("/api/v1/agencies/{id}/invitations"),
        None,
        json!({"email": "invite@example.com", "role": "agent"}),
    )
    .await;
    assert_eq!(
        status, 401,
        "create_invitation must return 401 without auth"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_invitation_authenticated_unknown_agency_returns_non_401(pool: PgPool) {
    let user = seed_user(&pool, "create-inv").await;
    let token = mint_token(user);
    let id = Uuid::new_v4();
    let app = agencies_router(pool);
    let status = send_json(
        &app,
        Method::POST,
        &format!("/api/v1/agencies/{id}/invitations"),
        Some(&token),
        json!({"email": "invite@example.com", "role": "agent"}),
    )
    .await;
    assert_ne!(
        status, 401,
        "authenticated create_invitation must not return 401"
    );
}

// REGRESSION (invite-authz audit): `create_invitation` had NO membership
// gate, so any authenticated user could invite themselves/others into ANY
// agency. A non-member acting on an *existing* agency must be rejected with
// 403 — not allowed to create the invitation. Uses a `platform` caller so the
// request clears the `RequestPrincipal` extractor and the 403 can only come
// from the handler's own `check_agency_membership` gate.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_invitation_non_member_returns_403(pool: PgPool) {
    let outsider = seed_platform_user(&pool, "inv-outsider").await;
    let agency_id = seed_agency(&pool, "inv-403").await;
    // NB: no membership seeded for `outsider` in this agency.
    let token = mint_token(outsider);
    let app = agencies_router(pool);
    let status = send_json(
        &app,
        Method::POST,
        &format!("/api/v1/agencies/{agency_id}/invitations"),
        Some(&token),
        json!({"email": "invite@example.com", "role": "agent"}),
    )
    .await;
    assert_eq!(
        status, 403,
        "a non-member must NOT be able to create invitations for an agency, got {status}"
    );
}

// Companion: the gate must not over-reject — an active member of the agency
// is allowed past the membership check (i.e. does NOT get 403).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_invitation_member_not_forbidden(pool: PgPool) {
    let member = seed_platform_user(&pool, "inv-member").await;
    let agency_id = seed_agency(&pool, "inv-member").await;
    seed_membership(&pool, agency_id, member).await;
    let token = mint_token(member);
    let app = agencies_router(pool);
    let status = send_json(
        &app,
        Method::POST,
        &format!("/api/v1/agencies/{agency_id}/invitations"),
        Some(&token),
        json!({"email": "invite@example.com", "role": "agent"}),
    )
    .await;
    assert_ne!(
        status, 403,
        "an active member must pass the membership gate, got {status}"
    );
}

// ── accept_invitation (protected) ────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn accept_invitation_unauthenticated_returns_401(pool: PgPool) {
    let app = agencies_router(pool);
    let status = send(
        &app,
        Method::POST,
        "/api/v1/agencies/invitations/fake-token/accept",
        None,
    )
    .await;
    assert_eq!(
        status, 401,
        "accept_invitation must return 401 without auth"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn accept_invitation_authenticated_unknown_token_returns_non_401(pool: PgPool) {
    let user = seed_user(&pool, "accept-inv").await;
    let token = mint_token(user);
    let app = agencies_router(pool);
    let status = send(
        &app,
        Method::POST,
        "/api/v1/agencies/invitations/fake-token/accept",
        Some(&token),
    )
    .await;
    assert_ne!(
        status, 401,
        "authenticated accept_invitation must not return 401"
    );
}

/// SECURITY regression (invite-email-match): a different logged-in user who
/// obtains a valid, unexpired invite token must NOT be able to join the agency.
/// The recipient gate must reject with 403 and create no membership. Before the
/// fix this returned 200 and added the wrong user as a member.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn accept_invitation_email_mismatch_returns_403_and_adds_no_member(pool: PgPool) {
    let inviter = seed_user(&pool, "mismatch-inviter").await;
    let agency = seed_agency(&pool, "mismatch").await;
    let token = "invite-token-mismatch";
    // Invitation issued to a target that is NOT the accepting user.
    seed_invitation(
        &pool,
        agency,
        inviter,
        "intended@agencies-authz.test",
        token,
    )
    .await;

    // A different, unrelated logged-in user holding the token.
    let attacker = seed_platform_user(&pool, "mismatch-attacker").await;
    let attacker_token = mint_token(attacker);

    let app = agencies_router(pool.clone());
    let status = send(
        &app,
        Method::POST,
        &format!("/api/v1/agencies/invitations/{token}/accept"),
        Some(&attacker_token),
    )
    .await;

    assert_eq!(
        status, 403,
        "accept_invitation must reject a non-recipient with 403, got {status}"
    );
    assert_eq!(
        membership_count(&pool, agency, attacker).await,
        0,
        "a rejected accept must not create membership for the wrong user"
    );
}

/// Happy path for the recipient gate: the invited email's owner accepts and
/// becomes a member (200). Guards against the fix over-rejecting legitimate
/// acceptances.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn accept_invitation_matching_email_succeeds(pool: PgPool) {
    let inviter = seed_user(&pool, "match-inviter").await;
    let agency = seed_agency(&pool, "match").await;
    // `seed_platform_user` stores email `{tag}@agencies-authz.test`; issue the
    // invitation to that exact address so the recipient gate passes.
    let recipient = seed_platform_user(&pool, "match-recipient").await;
    let recipient_email = "match-recipient@agencies-authz.test";
    let token = "invite-token-match";
    seed_invitation(&pool, agency, inviter, recipient_email, token).await;

    let recipient_token = mint_token(recipient);
    let app = agencies_router(pool.clone());
    let status = send(
        &app,
        Method::POST,
        &format!("/api/v1/agencies/invitations/{token}/accept"),
        Some(&recipient_token),
    )
    .await;

    assert_eq!(
        status, 200,
        "the invited recipient must be able to accept, got {status}"
    );
    assert_eq!(
        membership_count(&pool, agency, recipient).await,
        1,
        "a successful accept must create exactly one membership row"
    );
}
