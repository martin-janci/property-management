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
    let target_json = serde_json::json!(target_ids.iter().map(Uuid::to_string).collect::<Vec<_>>());
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
    let b1_doc = seed_doc(
        &pool,
        org_a,
        creator,
        "Building B1",
        "building",
        &[building_b1],
        &[],
    )
    .await;
    let b2_doc = seed_doc(
        &pool,
        org_a,
        creator,
        "Building B2",
        "building",
        &[building_b2],
        &[],
    )
    .await;
    seed_doc(&pool, org_a, creator, "Unit U1", "unit", &[unit_u1], &[]).await;
    seed_doc(&pool, org_a, creator, "Owner Role", "role", &[], &["owner"]).await;

    // Cross-tenant trap: a doc in another org scoped to the SAME building id.
    // Tenant isolation must hide it even though the building id matches.
    seed_doc(
        &pool,
        org_c,
        creator,
        "Cross-Org B1",
        "building",
        &[building_b1],
        &[],
    )
    .await;

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
    assert!(
        titles.contains(&"Org Wide"),
        "org-wide doc should be visible: {titles:?}"
    );
    assert!(
        titles.contains(&"Building B1"),
        "in-scope building doc should be visible: {titles:?}"
    );
    assert!(
        titles.contains(&"Unit U1"),
        "in-scope unit doc should be visible: {titles:?}"
    );
    assert!(
        titles.contains(&"Owner Role"),
        "in-scope role doc should be visible: {titles:?}"
    );

    // Negative — out-of-scope building hidden.
    assert!(
        !titles.contains(&"Building B2"),
        "out-of-scope building doc must be hidden: {titles:?}"
    );

    // Tenant isolation — another org's doc with the same building id is hidden.
    assert!(
        !titles.contains(&"Cross-Org B1"),
        "cross-tenant doc must be hidden: {titles:?}"
    );

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

    assert!(
        titles.contains(&"Org Wide"),
        "org-wide doc visible to everyone in org: {titles:?}"
    );
    assert!(
        !titles.contains(&"Building B1"),
        "building doc hidden from non-member: {titles:?}"
    );
    assert!(
        !titles.contains(&"Unit U1"),
        "unit doc hidden from non-member: {titles:?}"
    );
    assert!(
        !titles.contains(&"Owner Role"),
        "role doc hidden from non-role user: {titles:?}"
    );

    // ---- check_access_rls single-document gate (same org) ----
    // In-scope building doc: granted.
    let in_scope = repo
        .check_access_rls(&mut *conn, b1_doc, reader, &[building_b1], &[], &[])
        .await
        .expect("check_access_rls in-scope");
    assert!(
        in_scope,
        "reader in building B1 should pass the single-doc gate"
    );

    // Out-of-scope building doc: denied.
    let out_of_scope = repo
        .check_access_rls(&mut *conn, b2_doc, reader, &[building_b1], &[], &[])
        .await
        .expect("check_access_rls out-of-scope");
    assert!(
        !out_of_scope,
        "reader not in building B2 must fail the single-doc gate"
    );
}

/// Insert a building and return its id.
async fn seed_building(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, name, street, city, postal_code, country)
           VALUES ($1, 'Bldg', 'Addr', 'City', '00000', 'Country') RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed_building")
}

/// Insert a unit in a building and return its id.
async fn seed_unit(pool: &PgPool, building_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO units (building_id, designation) VALUES ($1, 'U-1') RETURNING id"#,
    )
    .bind(building_id)
    .fetch_one(pool)
    .await
    .expect("seed_unit")
}

/// Insert a `unit_residents` row. `ended` controls whether the residency is
/// still active (`end_date IS NULL`) or moved out 30 days ago.
async fn seed_resident(pool: &PgPool, unit_id: Uuid, user_id: Uuid, ended: bool) {
    let sql = if ended {
        // Moved out: a past `end_date` makes the residency inactive. `start_date`
        // precedes it to satisfy the `valid_date_range` check (end >= start).
        r#"INSERT INTO unit_residents (unit_id, user_id, resident_type, start_date, end_date)
           VALUES ($1, $2, 'tenant', CURRENT_DATE - 60, CURRENT_DATE - 30)"#
    } else {
        // Current resident: open-ended residency.
        r#"INSERT INTO unit_residents (unit_id, user_id, resident_type)
           VALUES ($1, $2, 'tenant')"#
    };
    sqlx::query(sql)
        .bind(unit_id)
        .bind(user_id)
        .execute(pool)
        .await
        .expect("seed_resident");
}

