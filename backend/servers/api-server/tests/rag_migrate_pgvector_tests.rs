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
//! returns 0, so the route-contract tests assert status + JSON shape, not a
//! specific migrated count — they pass whether or not CI's Postgres has pgvector
//! installed. The data-integrity test
//! (`rag_migrate_stamps_provenance_on_legacy_row`, #2300) seeds a legacy
//! no-provenance JSONB row and, *only when pgvector is present*, asserts the
//! back-fill converts it AND stamps assumed `embedding_model` provenance so a
//! provenance-filtered search can isolate its embedding space.
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

/// True when the `vector` (pgvector) extension is installed on the test DB.
/// The back-fill function `migrate_jsonb_to_vector()` and the `embedding_vector`
/// column only exist under pgvector, so the conversion assertions below are
/// gated on this — CI's Postgres may not carry the extension.
async fn pgvector_present(app: &TestApp) -> bool {
    sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM pg_extension WHERE extname = 'vector')")
        .fetch_one(&app.pool)
        .await
        .unwrap_or(false)
}

/// Seed a `documents` row so the `document_embeddings.document_id` FK
/// (migration 00081) is satisfied.
async fn seed_document(pool: &PgPool, org_id: Uuid, created_by: Uuid, id: Uuid) {
    sqlx::query(
        r#"INSERT INTO documents
               (id, organization_id, title, category, file_key, file_name,
                mime_type, size_bytes, created_by)
           VALUES ($1, $2, 'RAG Legacy Source', 'other', $3, 'legacy.txt', 'text/plain', 1024, $4)"#,
    )
    .bind(id)
    .bind(org_id)
    .bind(format!("{org_id}/{id}.txt"))
    .bind(created_by)
    .execute(pool)
    .await
    .expect("seed document");
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

// Data-integrity test (#2300): a legacy row with a 1536-dim JSONB embedding,
// NULL vector, and no `embedding_model` provenance must, after the back-fill,
// have its vector populated AND be stamped with the assumed provenance
// (`text-embedding-3-small`) so a provenance-filtered search can isolate it.
//
// The conversion + provenance assertions are gated on pgvector being installed
// (the `embedding_vector` column and `migrate_jsonb_to_vector()` only exist
// then); when absent the route is still exercised and must return 200 with a
// numeric `migrated` count, matching this batch's pgvector-independence rule.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn rag_migrate_stamps_provenance_on_legacy_row(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();

    // Platform-admin principal (same construction as the 200 test above).
    let (token, _refresh) = create_authenticated_user(&app, &user).await;
    let user_id = resolve_user_id(&app, &user).await;
    let org_id = seed_org(&app.pool, "rag-migrate-prov").await;
    seed_membership(&app.pool, org_id, user_id, "platform_admin").await;

    // Seed a legacy embedding row: 1536-dim JSONB array, NULL vector (omitted so
    // this INSERT also works when the pgvector column is absent), and metadata
    // WITHOUT an `embedding_model` key — i.e. a pre-provenance legacy row.
    let doc_id = Uuid::new_v4();
    seed_document(&app.pool, org_id, user_id, doc_id).await;
    let emb_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO document_embeddings
               (organization_id, document_id, chunk_index, chunk_text, embedding, metadata)
           VALUES ($1, $2, 0, 'legacy chunk',
                   to_jsonb(array_fill(0.1::float8, ARRAY[1536])),
                   '{}'::jsonb)
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(doc_id)
    .fetch_one(&app.pool)
    .await
    .expect("seed legacy embedding row");

    let session = app.session(token, org_id);
    let resp = app
        .execute(session.post("/api/v1/ai/llm/rag/migrate").build())
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "migrate must return 200; body={}",
        resp.text()
    );

    if !pgvector_present(&app).await {
        // No pgvector on this DB: the back-fill function is absent and the run is
        // a deterministic no-op. Nothing to convert — assert the contract only.
        assert_eq!(
            resp.json_value()["migrated"].as_i64(),
            Some(0),
            "without pgvector the back-fill must report 0 migrated"
        );
        return;
    }

    // pgvector present: the legacy row must now be converted AND provenance-stamped.
    assert!(
        resp.json_value()["migrated"]
            .as_i64()
            .is_some_and(|n| n >= 1),
        "back-fill must report at least the one seeded legacy row as migrated; body={}",
        resp.json_value()
    );

    let (vector_set, stamped_model): (bool, Option<String>) = sqlx::query_as(
        r#"SELECT embedding_vector IS NOT NULL,
                  metadata->>'embedding_model'
           FROM document_embeddings
           WHERE id = $1"#,
    )
    .bind(emb_id)
    .fetch_one(&app.pool)
    .await
    .expect("re-read migrated embedding row");

    assert!(
        vector_set,
        "back-fill must populate embedding_vector on the legacy row"
    );
    assert_eq!(
        stamped_model.as_deref(),
        Some("text-embedding-3-small"),
        "back-fill must stamp assumed embedding_model provenance so filtered \
         search can isolate the row's embedding space (was mixing before #2300)"
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
