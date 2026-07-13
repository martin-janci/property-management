//! Integration tests for the pgvector back-fill route (Story 84.5 / 103.5).
//!
//! Gap closed: "`migrate_embeddings_to_pgvector` not wired to any HTTP route —
//! no way to reindex legacy no-provenance rows (they still mix embedding spaces
//! in filtered search)". Before this, `LlmDocumentRepository::
//! migrate_embeddings_to_pgvector` could only be reached from tests. `POST
//! /api/v1/ai/llm/rag/migrate` is the operator-triggerable route that runs it.
//!
//! ## Access model
//! The back-fill converts legacy JSONB embeddings across every organization in
//! one pass, so it needs the super-admin RLS bypass and is gated to
//! platform/super admins (403 for anyone else, 401 unauthenticated).
//!
//! ## pgvector independence
//! `migrate_jsonb_to_vector()` is created only when the `vector` extension is
//! present. When it is absent the repository call is a deterministic no-op that
//! returns 0, so these tests assert the route contract (status + JSON shape),
//! not a specific migrated count — they pass whether or not CI's Postgres has
//! pgvector installed.
//!
//! DB-backed via `#[sqlx::test]` (migrator = db::MIGRATOR) — bodies run in CI
//! where Postgres is available; the local dispatcher runner only compile-gates.

mod common;

use axum::http::StatusCode;
use sqlx::PgPool;
use uuid::Uuid;

use common::{
    create_authenticated_user, create_authenticated_user_with_org, seed_membership, seed_org,
    TestApp, TestUser,
};

/// Resolve the id of a previously registered test user by email.
async fn resolve_user_id(app: &TestApp, user: &TestUser) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("resolve user id")
}

// POST /api/v1/ai/llm/rag/migrate as a platform admin → 200 with a numeric
// `migrated` count. The route exists (this whole endpoint is what the gap adds)
// and the migration is a no-op-or-more depending on pgvector availability.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn rag_migrate_as_platform_admin_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();

    // Build a platform-admin principal directly: register + log in, then seed an
    // active `platform_admin` membership so the DB-validated role (used by
    // RlsConnection, not the JWT) grants the super-admin RLS bypass.
    let (token, _refresh) = create_authenticated_user(&app, &user).await;
    let user_id = resolve_user_id(&app, &user).await;
    let org_id = seed_org(&app.pool, "rag-migrate-admin").await;
    seed_membership(&app.pool, org_id, user_id, "platform_admin").await;
    let session = app.session(token, org_id);

    let resp = app
        .execute(session.post("/api/v1/ai/llm/rag/migrate").build())
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "platform admin migrate must return 200; body={}",
        resp.text()
    );
    let body = resp.json_value();
    assert!(
        body["migrated"].is_number(),
        "response must carry a numeric `migrated` count; body={body}"
    );
    assert!(
        body["migrated"].as_i64().is_some_and(|n| n >= 0),
        "migrated count must be non-negative; body={body}"
    );
}

// A non-admin (org_admin) caller must be refused — the back-fill is a
// cross-tenant maintenance op reserved for platform administrators.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn rag_migrate_non_admin_returns_403(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    // `create_authenticated_user_with_org` seeds the user as `org_admin`.
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "rag-migrate-403").await;
    let session = app.session(token, org_id);

    let resp = app
        .execute(session.post("/api/v1/ai/llm/rag/migrate").build())
        .await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "non-admin migrate must be 403; body={}",
        resp.text()
    );
}

// Unauthenticated request → 401 (RlsConnection extractor guards the route).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn rag_migrate_requires_auth(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let resp = app
        .execute(app.post("/api/v1/ai/llm/rag/migrate").build())
        .await;

    assert_eq!(
        resp.status,
        StatusCode::UNAUTHORIZED,
        "unauthenticated migrate must be 401; body={}",
        resp.text()
    );
}
