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
//! The tests run both with and without the pgvector extension (CI has it).
//! `search_documents_by_embedding` is always the application-level JSONB
//! search; `search_documents_pgvector` uses the SQL function when pgvector is
//! installed, so the provenance test seeds 1536-dim vectors that fit
//! `embedding_vector vector(1536)`. They connect
//! as the `#[sqlx::test]` superuser (RLS is exercised separately in
//! `llm_document_rls_repo_tests.rs`); the focus here is retrieval correctness
//! and the stats-view contract.
//!
//! # Performance note — migration cost per test function
//!
//! `#[sqlx::test(migrator = "db::MIGRATOR")]` applies all ~194 migrations to a
//! fresh database for EACH annotated test function. The three original concerns
//! (similarity ranking/filtering, org-scoping IDOR, stats-view contract) each had
//! their own function, so the overhead was 3× the full migration set, which caused
//! the CI job to time out at 60 minutes. They are now expressed as sequential
//! sub-assertions in `rag_retrieval_correctness` so migrations run once for that
//! group. The provenance retrieval-filter contract (#2239) lives in its own
//! function `search_documents_pgvector_model_filter_provenance` so the regression
//! guard is an independently-named case with a small blast radius (#2272) rather
//! than section E buried inside the combined test. Prefer consolidating new assertions into an existing function over
//! adding a fresh migrator run unless a guard genuinely warrants its own case.

use crate::common::seed_org;
use db::repositories::LlmDocumentRepository;
use sqlx::PgPool;
use uuid::Uuid;

/// Whether migration 00081 installed the pgvector search function (only when
/// the pgvector extension is available on the server).
async fn pgvector_search_available(pool: &PgPool) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM pg_proc WHERE proname = 'search_similar_documents')",
    )
    .fetch_one(pool)
    .await
    .expect("probe pgvector search function")
}

/// A 1536-dim unit vector along axis `i` — the dimension of
/// `document_embeddings.embedding_vector` when pgvector is installed.
fn axis_1536(i: usize) -> Vec<f32> {
    let mut v = vec![0.0_f32; 1536];
    v[i] = 1.0;
    v
}

