//! Route-level pagination clamp tests for `GET /api/v1/migration/templates`
//! (Epic 66, Story 66.1 — follow-up #2051 to PR #2038).
//!
//! The list handler (`routes/migration.rs::list_templates`) sanitises the
//! caller-supplied paging params before touching the database:
//!
//! ```ignore
//! let page = query.page.unwrap_or(1).max(1);
//! let per_page = query.per_page.unwrap_or(20).clamp(1, 100);
//! let offset = (page - 1) as i64 * per_page as i64;
//! ```
//!
//! PR #2038 added this clamp but shipped without a test that actually drives
//! the route with hostile paging input. These tests close that gap: they assert
//! the clamp bounds (`per_page` -> `[1, 100]`, `page` -> `>= 1`) via the
//! `per_page` / `page` values echoed back in [`ListTemplatesResponse`], and that
//! the resulting `LIMIT`/`OFFSET` slice matches what the clamped values imply.
//!
//! These tests do NOT share the BIT-351 quarantine of `migration_db_tests.rs`:
//! that quarantine is scoped to the create/validate/preview/approve lifecycle,
//! whereas this file only seeds rows directly and reads them back through the
//! list endpoint — the same happy-path listing surface that the un-ignored
//! `infra_migration_platform_admin_tests.rs` already exercises.

#![allow(dead_code)]

mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use common::{create_authenticated_user, seed_membership, seed_org, TestApp, TestUser};

/// Subset of `routes::migration::ListTemplatesResponse` needed for assertions.
/// (The route type is not exported, so mirror the wire shape here.)
#[derive(Debug, Deserialize)]
struct ListTemplatesResponse {
    templates: Vec<TemplateSummary>,
    total: i64,
    page: i32,
    per_page: i32,
}

#[derive(Debug, Deserialize)]
struct TemplateSummary {
    id: Uuid,
    name: String,
}

/// Build a GET request carrying the platform-admin bearer token and the
/// `X-Tenant-ID` header both required to reach the `list_templates` handler.
fn get(token: &str, tenant_id: Uuid, uri: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", tenant_id.to_string())
        .body(Body::empty())
        .unwrap()
}

/// Mint an access token that carries the `platform_admin` role and a
/// `tenant_id` claim pinned to `org_id`.
///
/// The `list_templates` handler reads BOTH of these off the JWT *claims*:
/// `require_platform_admin` gates on `AuthUser::is_platform_admin()` (the
/// `role` claim) and the org scope comes from `user.tenant_id` (the
/// `tenant_id` claim). The `/auth/login` flow used by `create_authenticated_user`
/// mints a token with `role = None` / `tenant_id = None` ("roles are set when
/// org context is selected", see `routes/auth.rs`), so a login token can never
/// clear the platform-admin gate — it 403s before the query runs. We therefore
/// mint the token directly, mirroring the green `accounting_happy_path` pattern.
/// The signing secret matches `TestConfig::default().jwt_secret`, which
/// `TestApp` installs into `JWT_SECRET`.
///
/// `role` serialises via `TenantRole`'s `rename_all = "snake_case"`, so the
/// literal wire value is `"platform_admin"`.
fn mint_platform_admin_token(user_id: Uuid, org_id: Uuid, email: &str) -> String {
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde::Serialize;

    #[derive(Serialize)]
    struct Claims {
        sub: String,
        exp: i64,
        iat: i64,
        token_type: String,
        tenant_id: String,
        role: String,
        email: String,
        name: String,
    }

    let now = chrono::Utc::now().timestamp();
    encode(
        &Header::default(),
        &Claims {
            sub: user_id.to_string(),
            exp: now + 900,
            iat: now,
            token_type: "access".into(),
            tenant_id: org_id.to_string(),
            role: "platform_admin".into(),
            email: email.into(),
            name: "Migration Pagination Admin".into(),
        },
        &EncodingKey::from_secret(
            b"test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes",
        ),
    )
    .expect("mint platform-admin token")
}

