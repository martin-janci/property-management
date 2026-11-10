//! Real-SQL data-quality test for announcement fan-out metrics (issue #2484,
//! follow-up to PR #2455 / #2612).
//!
//! `AnnouncementRepository::get_fanout_metrics_rls` reports delivered / read /
//! acknowledged counts per targeting scope (`all` | `building` | `units` |
//! `roles`). `delivered` is the *scope-aware* audience resolved by the real
//! targeting SQL — the same cross-tenant-safe joins the notification fan-out
//! uses. Before this metric existed the only audience number was
//! `AcknowledgmentStats::total_targeted`, which counts every active org member
//! regardless of `target_type` and therefore over-counts a building/unit/role
//! announcement.
//!
//! These tests exercise the actual query against Postgres (not a pure-Rust
//! re-model of the audience), so a regression that drops the
//! `b.organization_id = a.organization_id` cross-tenant scope — the #2484
//! concern — surfaces as a metric that no longer matches the seeded in-scope
//! audience, and the assertions fail. A drifted Rust re-implementation cannot
//! satisfy them.
//!
//! Targeting contract (migration 00015 + `announcement_targeting_predicate!`):
//! `building`/`units` target_ids hold the UUIDs as text; `roles` target_ids hold
//! role-name strings matching `user_memberships.role`.

use db::repositories::AnnouncementRepository;
use serde_json::Value as Json;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

async fn set_ctx(conn: &mut PgConnection, org: Option<Uuid>, user: Option<Uuid>, su: bool) {
    sqlx::query("SELECT set_request_context($1, $2, $3)")
        .bind(org)
        .bind(user)
        .bind(su)
        .execute(conn)
        .await
        .expect("set_request_context");
}

async fn insert_org(conn: &mut PgConnection, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO organizations (name, slug, contact_email, status) \
         VALUES ($1, $2, $3, 'active') RETURNING id",
    )
    .bind(format!("Fanout Metrics {slug}"))
    .bind(slug)
    .bind(format!("{slug}@fanout-metrics.test"))
    .fetch_one(conn)
    .await
    .expect("insert org")
}

async fn insert_user(conn: &mut PgConnection, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (email, password_hash, name, status, email_verified_at) \
         VALUES ($1, 'test_hash', 'FanoutUser', 'active', NOW()) RETURNING id",
    )
    .bind(email)
    .fetch_one(conn)
    .await
    .expect("insert user")
}

async fn insert_building(conn: &mut PgConnection, org: Uuid, name: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO buildings (organization_id, name, street, city, postal_code, country) \
         VALUES ($1, $2, 'Addr', 'City', '00000', 'Country') RETURNING id",
    )
    .bind(org)
    .bind(name)
    .fetch_one(conn)
    .await
    .expect("insert building")
}

async fn insert_unit(conn: &mut PgConnection, building: Uuid, designation: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO units (building_id, designation) VALUES ($1, $2) RETURNING id",
    )
    .bind(building)
    .bind(designation)
    .fetch_one(conn)
    .await
    .expect("insert unit")
}

async fn insert_resident(conn: &mut PgConnection, unit: Uuid, user: Uuid, kind: &str) {
    sqlx::query(
        "INSERT INTO unit_residents (unit_id, user_id, resident_type) \
         VALUES ($1, $2, $3::resident_type)",
    )
    .bind(unit)
    .bind(user)
    .bind(kind)
    .execute(conn)
    .await
    .expect("insert unit resident");
}

async fn insert_membership(conn: &mut PgConnection, user: Uuid, org: Uuid, role: &str) {
    sqlx::query(
        "INSERT INTO user_memberships (user_id, organization_id, role) VALUES ($1, $2, $3)",
    )
    .bind(user)
    .bind(org)
    .bind(role)
    .execute(conn)
    .await
    .expect("insert membership");
}

async fn insert_published_ann(
    conn: &mut PgConnection,
    org: Uuid,
    author: Uuid,
    target_type: &str,
    target_ids: Json,
    title: &str,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO announcements \
            (organization_id, author_id, title, content, target_type, target_ids, status, published_at) \
         VALUES ($1, $2, $3, 'body', $4::announcement_target_type, $5, 'published', NOW()) \
         RETURNING id",
    )
    .bind(org)
    .bind(author)
    .bind(title)
    .bind(target_type)
    .bind(target_ids)
    .fetch_one(conn)
    .await
    .expect("insert published announcement")
}

async fn insert_read(conn: &mut PgConnection, ann: Uuid, user: Uuid, acknowledged: bool) {
    if acknowledged {
        sqlx::query(
            "INSERT INTO announcement_reads (announcement_id, user_id, acknowledged_at) \
             VALUES ($1, $2, NOW())",
        )
        .bind(ann)
        .bind(user)
        .execute(conn)
        .await
        .expect("insert read (acknowledged)");
    } else {
        sqlx::query("INSERT INTO announcement_reads (announcement_id, user_id) VALUES ($1, $2)")
            .bind(ann)
            .bind(user)
            .execute(conn)
            .await
            .expect("insert read");
    }
}