async fn set_super_ctx(pool: &PgPool) {
    sqlx::query("SELECT set_request_context($1, $2, $3)")
        .bind(Option::<Uuid>::None)
        .bind(Option::<Uuid>::None)
        .bind(true)
        .execute(pool)
        .await
        .expect("set super-admin context");
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
/// The provenance retrieval-filter contract (formerly section E, #2239) now lives
/// in its own focused function `search_documents_pgvector_model_filter_provenance`
/// so the regression guard is an independently-named case (#2272).
///
/// Sub-sections:
///   A. Similarity ranking & min_similarity floor (JSONB-fallback path)
///   B. Org-scoping: org A query must never surface org B chunks
///   C. Stats view: regression guard for migration 00203
///   D. Upsert identity + provenance persisted into metadata (Story 84.5 / #2201)
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

    // Without pgvector the wrapper falls back to the same JSONB path, so it must
    // return the same top hit. With pgvector it searches `embedding_vector`
    // instead (covered by `search_documents_pgvector_model_filter_provenance`).
    if !pgvector_search_available(&pool).await {
        let via_wrapper = repo
            .search_documents_pgvector(&mut conn, org_a, &query, 10, Some(0.5), None)
            .await
            .expect("pgvector wrapper search");
        assert_eq!(
            via_wrapper.first().map(|r| r.0.chunk_text.as_str()),
            Some("near chunk"),
            "search_documents_pgvector fallback must match the application search"
        );
    }

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
    let pgvector = pgvector_search_available(&pool).await;
    if pgvector {
        // With pgvector the view counts `embedding_vector`; JSONB-only rows from
        // `create_embedding` are reported as pending migration instead.
        assert_eq!(
            stats.chunks_with_vector, 0,
            "no chunk has embedding_vector yet"
        );
        assert_eq!(
            stats.chunks_pending_migration, 2,
            "two JSONB embeddings await migration to embedding_vector"
        );
    } else {
        assert_eq!(
            stats.chunks_with_vector, 2,
            "two chunks carry an embedding (JSONB fallback counts as 'with vector')"
        );
    }
    assert!(
        stats.avg_chunk_length > 0,
        "avg chunk length should be computed, got {}",
        stats.avg_chunk_length
    );
    assert!(
        stats.last_updated.is_some(),
        "last_updated should be populated"
    );

    // -------------------------------------------------------------------------
    // D. Upsert wrapper (Story 84.5): insert-then-update identity on
    //    (org, document, chunk_index). With pgvector this goes through the
    //    00081 SQL function into `embedding_vector`; without it, the JSONB
    //    fallback. 1536-dim vectors fit both.
    // -------------------------------------------------------------------------

    let org_up = seed_org(&pool, "rag-upsert-a").await;
    let user_up = seed_user(&pool, "upsert-a@rag.test").await;
    let doc_up = seed_document(&pool, org_up, user_up, "rag-upsert-doc").await;

    let first_id = repo
        .upsert_embedding(
            &mut conn,
            org_up,
            doc_up,
            0,
            "original chunk text",
            &axis_1536(0),
            "test-model-v1",
            serde_json::json!({"rev": 1}),
        )
        .await
        .expect("upsert (insert path)");

    let second_id = repo
        .upsert_embedding(
            &mut conn,
            org_up,
            doc_up,
            0,
            "revised chunk text",
            &axis_1536(1),
            "test-model-v1",
            serde_json::json!({"rev": 2}),
        )
        .await
        .expect("upsert (update path)");

    assert_eq!(
        first_id, second_id,
        "same (org, document, chunk_index) must update in place, not insert"
    );

    let chunks = repo
        .find_document_embeddings(&mut *conn, doc_up)
        .await
        .expect("find upserted chunks");
    assert_eq!(chunks.len(), 1, "upsert must not duplicate the chunk");
    assert_eq!(chunks[0].chunk_text, "revised chunk text");
    if !pgvector {
        // The JSONB fallback stores the vector in `embedding`; under pgvector it
        // lives in `embedding_vector`, which the model does not project.
        assert_eq!(chunks[0].embedding.as_deref(), Some(&axis_1536(1)[..]));
    }
    assert_eq!(chunks[0].metadata["rev"], 2);
    // Provenance (#2201): the model that produced the vector is folded into
    // metadata on upsert so retrieval can filter to a single embedding space.
    assert_eq!(
        chunks[0].metadata["embedding_model"], "test-model-v1",
        "upsert must persist embedding provenance into metadata"
    );

    // A different chunk_index for the same document inserts a new row.
    let third_id = repo
        .upsert_embedding(
            &mut conn,
            org_up,
            doc_up,
            1,
            "second chunk",
            &axis_1536(2),
            "test-model-v1",
            serde_json::json!({}),
        )
        .await
        .expect("upsert (new chunk_index)");
    assert_ne!(third_id, first_id, "new chunk_index must create a new row");
}

