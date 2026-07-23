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

use axum::http::StatusCode;
use db::repositories::LlmDocumentRepository;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{
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
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM pg_extension WHERE extname = 'vector')",
    )
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

/// Seed a legacy `document_embeddings` row for `org_id`: a 1536-dim JSONB
/// array, NULL vector (column omitted so it also works without pgvector), and
/// metadata WITHOUT an `embedding_model` key — i.e. a pre-provenance legacy row.
/// Returns the new row id.
async fn seed_legacy_embedding(pool: &PgPool, org_id: Uuid, created_by: Uuid) -> Uuid {
    let doc_id = Uuid::new_v4();
    seed_document(pool, org_id, created_by, doc_id).await;
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO document_embeddings
               (organization_id, document_id, chunk_index, chunk_text, embedding, metadata)
           VALUES ($1, $2, 0, 'legacy chunk',
                   to_jsonb(array_fill(0.1::float8, ARRAY[1536])),
                   '{}'::jsonb)
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(doc_id)
    .fetch_one(pool)
    .await
    .expect("seed legacy embedding row")
}

/// Drive `migrate_embeddings_to_pgvector` on a connection bound to a freshly
/// created `NOSUPERUSER NOBYPASSRLS` role, with a super-admin request context
/// bound to `org_id`. Returns the migrated count the repo reports.
///
/// Why the role switch: `#[sqlx::test]` connects as the Postgres superuser,
/// which bypasses FORCE RLS entirely — so any cross-org visibility assertion
/// driven on the raw pool passes vacuously whether or not migration 00216's
/// `document_embeddings_super_admin` policy exists (#2418). Under a plain
/// non-superuser role FORCE actually binds, so 00216 is what OR-widens
/// visibility for `is_super_admin()` sessions from the caller's own org to
/// every org — exactly the production owner-role experience. Mirrors the
/// role-switch discipline in `crates/db/tests/report_schedule_scheduler_rls_tests.rs`
/// and `budget_rls_repo_tests.rs`.
///
/// The context is set to `(org_id, user_id, is_super_admin = TRUE)`: the
/// org-isolation policy alone would pin the back-fill to `org_id`; the 00216
/// super-admin policy is what lets it reach the other orgs' rows.
async fn migrate_as_nonsuperuser(pool: &PgPool, org_id: Uuid, user_id: Uuid) -> i64 {
    let role = format!("ppt_rls_ragmig_{}", Uuid::new_v4().simple());
    for stmt in [
        format!("CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"),
        format!("GRANT SELECT, UPDATE ON document_embeddings TO \"{role}\""),
        // The back-fill function + the functions the RLS policies evaluate
        // (`is_super_admin`, `get_current_org_id`) plus context setters.
        format!(
            "GRANT EXECUTE ON FUNCTION migrate_jsonb_to_vector(), \
             set_request_context(UUID, UUID, BOOLEAN), get_current_org_id(), \
             is_super_admin(), clear_request_context() TO \"{role}\""
        ),
        // The `document_embeddings_org_isolation` policy (00179) is OR-combined
        // with the super-admin policy for FOR ALL, so both the before/after
        // `COUNT(*)` reads and the `migrate_jsonb_to_vector()` UPDATE evaluate
        // its `get_current_org_not_deleted()` soft-delete guard, which (as a
        // SECURITY INVOKER function) reads `organizations` under this role's
        // privileges. Without this the back-fill fails with 42501 (#2418).
        // Matches the pattern in budget_rls_repo_tests.rs /
        // report_schedule_scheduler_rls_tests.rs.
        format!("GRANT SELECT ON organizations TO \"{role}\""),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(stmt))
            .execute(pool)
            .await
            .expect("grant setup");
    }

    let mut conn = pool.acquire().await.expect("acquire");
    sqlx::query("SELECT clear_request_context()")
        .execute(&mut *conn)
        .await
        .expect("clear ctx");
    sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
        .execute(&mut *conn)
        .await
        .expect("set role");
    sqlx::query("SELECT set_request_context($1, $2, TRUE)")
        .bind(org_id)
        .bind(user_id)
        .execute(&mut *conn)
        .await
        .expect("set super-admin context bound to org A");

    let migrated = LlmDocumentRepository::new(pool.clone())
        .migrate_embeddings_to_pgvector(&mut conn)
        .await
        .expect("migrate under non-superuser role");

    // Reset the session before this connection returns to the pool: sqlx does
    // not run DISCARD on release, so without this the still-SET-ROLE + still-
    // context-bound connection could be handed back for the superuser
    // verification reads below and silently apply RLS to them.
    sqlx::query("RESET ROLE")
        .execute(&mut *conn)
        .await
        .expect("reset role");
    sqlx::query("SELECT clear_request_context()")
        .execute(&mut *conn)
        .await
        .expect("clear ctx after migrate");

    migrated
}