/// Story 3.3 AC3 (PM #981 gap 6): when a manager ends a resident's association,
/// the resident must lose access to unit/building-scoped content while a current
/// resident keeps it and historical records stay readable by the manager.
///
/// `DocumentRepository::user_scope_memberships_rls` is the single chokepoint that
/// resolves a user into their `(building_ids, unit_ids)` for the document read
/// path (both `list_accessible_rls` and the download/preview gate). It excludes
/// ended residencies with `unit_residents.end_date IS NULL`. This test pins:
///   - the chokepoint returns the unit/building for a current resident,
///   - it returns NOTHING for an ended (moved-out) resident, and
///   - the end-to-end effect: the ended resident no longer sees the unit/building
///     document, the current resident does, and the manager/creator still does.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn user_scope_memberships_rls_revokes_on_move_out(pool: PgPool) {
    let repo = DocumentRepository::new(pool.clone());

    let manager = seed_user(&pool, "manager").await;
    let org = seed_org(&pool, "moveout").await;
    let building = seed_building(&pool, org).await;
    let unit = seed_unit(&pool, building).await;

    // Two residents of the same unit: one current, one moved out.
    let current = seed_user(&pool, "current").await;
    let ended = seed_user(&pool, "ended").await;
    seed_resident(&pool, unit, current, false).await;
    seed_resident(&pool, unit, ended, true).await;

    // Manager-authored unit- and building-scoped documents (the "unit content").
    seed_doc(&pool, org, manager, "Org Wide", "organization", &[], &[]).await;
    seed_doc(&pool, org, manager, "Building Doc", "building", &[building], &[]).await;
    seed_doc(&pool, org, manager, "Unit Doc", "unit", &[unit], &[]).await;

    let mut conn = pool.acquire().await.expect("acquire");
    db::set_request_context(&mut *conn, Some(org), Some(manager), false)
        .await
        .expect("set context");

    // ---- Chokepoint: current resident resolves to the unit + building ----
    let (current_buildings, current_units) = repo
        .user_scope_memberships_rls(&mut *conn, current)
        .await
        .expect("user_scope_memberships_rls (current)");
    assert!(
        current_buildings.contains(&building) && current_units.contains(&unit),
        "current resident must resolve to their unit/building: \
         buildings={current_buildings:?} units={current_units:?}"
    );

    // ---- Chokepoint: ended resident resolves to NOTHING ----
    let (ended_buildings, ended_units) = repo
        .user_scope_memberships_rls(&mut *conn, ended)
        .await
        .expect("user_scope_memberships_rls (ended)");
    assert!(
        ended_buildings.is_empty() && ended_units.is_empty(),
        "moved-out resident must resolve to no unit/building (access revoked): \
         buildings={ended_buildings:?} units={ended_units:?}"
    );

    // ---- End-to-end: current resident sees unit/building docs ----
    let current_titles: Vec<String> = repo
        .list_accessible_rls(
            &mut *conn,
            org,
            current,
            &current_buildings,
            &current_units,
            &[],
            DocumentListQuery::default(),
        )
        .await
        .expect("list (current)")
        .into_iter()
        .map(|d| d.title)
        .collect();
    assert!(
        current_titles.iter().any(|t| t == "Building Doc")
            && current_titles.iter().any(|t| t == "Unit Doc"),
        "current resident must see unit/building documents: {current_titles:?}"
    );

    // ---- End-to-end: ended resident sees ONLY org-wide content ----
    let ended_titles: Vec<String> = repo
        .list_accessible_rls(
            &mut *conn,
            org,
            ended,
            &ended_buildings,
            &ended_units,
            &[],
            DocumentListQuery::default(),
        )
        .await
        .expect("list (ended)")
        .into_iter()
        .map(|d| d.title)
        .collect();
    assert!(
        ended_titles.iter().any(|t| t == "Org Wide"),
        "ended resident keeps org-wide content (org membership is separate): {ended_titles:?}"
    );
    assert!(
        !ended_titles.iter().any(|t| t == "Building Doc")
            && !ended_titles.iter().any(|t| t == "Unit Doc"),
        "moved-out resident must NOT see unit/building documents: {ended_titles:?}"
    );

    // ---- Historical records still readable by the manager (creator branch) ----
    let manager_titles: Vec<String> = repo
        .list_accessible_rls(
            &mut *conn,
            org,
            manager,
            &[],
            &[],
            &[],
            DocumentListQuery::default(),
        )
        .await
        .expect("list (manager)")
        .into_iter()
        .map(|d| d.title)
        .collect();
    assert!(
        manager_titles.iter().any(|t| t == "Building Doc")
            && manager_titles.iter().any(|t| t == "Unit Doc"),
        "manager (creator) must still read the historical unit/building docs: {manager_titles:?}"
    );
}