/// Provenance retrieval filter (#2239 / #2272): `search_documents_pgvector`'s
/// `model_filter` is the retrieval half of the #2201 provenance finding — it must
/// avoid comparing vectors produced by different embedding models (stub vs OpenAI,
/// both 1536-dim, live in incompatible spaces). Nothing else exercises the
/// filtering path: `rag_retrieval_correctness`'s section A calls the wrapper with
/// `None`, and the sessions.rs call site needs a live provider key.
///
/// Extracted from `rag_retrieval_correctness`'s former section E into its own
/// focused case so the provenance contract runs as an independent, clearly-named
/// test with a small blast radius once the BIT-351 DB lane is repaired (BIT-352).
/// It seeds one row per model plus a legacy untagged row and asserts:
///   * a row tagged with a DIFFERENT model is dropped,
///   * the matching-model row is kept,
///   * a legacy row with NO `embedding_model` metadata is kept (documented
///     backward-compat behaviour of `filter_by_embedding_model`),
///   * `limit` is honoured after over-fetch (×4, cap 200) + post-filter.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn search_documents_pgvector_model_filter_provenance(pool: PgPool) {
    set_super_ctx(&pool).await;
    let repo = LlmDocumentRepository::new(pool.clone());
    let mut conn = pool.acquire().await.expect("acquire");

    let org_prov = seed_org(&pool, "rag-prov-a").await;
    let user_prov = seed_user(&pool, "prov-a@rag.test").await;
    let doc_prov = seed_document(&pool, org_prov, user_prov, "rag-prov-doc").await;

    // All three chunks share the query vector, so cosine similarity is ~1.0 for
    // each and they all clear the min_similarity floor — the ONLY thing that can
    // remove a row from the result set is the model provenance filter.
    // 1536 dims so the rows also land in `embedding_vector` under pgvector.
    let prov_vec = axis_1536(0);

    // chunk 0: matching model — upsert folds `embedding_model` into metadata.
    repo.upsert_embedding(
        &mut conn,
        org_prov,
        doc_prov,
        0,
        "prov match",
        &prov_vec,
        "text-embedding-3-small",
        serde_json::json!({}),
    )
    .await
    .expect("seed matching-model chunk");

    // chunk 1: mismatched model — must be dropped when filtering to the query
    // model, even though its vector is identical to the query.
    repo.upsert_embedding(
        &mut conn,
        org_prov,
        doc_prov,
        1,
        "prov mismatch stub",
        &prov_vec,
        "stub-deterministic-1536",
        serde_json::json!({}),
    )
    .await
    .expect("seed mismatched-model chunk");

    // chunk 2: legacy row indexed before provenance existed — metadata carries
    // NO `embedding_model` key. `upsert_embedding` always stamps the model (and
    // is the only writer that fills `embedding_vector` under pgvector), so seed
    // through it and then strip the key, as a pre-#2201 row would look.
    repo.upsert_embedding(
        &mut conn,
        org_prov,
        doc_prov,
        2,
        "prov legacy untagged",
        &prov_vec,
        "legacy",
        serde_json::json!({"note": "no embedding_model"}),
    )
    .await
    .expect("seed legacy untagged chunk");
    sqlx::query(
        "UPDATE document_embeddings SET metadata = metadata - 'embedding_model' \
         WHERE document_id = $1 AND chunk_index = 2",
    )
    .bind(doc_prov)
    .execute(&mut *conn)
    .await
    .expect("strip embedding_model from legacy chunk");

    let prov_query = prov_vec.to_vec();

    let filtered = repo
        .search_documents_pgvector(
            &mut conn,
            org_prov,
            &prov_query,
            10,
            Some(0.5),
            Some("text-embedding-3-small"),
        )
        .await
        .expect("provenance-filtered pgvector search");

    let texts: Vec<&str> = filtered
        .iter()
        .map(|(emb, _)| emb.chunk_text.as_str())
        .collect();

    assert_eq!(
        filtered.len(),
        2,
        "model_filter must drop the mismatched-model row and keep the matching \
         + legacy-untagged rows, got {texts:?}"
    );
    assert!(
        texts.contains(&"prov match"),
        "the row tagged with the query model must be kept, got {texts:?}"
    );
    assert!(
        texts.contains(&"prov legacy untagged"),
        "a legacy row with no embedding_model provenance must be kept \
         (backward compat), got {texts:?}"
    );
    assert!(
        !texts.contains(&"prov mismatch stub"),
        "a row tagged with a different embedding_model must be dropped, got {texts:?}"
    );

    // `limit` is applied AFTER over-fetch + provenance filtering: with the filter
    // active the wrapper over-fetches (×4, cap 200) then truncates to `limit`, so
    // a limit of 1 yields exactly one surviving row — and it is never the
    // mismatched-model chunk that the filter already removed.
    let limited = repo
        .search_documents_pgvector(
            &mut conn,
            org_prov,
            &prov_query,
            1,
            Some(0.5),
            Some("text-embedding-3-small"),
        )
        .await
        .expect("provenance-filtered pgvector search (limited)");

    assert_eq!(
        limited.len(),
        1,
        "limit=1 must cap the provenance-filtered result set to a single row"
    );
    assert_ne!(
        limited[0].0.chunk_text, "prov mismatch stub",
        "the retained row must never be the dropped mismatched-model chunk"
    );
}