/// True when `id`'s `embedding_vector` column is populated (read as superuser,
/// RLS-exempt, so both orgs' rows are visible for verification).
async fn embedding_vector_set(pool: &PgPool, id: Uuid) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT embedding_vector IS NOT NULL FROM document_embeddings WHERE id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .expect("re-read migrated embedding row")
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

    let (vector_set, stamped_model, assumed_marker): (bool, Option<String>, Option<String>) =
        sqlx::query_as(
            r#"SELECT embedding_vector IS NOT NULL,
                      metadata->>'embedding_model',
                      metadata->>'embedding_model_assumed'
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
    // #2321: the assumed stamp must be distinguishable from genuinely recorded
    // provenance (1536-dim also covers ada-002 — an incompatible space), so an
    // `embedding_model_assumed` marker is set alongside it on untagged rows.
    assert_eq!(
        assumed_marker.as_deref(),
        Some("true"),
        "back-fill must mark inferred provenance with embedding_model_assumed=true \
         so a later re-embedding pass can find assumed rows (#2321)"
    );
}

// Cross-org back-fill (#2321): the endpoint is documented as converting legacy
// rows across EVERY organization in one super-admin pass. `document_embeddings`
// runs under FORCE RLS; migration 00216 adds the super-admin policy that makes
// that bypass actually apply. This test seeds one legacy row in each of two
// orgs, calls `/rag/migrate` with the platform admin's session bound to org A,
// and asserts BOTH orgs' rows are converted and assumed-stamped.
//
// Caveat (called out in #2321): `#[sqlx::test]` connects as the Postgres
// superuser, which bypasses RLS entirely — so this test passes with or without
// the 00216 policy in this harness. It stays a pgvector-conversion smoke test
// pinning the cross-org contract shape, but it is NOT the RLS regression guard:
// the two `*_nonsuperuser_role_*` / `*_without_super_admin_policy_*` tests below
// (#2418) drive the same back-fill under a NOBYPASSRLS role so FORCE actually
// binds and the 00216 policy is the thing under test (2 rows with it, 1 without).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn rag_migrate_converts_legacy_rows_across_orgs(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();

    // Platform-admin principal, session bound to org A only.
    let (token, _refresh) = create_authenticated_user(&app, &user).await;
    let user_id = resolve_user_id(&app, &user).await;
    let org_a = seed_org(&app.pool, "rag-xorg-a").await;
    let org_b = seed_org(&app.pool, "rag-xorg-b").await;
    seed_membership(&app.pool, org_a, user_id, "platform_admin").await;

    // One legacy row (1536-dim JSONB, NULL vector, no provenance) per org.
    let emb_a = seed_legacy_embedding(&app.pool, org_a, user_id).await;
    let emb_b = seed_legacy_embedding(&app.pool, org_b, user_id).await;

    let session = app.session(token, org_a);
    let resp = app
        .execute(session.post("/api/v1/ai/llm/rag/migrate").build())
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "cross-org migrate must return 200; body={}",
        resp.text()
    );

    if !pgvector_present(&app).await {
        assert_eq!(
            resp.json_value()["migrated"].as_i64(),
            Some(0),
            "without pgvector the back-fill must report 0 migrated"
        );
        return;
    }

    // pgvector present: both orgs' rows must convert even though the caller's
    // session is bound to org A — this is the cross-org super-admin pass.
    assert!(
        resp.json_value()["migrated"]
            .as_i64()
            .is_some_and(|n| n >= 2),
        "cross-org back-fill must convert the legacy row in BOTH orgs; body={}",
        resp.json_value()
    );

    for emb_id in [emb_a, emb_b] {
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
            "cross-org back-fill must populate embedding_vector on {emb_id}"
        );
        assert_eq!(
            stamped_model.as_deref(),
            Some("text-embedding-3-small"),
            "cross-org back-fill must stamp assumed provenance on {emb_id}"
        );
    }
}

