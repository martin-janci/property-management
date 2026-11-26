//! Error-sanitization regression tests for the import handlers in
//! `reality-server/src/routes/imports.rs`
//! (code-review finding: "reality server imports raw error leak").
//!
//! # Background
//!
//! Reality-server is internet-facing. Several import handlers mapped an
//! unexpected DB error with
//!
//! ```ignore
//! other => (StatusCode::INTERNAL_SERVER_ERROR,
//!           format!("Failed to update import job: {}", other))
//! ```
//!
//! which serialized the raw `sqlx::Error` straight into the 500 response body.
//! A driver error carries Postgres internals — column/constraint names, type
//! definitions (`character varying(255)`), sometimes pool/TLS state — none of
//! which a portal client should ever see. The convention for every other DB
//! `map_err` boundary in this crate is `crate::util::errors::db_error[_or]`,
//! which logs the full error server-side and returns a fixed generic body.
//!
//! This suite drives the real HTTP surface and provokes a *non-`RowNotFound`*
//! DB error deterministically: `portal_import_jobs.source_filename` is
//! `VARCHAR(255)` and the `update_import_job` handler applies no length check,
//! so a 300-char filename on an existing job trips a `22001`
//! ("value too long …") driver error inside the handler's `other` arm. The
//! test asserts the client never sees that raw text — only the sanitized
//! `(500, "Internal server error")`.
//!
//! Pre-fix this test fails (body is the raw `Failed to update import job: …`
//! leak); post-fix it passes.

#![allow(dead_code)]

use crate::common::{make_app_state, mint_token};
use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
    Router,
};
use db::models::CreatePortalImportJob;
use db::repositories::RealityPortalRepository;
use reality_server::routes;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

fn imports_router(pool: PgPool) -> Router {
    let state = make_app_state(pool);
    Router::new()
        .nest("/api/v1/imports", routes::imports::router())
        .with_state(state)
}

/// Seed a `users` row (`PortalPrincipal` re-derives the kind from this table)
/// and return its id.
async fn seed_portal_user(pool: &PgPool, tag: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', $2, 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(format!("{tag}@imports-error-leak.test"))
    .bind(format!("ImportsErrorLeak {tag}"))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_portal_user({tag}): {e}"))
}

async fn seed_import_job(pool: &PgPool, user_id: Uuid) -> Uuid {
    let repo = RealityPortalRepository::new(pool.clone());
    let job = repo
        .create_import_job(
            user_id,
            CreatePortalImportJob {
                agency_id: None,
                source_type: "url".to_string(),
                source_url: Some("https://example.com/feed.xml".to_string()),
                source_filename: None,
            },
        )
        .await
        .expect("seed_import_job");
    job.id
}

/// Send a JSON request and return the status plus the raw response body as a
/// string (the error body is `text/plain`, not JSON, so it must not be parsed).
async fn send_json_raw(
    router: &Router,
    method: Method,
    path: &str,
    token: &str,
    body: serde_json::Value,
) -> (StatusCode, String) {
    let req = Request::builder()
        .method(method)
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let response = router.clone().oneshot(req).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

/// A DB driver error inside `update_import_job` must be sanitized to the
/// generic `(500, "Internal server error")` — never serialized verbatim into
/// the client-facing body. `source_filename` is `VARCHAR(255)`, so a 300-char
/// value on an existing job trips a Postgres `22001` driver error (not
/// `RowNotFound`), exercising the handler's fall-through error arm.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_import_job_db_error_is_sanitized(pool: PgPool) {
    let user = seed_portal_user(&pool, "update-leak").await;
    let job_id = seed_import_job(&pool, user).await;
    let token = mint_token(user);

    let app = imports_router(pool);
    // 300 chars > VARCHAR(255): the UPDATE matches the row but violates the
    // column type, so the repo returns a driver error, not RowNotFound.
    let overlong = "x".repeat(300);
    let (status, body) = send_json_raw(
        &app,
        Method::PUT,
        &format!("/api/v1/imports/jobs/{job_id}"),
        &token,
        serde_json::json!({ "source_filename": overlong }),
    )
    .await;

    assert_eq!(
        status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "a DB driver error should surface as 500, got {status}"
    );
    assert_eq!(
        body, "Internal server error",
        "client body must be the fixed generic message, got `{body}`"
    );
    // None of these DB-internal fragments may reach the internet-facing client.
    for needle in [
        "Failed to update import job",
        "value too long",
        "character varying",
        "error returned from database",
        "portal_import_jobs",
        "source_filename",
    ] {
        assert!(
            !body.contains(needle),
            "import-job error body leaked DB internals: found `{needle}` in `{body}`"
        );
    }
}