/// Register + verify a user, seed a fresh org with an active `platform_admin`
/// membership for that user, then return a directly-minted platform-admin
/// access token scoped to the org (plus the org id).
///
/// The membership row is seeded to keep the fixture faithful to the real
/// authorization model (an admin *is* a member); the token's `platform_admin`
/// role claim is what actually clears `ValidatedTenantExtractor`'s membership
/// bypass and the `require_platform_admin` gate.
async fn create_platform_admin(app: &TestApp, user: &TestUser, slug: &str) -> (String, Uuid) {
    let (_login_token, _refresh) = create_authenticated_user(app, user).await;

    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("resolve user id");

    let org_id = seed_org(&app.pool, slug).await;
    seed_membership(&app.pool, org_id, user_id, "platform_admin").await;

    let token = mint_platform_admin_token(user_id, org_id, &user.email);
    (token, org_id)
}

/// Insert `count` custom `buildings` templates for `org_id` with strictly
/// increasing `updated_at` (index `i` = `NOW() + i seconds`), so the
/// `ORDER BY updated_at DESC, id DESC` list order is fully deterministic:
/// `pg-tpl-{count-1}` (newest) down to `pg-tpl-0` (oldest).
///
/// Returns the template names in that expected descending list order.
async fn seed_buildings_templates(pool: &PgPool, org_id: Uuid, count: i32) -> Vec<String> {
    for i in 0..count {
        sqlx::query(
            r#"INSERT INTO import_templates
                   (organization_id, name, description, data_type,
                    field_mappings, is_system_template, updated_at)
               VALUES ($1, $2, NULL, 'buildings', '[]'::jsonb, false,
                       NOW() + make_interval(secs => $3))"#,
        )
        .bind(org_id)
        .bind(format!("pg-tpl-{i}"))
        .bind(i as f64)
        .execute(pool)
        .await
        .expect("seed buildings template");
    }

    // Descending updated_at => highest index first.
    (0..count).rev().map(|i| format!("pg-tpl-{i}")).collect()
}

/// Shared oracle for Postgres `ORDER BY id DESC`: return `ids` sorted descending.
///
/// Postgres compares `uuid` byte-wise big-endian, which matches the derived
/// `Ord` on `uuid::Uuid`'s `[u8; 16]`, so a Rust descending sort is a faithful
/// oracle for Postgres `id DESC`. Both tie-break tests in this file
/// (`identical_updated_at_falls_back_to_deterministic_id_desc` on the strict-org
/// leg and `include_system_default_tie_group_is_deterministic_id_desc` on the
/// default leg) route their expected order through this single helper so their
/// oracles cannot drift apart.
///
/// This is the HTTP-layer twin of the repo-layer oracle in
/// `backend/crates/db/tests/migration_templates_pagination_tests.rs`
/// (`count_and_pages_are_consistent_and_disjoint`, the `sort_by(|a, b| b.cmp(a))`
/// block). Both files deliberately assert the same `id DESC` invariant at
/// different layers — repo SQL vs the `GET /api/v1/migration/templates` route —
/// so the HTTP straddling-slice coverage here is intentional, not a duplicate.
/// If you change the `id DESC` contract, update both oracles together.
fn expected_id_desc(mut ids: Vec<Uuid>) -> Vec<Uuid> {
    ids.sort_unstable();
    ids.reverse();
    ids
}

/// Insert `count` custom `buildings` templates for `org_id` that all share an
/// identical `updated_at`, collapsing the leading `ORDER BY updated_at DESC`
/// key into a single tie for every row. That makes the `id DESC` sub-sort the
/// *only* thing that decides order — the exact instability the 00212 index +
/// the `id DESC` ORDER BY key exist to pin (#2051 / #1997 / #2080).
///
/// Returns the inserted ids sorted DESC (via [`expected_id_desc`]) — the exact
/// order `ORDER BY updated_at DESC, id DESC` must produce for a single tie group.
async fn seed_tied_updated_at_templates(pool: &PgPool, org_id: Uuid, count: i32) -> Vec<Uuid> {
    let mut ids = Vec::with_capacity(count as usize);
    for i in 0..count {
        // Pin one literal timestamp for every row so no two rows can differ on
        // the leading sort key.
        let id: Uuid = sqlx::query_scalar(
            r#"INSERT INTO import_templates
                   (organization_id, name, description, data_type,
                    field_mappings, is_system_template, updated_at)
               VALUES ($1, $2, NULL, 'buildings', '[]'::jsonb, false,
                       TIMESTAMPTZ '2026-01-01 12:00:00+00')
               RETURNING id"#,
        )
        .bind(org_id)
        .bind(format!("tie-tpl-{i}"))
        .fetch_one(pool)
        .await
        .expect("seed tied-updated_at template");
        ids.push(id);
    }

    expected_id_desc(ids)
}