/// Full-coverage fan-out metric check: every targeting scope reports the
/// scope-aware audience, cross-tenant targets are excluded, and read/ack counts
/// match `announcement_reads`.
#[sqlx::test(migrations = "./migrations")]
async fn fanout_metrics_report_scope_aware_delivered_read_ack(pool: PgPool) {
    // ---- seed everything under a super-admin context (bypasses RLS WITH CHECK) ----
    let mut sa = pool.acquire().await.unwrap();
    set_ctx(&mut sa, None, None, true).await;

    let org_a = insert_org(&mut sa, "fm-a").await;
    let org_b = insert_org(&mut sa, "fm-b").await;
    let author = insert_user(&mut sa, "author@fanout-metrics.test").await;

    // Org A: one building with two units, plus a manager without a residency.
    let building_a = insert_building(&mut sa, org_a, "Bldg A").await;
    let unit_a1 = insert_unit(&mut sa, building_a, "A1").await;
    let unit_a2 = insert_unit(&mut sa, building_a, "A2").await;

    let manager_a = insert_user(&mut sa, "manager@fanout-metrics.test").await;
    let resident_a1 = insert_user(&mut sa, "res-a1@fanout-metrics.test").await;
    let resident_a2 = insert_user(&mut sa, "res-a2@fanout-metrics.test").await;

    insert_membership(&mut sa, manager_a, org_a, "manager").await;
    insert_membership(&mut sa, resident_a1, org_a, "resident").await;
    insert_membership(&mut sa, resident_a2, org_a, "resident").await;
    insert_resident(&mut sa, unit_a1, resident_a1, "owner").await;
    insert_resident(&mut sa, unit_a2, resident_a2, "tenant").await;

    // Org B: a separate tenant whose building/unit/resident must never be
    // counted by an org-A announcement even if their ids leak into target_ids.
    let building_b = insert_building(&mut sa, org_b, "Bldg B").await;
    let unit_b1 = insert_unit(&mut sa, building_b, "B1").await;
    let resident_b = insert_user(&mut sa, "res-b@fanout-metrics.test").await;
    insert_membership(&mut sa, resident_b, org_b, "resident").await;
    insert_resident(&mut sa, unit_b1, resident_b, "owner").await;

    // ---- announcements (all owned by org A) ----
    let ann_all =
        insert_published_ann(&mut sa, org_a, author, "all", serde_json::json!([]), "All").await;
    // building + units + roles announcements deliberately also carry an org-B id
    // (the bypassed-validation / cross-tenant leak path #2484 guards against).
    let ann_building = insert_published_ann(
        &mut sa,
        org_a,
        author,
        "building",
        serde_json::json!([building_a.to_string(), building_b.to_string()]),
        "Building A (+leaked B)",
    )
    .await;
    let ann_units = insert_published_ann(
        &mut sa,
        org_a,
        author,
        "units",
        serde_json::json!([unit_a1.to_string(), unit_b1.to_string()]),
        "Unit A1 (+leaked B1)",
    )
    .await;
    let ann_roles = insert_published_ann(
        &mut sa,
        org_a,
        author,
        "roles",
        serde_json::json!(["manager"]),
        "Managers only",
    )
    .await;

    // Read/ack activity on the 'all' announcement: two reads, one acknowledged.
    insert_read(&mut sa, ann_all, resident_a1, false).await;
    insert_read(&mut sa, ann_all, resident_a2, true).await;
    drop(sa);

    let repo = AnnouncementRepository::new(pool.clone());
    let mut conn = pool.acquire().await.unwrap();
    // Manager/admin analytics read — super-admin context so the aggregate sees
    // the true row set rather than an RLS-filtered slice.
    set_ctx(&mut conn, Some(org_a), None, true).await;

    // ---- 'all' scope: every active member of org A (3), plus read/ack. ----
    let m_all = repo
        .get_fanout_metrics_rls(&mut *conn, ann_all)
        .await
        .expect("fanout metrics query")
        .expect("announcement exists");
    assert_eq!(m_all.scope, "all");
    assert_eq!(
        m_all.delivered, 3,
        "'all' delivers to every active org-A member (manager + 2 residents)"
    );
    assert_eq!(m_all.read, 2, "two users have read records");
    assert_eq!(m_all.acknowledged, 1, "one of the reads is acknowledged");

    // ---- 'building' scope: residents of building A only; org B excluded. ----
    let m_building = repo
        .get_fanout_metrics_rls(&mut *conn, ann_building)
        .await
        .expect("fanout metrics query")
        .expect("announcement exists");
    assert_eq!(m_building.scope, "building");
    assert_eq!(
        m_building.delivered, 2,
        "building A has two resident units; org-B building id in target_ids must \
         contribute nobody (cross-tenant fan-out leak — #2484)"
    );
    assert_eq!(m_building.read, 0);
    assert_eq!(m_building.acknowledged, 0);

    // ---- 'units' scope: resident of unit A1 only; org-B unit excluded. ----
    let m_units = repo
        .get_fanout_metrics_rls(&mut *conn, ann_units)
        .await
        .expect("fanout metrics query")
        .expect("announcement exists");
    assert_eq!(m_units.scope, "units");
    assert_eq!(
        m_units.delivered, 1,
        "only unit A1's resident is in scope; org-B unit id must be excluded"
    );

    // ---- 'roles' scope: members whose role matches (the manager). ----
    let m_roles = repo
        .get_fanout_metrics_rls(&mut *conn, ann_roles)
        .await
        .expect("fanout metrics query")
        .expect("announcement exists");
    assert_eq!(m_roles.scope, "roles");
    assert_eq!(
        m_roles.delivered, 1,
        "one org-A member holds the 'manager' role"
    );

    // ---- unknown / missing announcement returns None. ----
    let missing = repo
        .get_fanout_metrics_rls(&mut *conn, Uuid::new_v4())
        .await
        .expect("fanout metrics query");
    assert!(missing.is_none(), "unknown announcement id yields None");
}
