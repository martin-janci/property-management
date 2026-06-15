//! Building-scope document read-path RLS tests (issue #1356).
//!
//! Background
//! ----------
//! `DocumentRepository::list_accessible_rls` / `check_access_rls` are the
//! building/unit/role-aware document read path (Story 7A.3, PR #1311). They
//! gate visibility with the JSONB existence operator `?|`:
//!
//! ```sql
//! access_scope = 'building' AND access_target_ids ?| $3
//! ```
//!
//! `?|` is `jsonb ?| text[]` — its right operand MUST be a Postgres `text[]`.
//! The original code bound that operand as `jsonb` (`serde_json::to_value(..)`),
//! so Postgres tried to resolve `jsonb ?| jsonb` (no such operator) and the
//! query errored at runtime. The building/unit/role scope branches therefore
//! never matched on the read path. The fix binds the membership lists as
//! `text[]`; these tests pin the corrected behaviour.
//!
//! What is asserted
//! ----------------
//! - **Positive (in-scope visible):** a building-scoped doc whose target
//!   building is in the reader's building set is returned; likewise unit / role
//!   / organization-wide docs.
//! - **Negative (out-of-scope hidden):** a building-scoped doc for a building
//!   the reader is NOT in is hidden; a reader with an empty membership set sees
//!   only organization-wide docs.
//! - **Tenant isolation:** a doc in another org with the *same* building id is
//!   never returned — `list_accessible_rls` carries an explicit
//!   `organization_id = $1` predicate (defence-in-depth on top of the
//!   `document_tenant_isolation` RLS policy).
//!
//! The query is exercised through a connection that has the request context set
//! via `db::set_request_context` (the same call `RlsConnection` makes in
//! production). The CI/test database role bypasses RLS, so these assertions pin
//! the application-level predicates inside the query; production additionally
//! enforces the FORCE-RLS `document_tenant_isolation` policy on top.

#[allow(dead_code)]
mod common;

use db::models::DocumentListQuery;
use db::repositories::DocumentRepository;
use sqlx::PgPool;
use uuid::Uuid;

use common::seed_org;

/// Insert a `users` row (minimal) and return its id, for use as `created_by`.
async fn seed_user(pool: &PgPool, label: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name)
           VALUES ($1, 'x', $2) RETURNING id"#,
    )
    .bind(format!("{label}-{}@doc-rls.test", Uuid::new_v4()))
    .bind(format!("Doc RLS {label}"))
    .fetch_one(pool)
    .await
    .expect("seed_user")
}

/// Insert a `documents` row with an explicit access scope and return its id.
async fn seed_doc(
    pool: &PgPool,
    org_id: Uuid,
    created_by: Uuid,
    title: &str,
    scope: &str,
    target_ids: &[Uuid],
    roles: &[&str],
) -> Uuid {
    let target_json = serde_json::json!(target_ids
        .iter()
        .map(Uuid::to_string)
        .collect::<Vec<_>>());
    let roles_json = serde_json::json!(roles);
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO documents
               (organization_id, title, category, file_key, file_name, mime_type,
                size_bytes, access_scope, access_target_ids, access_roles, created_by)
           VALUES ($1, $2, 'other', $3, 'f.pdf', 'application/pdf', 1024,
                   $4::document_access_scope, $5::jsonb, $6::jsonb, $7)
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(title)
    .bind(format!("{org_id}/{}.pdf", Uuid::new_v4()))
    .bind(scope)
    .bind(target_json)
    .bind(roles_json)
    .bind(created_by)
    .fetch_one(pool)
    .await
    .expect("seed_doc")
}

