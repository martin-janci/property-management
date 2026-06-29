//! Functional tests for the RAG retrieval path (Story 103.5 / Epic 103).
//!
//! Background
//! ----------
//! `00081_create_pgvector.sql` added the `document_embeddings` store, the
//! `v_rag_statistics` view, and (when pgvector is installed) the native
//! similarity-search SQL functions. The repository layer
//! (`LlmDocumentRepository`) exposes:
//!
//!   * `search_documents_by_embedding` — cosine-similarity search with a
//!     pgvector-native fast path and an application-level JSONB fallback,
//!   * `search_documents_pgvector`     — thin wrapper over the SQL function
//!     with the same fallback,
//!   * `get_rag_statistics`            — reads the `v_rag_statistics` view.
//!
//! Until now nothing exercised the similarity search end-to-end, and the
//! stats view shipped column names (`chunks_with_embedding`) that did not
//! match the columns the repository SELECTs (`chunks_with_vector`,
//! `chunks_pending_migration`) — so `get_rag_statistics` failed at runtime on
//! any DB where the view existed. Migration `00203_fix_rag_statistics_view.sql`
//! realigns the view; the stats assertions below are the regression guard for
//! that fix, and would fail on `dev` before 00203.
//!
//! CI runs against stock PostgreSQL **without** the pgvector extension, so
//! these tests deliberately drive the JSONB-fallback code paths. They connect
//! as the `#[sqlx::test]` superuser (RLS is exercised separately in
//! `llm_document_rls_repo_tests.rs`); the focus here is retrieval correctness
//! and the stats-view contract.
//!
//! # Performance note — single test function
//!
//! `#[sqlx::test(migrator = "db::MIGRATOR")]` applies all ~194 migrations to a
//! fresh database for EACH annotated test function. With three separate
//! functions that overhead was 3× the full migration set, which caused the CI
//! job to time out at 60 minutes. All three logical concerns (similarity
//! ranking/filtering, org-scoping IDOR, stats-view contract) are now expressed
//! as sequential sub-assertions in a single test so migrations run only once.

use db::repositories::LlmDocumentRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn set_super_ctx(pool: &PgPool) {
    sqlx::query("SELECT set_request_context($1, $2, $3)")
        .bind(Option::<Uuid>::None)
        .bind(Option::<Uuid>::None)
        .bind(true)
        .execute(pool)
        .await
        .expect("set super-admin context");
}

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active')
        RETURNING id
        "#,
    )
    .bind(format!("Rag {slug}"))
    .bind(slug)
    .bind(format!("{slug}@rag.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', 'Rag User', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_document(pool: &PgPool, org_id: Uuid, created_by: Uuid, title: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO documents (
            organization_id, title, category, file_key, file_name, mime_type,
            size_bytes, created_by
        )
        VALUES ($1, $2, 'other', $3, $4, 'application/pdf', 1024, $5)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(title)
    .bind(format!("s3/{title}.pdf"))
    .bind(format!("{title}.pdf"))
    .bind(created_by)
    .fetch_one(pool)
    .await
    .expect("seed document")
}