/// Seed a tie group that STRADDLES both legs of the default `include_system=true`
/// OR predicate: `org_count` org-scoped rows (`organization_id = org_id`) plus one
/// system template (`organization_id IS NULL`, `is_system_template = true`), all
/// sharing a single far-future `updated_at`.
///
/// The shared timestamp is pinned in the year 2999 on purpose: migration 00198
/// seeds three system templates (`organization_id IS NULL`) at migration time, so
/// their `updated_at` is "now-ish". Placing this tie group far in the future
/// guarantees it occupies the HEAD of the `updated_at DESC, id DESC` ordering
/// regardless of how many system templates the schema seeds — the caller can then
/// assert against a fixed prefix of the list without counting pre-seeded rows.
///
/// Returns the inserted ids (org rows + the system row) sorted DESC (via
/// [`expected_id_desc`]) — the exact order the combined org+system tie group must
/// produce at the head of the default-leg list.
async fn seed_tied_org_and_system_templates(
    pool: &PgPool,
    org_id: Uuid,
    org_count: i32,
) -> Vec<Uuid> {
    let mut ids = Vec::with_capacity(org_count as usize + 1);
    for i in 0..org_count {
        let id: Uuid = sqlx::query_scalar(
            r#"INSERT INTO import_templates
                   (organization_id, name, description, data_type,
                    field_mappings, is_system_template, updated_at)
               VALUES ($1, $2, NULL, 'buildings', '[]'::jsonb, false,
                       TIMESTAMPTZ '2999-01-01 12:00:00+00')
               RETURNING id"#,
        )
        .bind(org_id)
        .bind(format!("tie-org-{i}"))
        .fetch_one(pool)
        .await
        .expect("seed tied org template");
        ids.push(id);
    }

    // One system template (organization_id IS NULL) sharing the exact instant,
    // so the tie group spans both legs of the include_system OR.
    let sys_id: Uuid = sqlx::query_scalar(
        r#"INSERT INTO import_templates
               (organization_id, name, description, data_type,
                field_mappings, is_system_template, updated_at)
           VALUES (NULL, $1, NULL, 'buildings', '[]'::jsonb, true,
                   TIMESTAMPTZ '2999-01-01 12:00:00+00')
           RETURNING id"#,
    )
    .bind("tie-system")
    .fetch_one(pool)
    .await
    .expect("seed tied system template");
    ids.push(sys_id);

    expected_id_desc(ids)
}

/// `per_page=0` and `per_page=-5` must both clamp up to 1.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn per_page_zero_and_negative_clamp_to_one(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (token, org_id) = create_platform_admin(&app, &TestUser::new(), "pgclamp1").await;
    seed_buildings_templates(&pool, org_id, 3).await;

    for raw in ["0", "-5"] {
        let uri = format!("/api/v1/migration/templates?include_system=false&per_page={raw}");
        let resp = app.execute(get(&token, org_id, &uri)).await;
        assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
        let body: ListTemplatesResponse = resp.json();

        assert_eq!(body.per_page, 1, "per_page={raw} must clamp to 1");
        assert_eq!(body.page, 1, "page defaults to 1");
        assert_eq!(body.total, 3, "total ignores paging window");
        assert_eq!(
            body.templates.len(),
            1,
            "per_page clamped to 1 => at most one row for per_page={raw}"
        );
    }
}

