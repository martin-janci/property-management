//! Integration tests for the RAG embedding-write flow (Story 84.5 / 103.5).
//!
//! Gap closed: "No application handler calling search_similar_documents or
//! embedding-write flow (84-5-pgvector-rag)". Before this, the pgvector RAG
//! store could only be populated from tests — there was no application-side
//! endpoint that generated embeddings and wrote them. `POST
//! /api/v1/ai/llm/rag/index` is that endpoint.
//!
//! ## Provider selection & CI
//! The handler embeds via the Story 84.5 `EmbeddingProvider` abstraction: the
//! live OpenAI backend when `OPENAI_API_KEY` is set, otherwise the
//! deterministic offline `StubEmbeddingProvider`. CI runs without a key, so the
//! stub path is what these tests exercise. Tests that would trigger a live
//! embedding call skip themselves when a key is present (same convention as the
//! other `ai/llm.rs` batches, which skip live-provider endpoints).
//!
//! DB-backed via `#[sqlx::test]` (migrator = db::MIGRATOR) — bodies run in CI
//! where Postgres is available; the local dispatcher runner only compile-gates.

mod common;

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::{create_authenticated_user_with_org, TestApp, TestUser};

/// True when a live OpenAI key is configured — in that case the handler makes a
/// real network embedding call, so tests that hit the embed path skip.
fn live_openai_key_present() -> bool {
    std::env::var("OPENAI_API_KEY").is_ok()
}

/// Resolve the id of a previously registered test user by email.
async fn resolve_user_id(app: &TestApp, user: &TestUser) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("resolve user id")
}

/// Seed a `documents` row with an explicit id so the embedding-write FK
/// (`document_embeddings.document_id → documents(id)`, migration 00081) is
/// satisfied. The index handler stores embeddings keyed by the caller-supplied
/// `document_id`, so the referenced document must already exist — otherwise both
/// the pgvector and JSONB-fallback INSERT paths fail the foreign key with a 500.
async fn seed_document(pool: &PgPool, org_id: Uuid, created_by: Uuid, id: Uuid) {
    sqlx::query(
        r#"INSERT INTO documents
               (id, organization_id, title, category, file_key, file_name,
                mime_type, size_bytes, created_by)
           VALUES ($1, $2, 'RAG Source', 'other', $3, 'rag.txt', 'text/plain', 1024, $4)"#,
    )
    .bind(id)
    .bind(org_id)
    .bind(format!("{org_id}/{id}.txt"))
    .bind(created_by)
    .execute(pool)
    .await
    .expect("seed document");
}

// POST /api/v1/ai/llm/rag/index → 201, persists one embedding row per chunk.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn rag_index_persists_chunks(pool: PgPool) {
    if live_openai_key_present() {
        return; // avoid a live embedding network call
    }
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "rag-index-1").await;
    let session = app.session(token, org_id);

    // The RAG store is FK-bound to `documents`; seed the referenced row first.
    let user_id = resolve_user_id(&app, &user).await;
    let document_id = Uuid::new_v4();
    seed_document(&app.pool, org_id, user_id, document_id).await;

    let resp = app
        .execute(
            session
                .post("/api/v1/ai/llm/rag/index")
                .json(json!({
                    "document_id": document_id,
                    "title": "House Rules",
                    "chunks": [
                        "Quiet hours are between 22:00 and 06:00.",
                        "Pets are allowed with prior written approval.",
                    ],
                }))
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "index must return 201; body={}",
        resp.text()
    );
    let body = resp.json_value();
    assert_eq!(body["chunks_indexed"], json!(2));
    assert_eq!(
        body["provider"], "stub-deterministic",
        "no key configured → deterministic stub provider; body={body}"
    );
    let ids = body["embedding_ids"]
        .as_array()
        .expect("embedding_ids array");
    assert_eq!(ids.len(), 2, "one embedding id per chunk");
    assert!(
        ids.iter()
            .all(|v| v.as_str().is_some_and(|s| !s.is_empty())),
        "embedding ids must be non-empty UUIDs"
    );
}

// Re-indexing the same (document_id, chunk_index) upserts rather than
// duplicating — the second call returns the same row ids.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn rag_index_is_idempotent(pool: PgPool) {
    if live_openai_key_present() {
        return;
    }
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "rag-index-2").await;
    let session = app.session(token, org_id);

    // The RAG store is FK-bound to `documents`; seed the referenced row first.
    let user_id = resolve_user_id(&app, &user).await;
    let document_id = Uuid::new_v4();
    seed_document(&app.pool, org_id, user_id, document_id).await;
    let payload = json!({
        "document_id": document_id,
        "chunks": ["The reserve fund covers roof repairs."],
    });

    let first = app
        .execute(
            session
                .post("/api/v1/ai/llm/rag/index")
                .json(&payload)
                .build(),
        )
        .await;
    assert_eq!(first.status, StatusCode::CREATED, "body={}", first.text());
    let second = app
        .execute(
            session
                .post("/api/v1/ai/llm/rag/index")
                .json(&payload)
                .build(),
        )
        .await;
    assert_eq!(second.status, StatusCode::CREATED, "body={}", second.text());

    assert_eq!(
        first.json_value()["embedding_ids"],
        second.json_value()["embedding_ids"],
        "upsert on (document_id, chunk_index) must reuse the same row id"
    );
}

// Empty / whitespace-only chunk list → 400 before any embedding work.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn rag_index_rejects_empty_chunks(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "rag-index-3").await;
    let session = app.session(token, org_id);

    let resp = app
        .execute(
            session
                .post("/api/v1/ai/llm/rag/index")
                .json(json!({
                    "document_id": Uuid::new_v4(),
                    "chunks": ["", "   "],
                }))
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::BAD_REQUEST,
        "empty chunks must be rejected; body={}",
        resp.text()
    );
}

// Non-existent document_id → 404 BEFORE any embedding work (#2201 fk-error-shape).
// Embeddings are FK-bound to `documents`; the handler pre-checks existence and
// returns 404 rather than paying for an embed and surfacing a 500 FK violation.
// No live-key skip needed: the 404 is returned before the embed phase.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn rag_index_missing_document_returns_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "rag-index-404").await;
    let session = app.session(token, org_id);

    // Deliberately do NOT seed a `documents` row — the id does not exist.
    let resp = app
        .execute(
            session
                .post("/api/v1/ai/llm/rag/index")
                .json(json!({
                    "document_id": Uuid::new_v4(),
                    "chunks": ["orphan chunk that must not be embedded"],
                }))
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "indexing against a non-existent document must be 404; body={}",
        resp.text()
    );
}

// Unauthenticated request → 401 (RlsConnection extractor guards the route).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn rag_index_requires_auth(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let resp = app
        .execute(
            app.post("/api/v1/ai/llm/rag/index")
                .json(json!({
                    "document_id": Uuid::new_v4(),
                    "chunks": ["anything"],
                }))
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::UNAUTHORIZED,
        "unauthenticated index must be 401; body={}",
        resp.text()
    );
}
