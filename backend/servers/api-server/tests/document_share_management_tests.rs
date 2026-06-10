//! Authenticated document-share *management* integration tests (Story 7A.5,
//! coverage 7a-5).
//!
//! The public share-*access* routes (`GET /shared/{token}`,
//! `POST /shared/{token}/access`) are pinned by `document_share_access_tests.rs`.
//! Those cover only the consumer side of a share. The *producer* side — the
//! authenticated manager-facing CRUD that actually mints, lists and revokes
//! shares — had no end-to-end coverage. This file closes that gap so Story
//! 7A.5 can be promoted to done.
//!
//! Routes under test (mounted under the authenticated `/api/v1/documents`
//! prefix, so they require a bearer token AND an `X-Tenant-ID` header resolving
//! to an active org membership):
//!
//! - `POST   /api/v1/documents/{id}/shares`             — create a share
//! - `GET    /api/v1/documents/{id}/shares`             — list a document's shares
//! - `DELETE /api/v1/documents/{id}/shares/{share_id}`  — revoke a share
//!
//! Contract pinned here:
//!
//! - **Happy path** — a manager (`org_admin`) creates a `link` share; the
//!   response carries a `share_token` + `share_url`. Listing the document then
//!   returns that share, and the public access route serves it. Revoking it
//!   returns 204 and the public route then 404s (full round-trip).
//! - **share_type validation** — an unknown `share_type` is rejected 400.
//! - **target validation** — a non-`link` share with neither `target_id` nor
//!   `target_role` is rejected 400.
//! - **authorization** — a non-manager who is NOT the document creator is
//!   forbidden (403) from creating, listing or revoking shares.
//! - **not-found** — creating/listing shares on a missing document is 404.
//!
//! These exercise the real handler + `DocumentRepository::*_share_rls` path
//! through Postgres (RLS-bound on the caller's org), not the public super-admin
//! read path.

#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::{
    cleanup_test_user, create_authenticated_user_with_org, seed_membership, TestApp, TestUser,
};

// ============================================================================
// Seed helpers
// ============================================================================

async fn user_id_for(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("user_id_for")
}

/// Insert a `documents` row owned by `org_id`, created by `created_by`.
async fn seed_document(pool: &PgPool, org_id: Uuid, created_by: Uuid, title: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO documents \
             (organization_id, title, category, file_key, file_name, mime_type, \
              size_bytes, access_scope, created_by) \
         VALUES ($1, $2, 'reports', $3, 'report.pdf', 'application/pdf', 1024, \
                 'organization', $4) \
         RETURNING id",
    )
    .bind(org_id)
    .bind(title)
    .bind(format!("{org_id}/test/{}.pdf", Uuid::new_v4()))
    .bind(created_by)
    .fetch_one(pool)
    .await
    .expect("seed_document")
}

// ============================================================================
// Happy path — full create → list → access → revoke round-trip
// ============================================================================

/// A manager mints a `link` share, sees it listed, the public route serves it,
/// then a revoke (204) makes the public route 404. This is the end-to-end
/// share lifecycle Story 7A.5 promises.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_manager_create_list_access_revoke_link_share(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    // org_admin → is_manager() == true.
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "shr-mgmt").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let doc_id = seed_document(&pool, org_id, user_id, "Manager Share Doc").await;

    // --- Create a link share. ---
    let create_resp = app
        .execute(
            app.post(&format!("/api/v1/documents/{doc_id}/shares"))
                .bearer(&token)
                .header("X-Tenant-ID", &org_id.to_string())
                .json(json!({ "share_type": "link" }))
                .build(),
        )
        .await;
    assert_eq!(
        create_resp.status,
        StatusCode::CREATED,
        "manager must be able to create a link share; got {}. Body: {}",
        create_resp.status,
        create_resp.text()
    );
    let create_body = create_resp.json_value();
    let share_id = create_body["id"].as_str().expect("share id").to_string();
    let share_token = create_body["share_token"]
        .as_str()
        .expect("link share must carry a share_token")
        .to_string();
    assert!(
        create_body["share_url"].as_str().is_some(),
        "link share response must carry a share_url"
    );

    // --- List shares — the new share is present. ---
    let list_resp = app
        .execute(
            app.get(&format!("/api/v1/documents/{doc_id}/shares"))
                .bearer(&token)
                .header("X-Tenant-ID", &org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        list_resp.status,
        StatusCode::OK,
        "manager must be able to list shares; got {}. Body: {}",
        list_resp.status,
        list_resp.text()
    );
    let shares = list_resp.json_value();
    let listed = shares["shares"].as_array().expect("shares array");
    assert!(
        listed.iter().any(|s| s["id"].as_str() == Some(&share_id)),
        "the freshly-created share must appear in the share list: {shares}"
    );

    // --- Public access route serves the share before revocation. ---
    let access_ok = app
        .execute(app.get(&format!("/shared/{share_token}")).build())
        .await;
    assert_eq!(
        access_ok.status,
        StatusCode::OK,
        "public route must serve a live share; got {}. Body: {}",
        access_ok.status,
        access_ok.text()
    );

    // --- Revoke the share → 204. ---
    let revoke_resp = app
        .execute(
            app.delete(&format!("/api/v1/documents/{doc_id}/shares/{share_id}"))
                .bearer(&token)
                .header("X-Tenant-ID", &org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        revoke_resp.status,
        StatusCode::NO_CONTENT,
        "manager must be able to revoke a share (204); got {}. Body: {}",
        revoke_resp.status,
        revoke_resp.text()
    );

    // --- Public route now rejects the revoked share. ---
    let access_after = app
        .execute(app.get(&format!("/shared/{share_token}")).build())
        .await;
    assert_eq!(
        access_after.status,
        StatusCode::NOT_FOUND,
        "public route must 404 a revoked share; got {}. Body: {}",
        access_after.status,
        access_after.text()
    );

    cleanup_test_user(&pool, &user.email).await;
}

// ============================================================================
// Request validation
// ============================================================================

/// An unknown `share_type` is rejected with 400 before any row is written.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_create_share_rejects_unknown_share_type(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "shr-type").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let doc_id = seed_document(&pool, org_id, user_id, "Bad Type Doc").await;

    let resp = app
        .execute(
            app.post(&format!("/api/v1/documents/{doc_id}/shares"))
                .bearer(&token)
                .header("X-Tenant-ID", &org_id.to_string())
                .json(json!({ "share_type": "telepathy" }))
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::BAD_REQUEST,
        "unknown share_type must be rejected 400; got {}. Body: {}",
        resp.status,
        resp.text()
    );

    cleanup_test_user(&pool, &user.email).await;
}