/// `per_page=500` must clamp down to 100. With only a handful of rows the
/// returned slice equals the full set, and the echoed `per_page` reflects the
/// clamp (not the requested 500).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn per_page_above_max_clamps_to_hundred(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (token, org_id) = create_platform_admin(&app, &TestUser::new(), "pgclamp2").await;
    seed_buildings_templates(&pool, org_id, 5).await;

    let uri = "/api/v1/migration/templates?include_system=false&per_page=500";
    let resp = app.execute(get(&token, org_id, uri)).await;
    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    let body: ListTemplatesResponse = resp.json();

    assert_eq!(body.per_page, 100, "per_page=500 must clamp to 100");
    assert_eq!(body.total, 5);
    assert_eq!(
        body.templates.len(),
        5,
        "clamp to 100 still returns all 5 seeded rows (5 < 100)"
    );
}

/// `page=0` must clamp up to 1 (offset 0 => first page).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn page_zero_clamps_to_one(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (token, org_id) = create_platform_admin(&app, &TestUser::new(), "pgclamp3").await;
    let expected_order = seed_buildings_templates(&pool, org_id, 5).await;

    let uri = "/api/v1/migration/templates?include_system=false&page=0&per_page=2";
    let resp = app.execute(get(&token, org_id, uri)).await;
    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    let body: ListTemplatesResponse = resp.json();

    assert_eq!(body.page, 1, "page=0 must clamp to 1");
    assert_eq!(body.per_page, 2);
    assert_eq!(body.total, 5);
    // Offset 0 => the first two rows in updated_at DESC, id DESC order.
    let names: Vec<&str> = body.templates.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, vec![&expected_order[0][..], &expected_order[1][..]]);
}

/// `page=2&per_page=2` selects offset `(2-1)*2 = 2` — the third and fourth
/// rows in `updated_at DESC, id DESC` order.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn page_two_returns_expected_offset_slice(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (token, org_id) = create_platform_admin(&app, &TestUser::new(), "pgclamp4").await;
    let expected_order = seed_buildings_templates(&pool, org_id, 5).await;

    let uri = "/api/v1/migration/templates?include_system=false&page=2&per_page=2";
    let resp = app.execute(get(&token, org_id, uri)).await;
    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    let body: ListTemplatesResponse = resp.json();

    assert_eq!(body.page, 2);
    assert_eq!(body.per_page, 2);
    assert_eq!(body.total, 5);

    // offset = (page - 1) * per_page = 2 => rows at indices 2 and 3.
    let names: Vec<&str> = body.templates.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(
        names,
        vec![&expected_order[2][..], &expected_order[3][..]],
        "page 2 must be the offset-2 slice of the DESC ordering"
    );
}

/// The whole point of the `id DESC` tie-break (00212's third index column and
/// the second ORDER BY key) is to make paging deterministic when rows share an
/// `updated_at`. The clamp tests above give every row a distinct `updated_at`,
/// so their slice assertions would pass even if the tiebreak regressed. This
/// test pins the tiebreak directly (#2080): it seeds one tie group — several
/// rows with an identical `updated_at` — and asserts both the full-list order
/// and an offset slice straddling that group resolve to a strict `id DESC`
/// order.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn identical_updated_at_falls_back_to_deterministic_id_desc(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (token, org_id) = create_platform_admin(&app, &TestUser::new(), "pgtie").await;

    // Four rows, all with the same updated_at => a single 4-row tie group.
    let ids_desc = seed_tied_updated_at_templates(&pool, org_id, 4).await;

    // (1) Full list order must equal the seeded ids sorted DESC. Without the
    // `id DESC` key this order would be unspecified across the tie group.
    let uri = "/api/v1/migration/templates?include_system=false&per_page=100";
    let resp = app.execute(get(&token, org_id, uri)).await;
    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    let body: ListTemplatesResponse = resp.json();
    assert_eq!(body.total, 4, "all four tied rows are listed");
    let full_order: Vec<Uuid> = body.templates.iter().map(|t| t.id).collect();
    assert_eq!(
        full_order, ids_desc,
        "tied updated_at must resolve to a strict id DESC order"
    );

    // (2) Offset slice straddling the tie group: page=2&per_page=2 => offset
    // (2-1)*2 = 2 => the 3rd and 4th ids of the DESC order. This slice is only
    // stable because the id tiebreak fixes the intra-tie ordering.
    let uri = "/api/v1/migration/templates?include_system=false&page=2&per_page=2";
    let resp = app.execute(get(&token, org_id, uri)).await;
    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    let body: ListTemplatesResponse = resp.json();
    assert_eq!(body.page, 2);
    assert_eq!(body.per_page, 2);
    assert_eq!(body.total, 4);
    let slice: Vec<Uuid> = body.templates.iter().map(|t| t.id).collect();
    assert_eq!(
        slice,
        vec![ids_desc[2], ids_desc[3]],
        "page-2 slice of a single tie group must be the deterministic id DESC sub-slice"
    );
}