// #2418 — REAL cross-org RLS regression guard (positive direction).
//
// The superuser test above cannot fail if migration 00216 regresses (a
// superuser bypasses FORCE RLS). This test drives the same back-fill under a
// `NOSUPERUSER NOBYPASSRLS` role with a super-admin session bound to org A, so
// FORCE binds and `document_embeddings_super_admin` is genuinely load-bearing.
// With the policy present the back-fill must see and convert BOTH orgs' legacy
// rows → migrated count == 2. The sibling negative test proves it would be 1
// without the policy — together they pin the count at exactly the policy's
// contribution.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn rag_migrate_cross_org_under_nonsuperuser_role_converts_both(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();

    // A real user row (FK target for the seeded documents) and two orgs, one
    // legacy row each. Seeding runs as the RLS-exempt superuser.
    let (_token, _refresh) = create_authenticated_user(&app, &user).await;
    let user_id = resolve_user_id(&app, &user).await;
    let org_a = seed_org(&app.pool, "rag-xorg-role-a").await;
    let org_b = seed_org(&app.pool, "rag-xorg-role-b").await;
    let emb_a = seed_legacy_embedding(&app.pool, org_a, user_id).await;
    let emb_b = seed_legacy_embedding(&app.pool, org_b, user_id).await;

    if !pgvector_present(&app).await {
        // No pgvector → `migrate_jsonb_to_vector()` is absent and the back-fill
        // is a deterministic 0 no-op; the RLS contract can't be exercised. The
        // superuser smoke test already covers the pgvector-independent shape.
        return;
    }

    let migrated = migrate_as_nonsuperuser(&app.pool, org_a, user_id).await;
    assert_eq!(
        migrated, 2,
        "under a NOBYPASSRLS super-admin session bound to org A, migration 00216's \
         document_embeddings_super_admin policy must let the back-fill convert BOTH orgs' \
         legacy rows (would be 1 without the policy — see the negative test below)"
    );

    assert!(
        embedding_vector_set(&app.pool, emb_a).await,
        "own-org (A) legacy row must be converted"
    );
    assert!(
        embedding_vector_set(&app.pool, emb_b).await,
        "cross-org (B) legacy row must be converted via the super-admin bypass"
    );
}

// #2418 — REAL cross-org RLS regression guard (negative direction).
//
// Same setup and same non-superuser super-admin session as the positive test,
// but with migration 00216's `document_embeddings_super_admin` policy DROPPED.
// Only `document_embeddings_org_isolation` then remains, so the back-fill can
// see/convert org A's row ONLY → migrated count == 1, and org B's row stays
// unconverted. This proves the policy is the load-bearing element (not the
// harness): if a future edit drops or narrows it, the positive test's count
// falls from 2 to 1 and fails — the exact #2321 cross-org regression, now
// actually guarded.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn rag_migrate_cross_org_without_super_admin_policy_converts_only_own_org(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();

    let (_token, _refresh) = create_authenticated_user(&app, &user).await;
    let user_id = resolve_user_id(&app, &user).await;
    let org_a = seed_org(&app.pool, "rag-noxorg-a").await;
    let org_b = seed_org(&app.pool, "rag-noxorg-b").await;
    let emb_a = seed_legacy_embedding(&app.pool, org_a, user_id).await;
    let emb_b = seed_legacy_embedding(&app.pool, org_b, user_id).await;

    if !pgvector_present(&app).await {
        return;
    }

    // Remove the super-admin bypass (as the RLS-exempt superuser) to isolate its
    // effect. Without it, FORCE RLS + the org-isolation policy confine the
    // back-fill to the caller's own org.
    sqlx::query("DROP POLICY document_embeddings_super_admin ON document_embeddings")
        .execute(&app.pool)
        .await
        .expect("drop super-admin policy");

    let migrated = migrate_as_nonsuperuser(&app.pool, org_a, user_id).await;
    assert_eq!(
        migrated, 1,
        "without migration 00216's super-admin policy, a non-superuser session bound to org A \
         must convert ONLY its own org's row — the exact #2321 cross-org back-fill regression"
    );

    assert!(
        embedding_vector_set(&app.pool, emb_a).await,
        "own-org (A) row must still convert without the policy"
    );
    assert!(
        !embedding_vector_set(&app.pool, emb_b).await,
        "cross-org (B) row must remain unconverted once the super-admin policy is gone"
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