/// Full positive + negative sweep of the building-scope list read path.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_accessible_rls_enforces_building_scope(pool: PgPool) {
    let repo = DocumentRepository::new(pool.clone());

    // Creator is a different user than the readers below, so the
    // "creator always has access" branch never decides visibility here.
    let creator = seed_user(&pool, "creator").await;
    let org_a = seed_org(&pool, "a").await;
    let org_c = seed_org(&pool, "c").await;

    let building_b1 = Uuid::new_v4();
    let building_b2 = Uuid::new_v4();
    let unit_u1 = Uuid::new_v4();

    // Org A documents across scopes.
    seed_doc(&pool, org_a, creator, "Org Wide", "organization", &[], &[]).await;
    let b1_doc =
        seed_doc(&pool, org_a, creator, "Building B1", "building", &[building_b1], &[]).await;
    let b2_doc =
        seed_doc(&pool, org_a, creator, "Building B2", "building", &[building_b2], &[]).await;
    seed_doc(&pool, org_a, creator, "Unit U1", "unit", &[unit_u1], &[]).await;
    seed_doc(&pool, org_a, creator, "Owner Role", "role", &[], &["owner"]).await;

    // Cross-tenant trap: a doc in another org scoped to the SAME building id.
    // Tenant isolation must hide it even though the building id matches.
    seed_doc(&pool, org_c, creator, "Cross-Org B1", "building", &[building_b1], &[]).await;

    let mut conn = pool.acquire().await.expect("acquire");

    // ---- Reader IN building B1 and unit U1, with role `owner` ----
    let reader = Uuid::new_v4();
    db::set_request_context(&mut *conn, Some(org_a), Some(reader), false)
        .await
        .expect("set context");

    let docs = repo
        .list_accessible_rls(
            &mut *conn,
            org_a,
            reader,
            &[building_b1],
            &[unit_u1],
            &["owner".to_string()],
            DocumentListQuery::default(),
        )
        .await
        .expect("list_accessible_rls");
    let titles: Vec<&str> = docs.iter().map(|d| d.title.as_str()).collect();

    // Positive — in-scope documents visible.
    assert!(titles.contains(&"Org Wide"), "org-wide doc should be visible: {titles:?}");
    assert!(titles.contains(&"Building B1"), "in-scope building doc should be visible: {titles:?}");
    assert!(titles.contains(&"Unit U1"), "in-scope unit doc should be visible: {titles:?}");
    assert!(titles.contains(&"Owner Role"), "in-scope role doc should be visible: {titles:?}");

    // Negative — out-of-scope building hidden.
    assert!(
        !titles.contains(&"Building B2"),
        "out-of-scope building doc must be hidden: {titles:?}"
    );

    // Tenant isolation — another org's doc with the same building id is hidden.
    assert!(!titles.contains(&"Cross-Org B1"), "cross-tenant doc must be hidden: {titles:?}");

    // ---- Reader with NO building/unit membership and no roles ----
    let stranger = Uuid::new_v4();
    db::set_request_context(&mut *conn, Some(org_a), Some(stranger), false)
        .await
        .expect("set context");

    let docs = repo
        .list_accessible_rls(
            &mut *conn,
            org_a,
            stranger,
            &[],
            &[],
            &[],
            DocumentListQuery::default(),
        )
        .await
        .expect("list_accessible_rls (stranger)");
    let titles: Vec<&str> = docs.iter().map(|d| d.title.as_str()).collect();

    assert!(titles.contains(&"Org Wide"), "org-wide doc visible to everyone in org: {titles:?}");
    assert!(!titles.contains(&"Building B1"), "building doc hidden from non-member: {titles:?}");
    assert!(!titles.contains(&"Unit U1"), "unit doc hidden from non-member: {titles:?}");
    assert!(!titles.contains(&"Owner Role"), "role doc hidden from non-role user: {titles:?}");

    // ---- check_access_rls single-document gate (same org) ----
    // In-scope building doc: granted.
    let in_scope = repo
        .check_access_rls(&mut *conn, b1_doc, reader, &[building_b1], &[], &[])
        .await
        .expect("check_access_rls in-scope");
    assert!(in_scope, "reader in building B1 should pass the single-doc gate");

    // Out-of-scope building doc: denied.
    let out_of_scope = repo
        .check_access_rls(&mut *conn, b2_doc, reader, &[building_b1], &[], &[])
        .await
        .expect("check_access_rls out-of-scope");
    assert!(!out_of_scope, "reader not in building B2 must fail the single-doc gate");
}