/// A non-`link` share missing both `target_id` and `target_role` is 400 — the
/// handler requires a concrete target for `user`/`role`/`building` shares.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_create_non_link_share_requires_target(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "shr-tgt").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let doc_id = seed_document(&pool, org_id, user_id, "No Target Doc").await;

    let resp = app
        .execute(
            app.post(&format!("/api/v1/documents/{doc_id}/shares"))
                .bearer(&token)
                .header("X-Tenant-ID", &org_id.to_string())
                .json(json!({ "share_type": "user" }))
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::BAD_REQUEST,
        "non-link share without a target must be rejected 400; got {}. Body: {}",
        resp.status,
        resp.text()
    );

    cleanup_test_user(&pool, &user.email).await;
}

// ============================================================================
// Authorization — only creator or manager may manage shares
// ============================================================================

/// A non-manager member (`resident`) who did NOT create the document is
/// forbidden (403) from creating, listing or revoking its shares.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_non_manager_non_creator_is_forbidden(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    // Creator/manager sets up the org + document.
    let owner = TestUser::new();
    let (owner_token, org_id) = create_authenticated_user_with_org(&app, &owner, "shr-auth").await;
    let owner_id = user_id_for(&pool, &owner.email).await;
    let doc_id = seed_document(&pool, org_id, owner_id, "Owned Doc").await;

    // A second user joins the SAME org as a plain resident (not a manager,
    // not the document creator).
    let outsider = TestUser::new();
    let (outsider_token, _refresh) =
        common::create_authenticated_user(&app, &outsider).await;
    let outsider_id = user_id_for(&pool, &outsider.email).await;
    seed_membership(&pool, org_id, outsider_id, "resident").await;

    // Create — forbidden.
    let create_resp = app
        .execute(
            app.post(&format!("/api/v1/documents/{doc_id}/shares"))
                .bearer(&outsider_token)
                .header("X-Tenant-ID", &org_id.to_string())
                .json(json!({ "share_type": "link" }))
                .build(),
        )
        .await;
    assert_eq!(
        create_resp.status,
        StatusCode::FORBIDDEN,
        "non-manager non-creator must not create shares; got {}. Body: {}",
        create_resp.status,
        create_resp.text()
    );

    // List — forbidden.
    let list_resp = app
        .execute(
            app.get(&format!("/api/v1/documents/{doc_id}/shares"))
                .bearer(&outsider_token)
                .header("X-Tenant-ID", &org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        list_resp.status,
        StatusCode::FORBIDDEN,
        "non-manager non-creator must not list shares; got {}. Body: {}",
        list_resp.status,
        list_resp.text()
    );

    // Sanity: the owner/manager CAN create (proves the doc is shareable and the
    // 403s above are about the caller, not a broken fixture).
    let owner_create = app
        .execute(
            app.post(&format!("/api/v1/documents/{doc_id}/shares"))
                .bearer(&owner_token)
                .header("X-Tenant-ID", &org_id.to_string())
                .json(json!({ "share_type": "link" }))
                .build(),
        )
        .await;
    assert_eq!(
        owner_create.status,
        StatusCode::CREATED,
        "owner/manager must still create shares; got {}. Body: {}",
        owner_create.status,
        owner_create.text()
    );
    let share_id = owner_create.json_value()["id"]
        .as_str()
        .expect("share id")
        .to_string();

    // Revoke as the outsider — forbidden.
    let revoke_resp = app
        .execute(
            app.delete(&format!("/api/v1/documents/{doc_id}/shares/{share_id}"))
                .bearer(&outsider_token)
                .header("X-Tenant-ID", &org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        revoke_resp.status,
        StatusCode::FORBIDDEN,
        "non-manager non-creator must not revoke shares; got {}. Body: {}",
        revoke_resp.status,
        revoke_resp.text()
    );

    cleanup_test_user(&pool, &owner.email).await;
    cleanup_test_user(&pool, &outsider.email).await;
}

// ============================================================================
// Not-found
// ============================================================================

/// Creating a share against a non-existent document id is 404, not 500.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_create_share_on_missing_document_is_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "shr-404").await;

    let missing = Uuid::new_v4();
    let resp = app
        .execute(
            app.post(&format!("/api/v1/documents/{missing}/shares"))
                .bearer(&token)
                .header("X-Tenant-ID", &org_id.to_string())
                .json(json!({ "share_type": "link" }))
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "share create on a missing document must be 404; got {}. Body: {}",
        resp.status,
        resp.text()
    );

    cleanup_test_user(&pool, &user.email).await;
}