/// Combined RAG retrieval test: similarity ranking/filtering, org-scoping IDOR
/// guard, and v_rag_statistics view contract — all in one DB so migrations
/// apply only once instead of three times.
///
/// Sub-sections:
///   A. Similarity ranking & min_similarity floor (JSONB-fallback path)
///   B. Org-scoping: org A query must never surface org B chunks
///   C. Stats view: regression guard for migration 00203
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn rag_retrieval_correctness(pool: PgPool) {
    set_super_ctx(&pool).await;
    let repo = LlmDocumentRepository::new(pool.clone());

    // -------------------------------------------------------------------------
    // A. Similarity ranking & min_similarity filter
    // -------------------------------------------------------------------------

    let org_a = seed_org(&pool, "rag-sim-a").await;
    let user_a = seed_user(&pool, "sim-a@rag.test").await;
    let doc_a = seed_document(&pool, org_a, user_a, "rag-sim-doc").await;

    // Three orthogonal-ish 3-dim embeddings. The query points straight along
    // the first axis, so the "near" chunk should win, "mid" second, and the
    // orthogonal chunk should be filtered out by min_similarity.
    let mut conn = pool.acquire().await.expect("acquire");
    repo.create_embedding(
        &mut *conn,
        org_a,
        doc_a,
        0,
        "near chunk",
        Some(vec![1.0, 0.0, 0.0]),
        serde_json::json!({"tag": "near"}),
    )
    .await
    .expect("create near chunk");
    repo.create_embedding(
        &mut *conn,
        org_a,
        doc_a,
        1,
        "mid chunk",
        Some(vec![0.8, 0.6, 0.0]),
        serde_json::json!({"tag": "mid"}),
    )
    .await
    .expect("create mid chunk");
    repo.create_embedding(
        &mut *conn,
        org_a,
        doc_a,
        2,
        "orthogonal chunk",
        Some(vec![0.0, 1.0, 0.0]),
        serde_json::json!({"tag": "orthogonal"}),
    )
    .await
    .expect("create orthogonal chunk");

    let query = vec![1.0_f32, 0.0, 0.0];

    // With a 0.5 floor the orthogonal chunk (cosine 0.0) is excluded.
    let results = repo
        .search_documents_by_embedding(&mut conn, org_a, &query, 10, Some(0.5))
        .await
        .expect("similarity search");

    assert_eq!(
        results.len(),
        2,
        "min_similarity=0.5 must exclude the orthogonal chunk (cosine 0.0)"
    );
    assert_eq!(
        results[0].0.chunk_text, "near chunk",
        "the chunk co-linear with the query must rank first"
    );
    assert_eq!(
        results[1].0.chunk_text, "mid chunk",
        "the partially-aligned chunk must rank second"
    );
    assert!(
        results[0].1 >= results[1].1,
        "scores must be sorted descending: {} >= {}",
        results[0].1,
        results[1].1
    );
    assert!(
        (results[0].1 - 1.0).abs() < 1e-6,
        "co-linear chunk must have cosine ~1.0, got {}",
        results[0].1
    );

    // `limit` caps the result set.
    let limited = repo
        .search_documents_by_embedding(&mut conn, org_a, &query, 1, Some(0.0))
        .await
        .expect("similarity search (limited)");
    assert_eq!(limited.len(), 1, "limit=1 must return a single chunk");
    assert_eq!(limited[0].0.chunk_text, "near chunk");

    // The pgvector wrapper falls back to the same path when the SQL function
    // is absent (CI), so it must return the same top hit.
    let via_wrapper = repo
        .search_documents_pgvector(&mut conn, org_a, &query, 10, Some(0.5))
        .await
        .expect("pgvector wrapper search");
    assert_eq!(
        via_wrapper.first().map(|r| r.0.chunk_text.as_str()),
        Some("near chunk"),
        "search_documents_pgvector fallback must match the application search"
    );

    // -------------------------------------------------------------------------
    // B. Org-scoping IDOR: a query in org A must never surface org B's chunks
    // -------------------------------------------------------------------------

    let org_b = seed_org(&pool, "rag-scope-b").await;
    let user_b = seed_user(&pool, "scope-b@rag.test").await;
    let doc_b = seed_document(&pool, org_b, user_b, "rag-scope-doc-b").await;

    let shared_vec = vec![1.0_f32, 0.0, 0.0];
    // org_a already has "near chunk" with the same vector; add org_b's version.
    repo.create_embedding(
        &mut *conn,
        org_b,
        doc_b,
        0,
        "org b secret",
        Some(shared_vec.clone()),
        serde_json::json!({}),
    )
    .await
    .expect("seed org b chunk");

    let scoped_results = repo
        .search_documents_by_embedding(&mut conn, org_a, &shared_vec, 10, Some(0.5))
        .await
        .expect("org-a similarity search");

    // org_a has "near chunk" (and "mid chunk" at 0.8 cos) — both above 0.5.
    // "org b secret" (identical vector) must NOT appear.
    assert!(
        scoped_results
            .iter()
            .all(|(emb, _)| emb.organization_id == org_a),
        "org A search must only see org A's chunks; found foreign org_id"
    );
    assert!(
        scoped_results
            .iter()
            .all(|(emb, _)| emb.chunk_text != "org b secret"),
        "org B's chunk must not appear in org A's search results"
    );

    // -------------------------------------------------------------------------
    // C. Stats view: regression guard for migration 00203
    //    (pre-00203 view exposed `chunks_with_embedding`; repo SELECTs
    //     `chunks_with_vector` / `chunks_pending_migration` — name mismatch
    //     caused a runtime error)
    // -------------------------------------------------------------------------

    // Use a fresh org so counts are predictable regardless of section-A/B data.
    let org_stats = seed_org(&pool, "rag-stats-a").await;
    let user_stats = seed_user(&pool, "stats-a@rag.test").await;
    let doc_stats = seed_document(&pool, org_stats, user_stats, "rag-stats-doc").await;

    // Two chunks WITH a JSONB embedding, one chunk WITHOUT (pending).
    repo.create_embedding(
        &mut *conn,
        org_stats,
        doc_stats,
        0,
        "chunk one",
        Some(vec![1.0, 0.0]),
        serde_json::json!({}),
    )
    .await
    .expect("chunk 0");
    repo.create_embedding(
        &mut *conn,
        org_stats,
        doc_stats,
        1,
        "chunk two",
        Some(vec![0.0, 1.0]),
        serde_json::json!({}),
    )
    .await
    .expect("chunk 1");
    repo.create_embedding(
        &mut *conn,
        org_stats,
        doc_stats,
        2,
        "chunk three (no embedding)",
        None,
        serde_json::json!({}),
    )
    .await
    .expect("chunk 2");

    // The load-bearing assertion: this call SELECTs the view columns the model
    // declares. On the pre-00203 view it errors with
    // `column "chunks_with_vector" does not exist`.
    let stats = repo
        .get_rag_statistics(&mut conn, org_stats)
        .await
        .expect("get_rag_statistics must succeed against the aligned view");

    assert_eq!(stats.indexed_documents, 1, "one source document indexed");
    assert_eq!(stats.total_chunks, 3, "three chunks total");
    assert_eq!(
        stats.chunks_with_vector, 2,
        "two chunks carry an embedding (JSONB fallback counts as 'with vector')"
    );
    assert!(
        stats.avg_chunk_length > 0,
        "avg chunk length should be computed, got {}",
        stats.avg_chunk_length
    );
    assert!(
        stats.last_updated.is_some(),
        "last_updated should be populated"
    );
}