/// Sibling of `identical_updated_at_falls_back_to_deterministic_id_desc`, on the
/// DEFAULT and more common `include_system=true` leg (#2109, follow-up to PR
/// #2092). The strict-org test only exercises `include_system=false`, whose query
/// is a single equality on the leading index column and so walks the 00212 index
/// in order (no top-level sort). The default leg is different: migration 00212's
/// own comment flags that `WHERE (organization_id = $1 OR organization_id IS NULL)
/// ORDER BY updated_at DESC, id DESC` resolves via BitmapOr -> Bitmap Heap Scan
/// (index order discarded) -> a top-level Sort. So on this leg index order does
/// NOT carry the tie-break — only the explicit `id DESC` ORDER BY key does, and
/// this is the path real callers hit.
///
/// This seeds a tie group that straddles BOTH legs of the OR — org-scoped rows
/// AND a system template (`organization_id IS NULL`) sharing one `updated_at` —
/// and asserts the combined group resolves to a strict `id DESC` through the
/// BitmapOr/Sort, both for the full-list head and for an offset slice cutting
/// through the group.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn include_system_default_tie_group_is_deterministic_id_desc(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (token, org_id) = create_platform_admin(&app, &TestUser::new(), "pgtiesys").await;

    // Three org rows + one system row, all sharing a far-future updated_at =>
    // a 4-row tie group guaranteed to sit at the head of the DESC ordering,
    // ahead of the migration-seeded system templates.
    let org_count = 3;
    let head_desc = seed_tied_org_and_system_templates(&pool, org_id, org_count).await;
    assert_eq!(
        head_desc.len(),
        (org_count + 1) as usize,
        "tie group is org_count org rows + one system row"
    );

    // (1) Default leg: OMIT include_system entirely so the serde default (true)
    // is exercised — the exact predicate real callers hit. The list also holds
    // the migration-seeded system templates, but our tie group is strictly newer,
    // so it must occupy the first `head_desc.len()` slots in
    // (updated_at DESC, id DESC) order across the combined org+system group.
    let uri = "/api/v1/migration/templates?per_page=100";
    let resp = app.execute(get(&token, org_id, uri)).await;
    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    let body: ListTemplatesResponse = resp.json();
    assert!(
        body.total >= head_desc.len() as i64,
        "include_system total must cover org rows + all system rows (got {})",
        body.total
    );
    let head: Vec<Uuid> = body
        .templates
        .iter()
        .take(head_desc.len())
        .map(|t| t.id)
        .collect();
    assert_eq!(
        head, head_desc,
        "combined org+system tie group must resolve to strict id DESC through the BitmapOr/Sort path"
    );

    // (2) Offset slice straddling the combined tie group: page=2&per_page=2 =>
    // offset (2-1)*2 = 2 => the 3rd and 4th ids of the combined DESC order.
    // Because the whole tie group is newest, this slice lands entirely inside it
    // and is only stable because the `id DESC` key survives the BitmapOr -> Sort.
    let uri = "/api/v1/migration/templates?page=2&per_page=2";
    let resp = app.execute(get(&token, org_id, uri)).await;
    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    let body: ListTemplatesResponse = resp.json();
    assert_eq!(body.page, 2);
    assert_eq!(body.per_page, 2);
    let slice: Vec<Uuid> = body.templates.iter().map(|t| t.id).collect();
    assert_eq!(
        slice,
        vec![head_desc[2], head_desc[3]],
        "page-2 slice of the combined org+system tie group must be the deterministic id DESC sub-slice"
    );
}
