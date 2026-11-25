//! Resident person-month submissions awaiting manager approval (UC-10).
//!
//! A current resident proposes a count for their own unit; a manager lists,
//! approves (the count lands in `person_months`) or rejects it. Residents of
//! other units and non-managers are refused.

use axum::http::StatusCode;
use chrono::Datelike;
use common::tenant::TenantRole;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{mint_tenant_token, seed_membership, seed_org, TestApp};

async fn seed_user(pool: &PgPool, label: &str) -> (Uuid, String) {
    let email = format!("{label}-{}@pm-sub.test", Uuid::new_v4());
    let id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (email, password_hash, name) VALUES ($1, 'x', $2) RETURNING id",
    )
    .bind(&email)
    .bind(label)
    .fetch_one(pool)
    .await
    .expect("seed user");
    (id, email)
}

struct Fixture {
    org: Uuid,
    building: Uuid,
    unit: Uuid,
    other_unit: Uuid,
    resident: (Uuid, String),
    manager: (Uuid, String),
}

async fn fixture(pool: &PgPool) -> Fixture {
    let org = seed_org(pool, "pm-sub").await;
    let building = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, street, city, postal_code, country)
           VALUES ($1, 'Sub St 1', 'Bratislava', '81101', 'SK') RETURNING id"#,
    )
    .bind(org)
    .fetch_one(pool)
    .await
    .expect("building");
    let mut units = vec![];
    for d in ["1A", "1B"] {
        units.push(
            sqlx::query_scalar::<_, Uuid>(
                "INSERT INTO units (building_id, designation) VALUES ($1, $2) RETURNING id",
            )
            .bind(building)
            .bind(d)
            .fetch_one(pool)
            .await
            .expect("unit"),
        );
    }
    let resident = seed_user(pool, "resident").await;
    let manager = seed_user(pool, "manager").await;
    seed_membership(pool, org, resident.0, "resident").await;
    seed_membership(pool, org, manager.0, "manager").await;
    sqlx::query(
        "INSERT INTO unit_residents (unit_id, user_id, resident_type) VALUES ($1, $2, 'tenant')",
    )
    .bind(units[0])
    .bind(resident.0)
    .execute(pool)
    .await
    .expect("residency");
    Fixture {
        org,
        building,
        unit: units[0],
        other_unit: units[1],
        resident,
        manager,
    }
}

fn this_month() -> (i32, i32) {
    let today = chrono::Utc::now().date_naive();
    (today.year(), today.month() as i32)
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resident_submits_and_manager_approves(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let f = fixture(&pool).await;
    let resident_token =
        mint_tenant_token(f.resident.0, &f.resident.1, f.org, TenantRole::Resident);
    let manager_token = mint_tenant_token(f.manager.0, &f.manager.1, f.org, TenantRole::Manager);
    let (year, month) = this_month();
    let unit_base = format!(
        "/api/v1/buildings/{}/units/{}/person-months/submissions",
        f.building, f.unit
    );

    // Resident submits; a second submission replaces the pending one.
    for count in [2, 3] {
        let resp = app
            .execute(
                app.post(&unit_base)
                    .bearer(&resident_token)
                    .tenant(f.org)
                    .json(json!({ "year": year, "month": month, "count": count, "note": "baby" }))
                    .build(),
            )
            .await;
        assert_eq!(resp.status, StatusCode::CREATED, "{}", resp.text());
    }
    let resp = app
        .execute(
            app.get(&format!("{unit_base}?year={year}"))
                .bearer(&resident_token)
                .tenant(f.org)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
    let list = resp.json_value();
    let items = list.as_array().expect("array");
    assert_eq!(items.len(), 1, "pending submission is replaced: {list}");
    assert_eq!(items[0]["count"], 3);
    assert_eq!(items[0]["status"], "pending");
    let submission_id = items[0]["id"].as_str().unwrap().to_string();

    // Resident cannot see the review list or approve.
    let review = format!("/api/v1/buildings/{}/person-months/submissions", f.building);
    let resp = app
        .execute(
            app.get(&review)
                .bearer(&resident_token)
                .tenant(f.org)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::FORBIDDEN, "{}", resp.text());
    let approve = format!("{review}/{submission_id}/approve");
    let resp = app
        .execute(
            app.post(&approve)
                .bearer(&resident_token)
                .tenant(f.org)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::FORBIDDEN, "{}", resp.text());

    // Manager sees it and approves; the count lands in person_months.
    let resp = app
        .execute(
            app.get(&review)
                .bearer(&manager_token)
                .tenant(f.org)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
    let pending = resp.json_value();
    assert_eq!(pending[0]["unit_designation"], "1A", "{pending}");
    assert!(pending[0]["submitter_email"]
        .as_str()
        .unwrap_or("")
        .starts_with("resident-"));

    let resp = app
        .execute(
            app.post(&approve)
                .bearer(&manager_token)
                .tenant(f.org)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
    assert_eq!(resp.json_value()["status"], "approved");

    let recorded: i32 = sqlx::query_scalar(
        "SELECT count FROM person_months WHERE unit_id = $1 AND year = $2 AND month = $3",
    )
    .bind(f.unit)
    .bind(year)
    .bind(month)
    .fetch_one(&pool)
    .await
    .expect("person_months row");
    assert_eq!(recorded, 3);

    // A decision applies once.
    let resp = app
        .execute(
            app.post(&approve)
                .bearer(&manager_token)
                .tenant(f.org)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::CONFLICT, "{}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn manager_rejects_and_non_residents_are_refused(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let f = fixture(&pool).await;
    let resident_token =
        mint_tenant_token(f.resident.0, &f.resident.1, f.org, TenantRole::Resident);
    let manager_token = mint_tenant_token(f.manager.0, &f.manager.1, f.org, TenantRole::Manager);
    let (year, month) = this_month();

    // Not a resident of the other unit.
    let other = format!(
        "/api/v1/buildings/{}/units/{}/person-months/submissions",
        f.building, f.other_unit
    );
    let resp = app
        .execute(
            app.post(&other)
                .bearer(&resident_token)
                .tenant(f.org)
                .json(json!({ "year": year, "month": month, "count": 1 }))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::FORBIDDEN, "{}", resp.text());

    // Future month and out-of-range count are rejected.
    let own = format!(
        "/api/v1/buildings/{}/units/{}/person-months/submissions",
        f.building, f.unit
    );
    for body in [
        json!({ "year": year + 1, "month": month, "count": 1 }),
        json!({ "year": year, "month": month, "count": 99 }),
    ] {
        let resp = app
            .execute(
                app.post(&own)
                    .bearer(&resident_token)
                    .tenant(f.org)
                    .json(body)
                    .build(),
            )
            .await;
        assert_eq!(resp.status, StatusCode::BAD_REQUEST, "{}", resp.text());
    }

    let resp = app
        .execute(
            app.post(&own)
                .bearer(&resident_token)
                .tenant(f.org)
                .json(json!({ "year": year, "month": month, "count": 4 }))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::CREATED, "{}", resp.text());
    let id = resp.json_value()["id"].as_str().unwrap().to_string();

    let reject = format!(
        "/api/v1/buildings/{}/person-months/submissions/{id}/reject",
        f.building
    );
    let resp = app
        .execute(
            app.post(&reject)
                .bearer(&manager_token)
                .tenant(f.org)
                .json(json!({ "note": "Only 2 people registered" }))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
    let body = resp.json_value();
    assert_eq!(body["status"], "rejected");
    assert_eq!(body["decision_note"], "Only 2 people registered");

    let written: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM person_months WHERE unit_id = $1")
        .bind(f.unit)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(written, 0, "a rejected submission writes nothing");
}

/// Seed an independent tenant (org + building + unit + resident) with one
/// pending submission, and return `(org, building, unit, resident)`.
///
/// The submission is inserted under the tenant's own RLS context so it is
/// seeded identically whether or not `person_month_submissions` is FORCE'd.
async fn seed_tenant_with_submission(
    pool: &PgPool,
    label: &str,
    count: i32,
) -> (Uuid, Uuid, Uuid, Uuid) {
    let org = seed_org(pool, label).await;
    let building = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, street, city, postal_code, country)
           VALUES ($1, 'Cross St 1', 'Bratislava', '81101', 'SK') RETURNING id"#,
    )
    .bind(org)
    .fetch_one(pool)
    .await
    .expect("building");
    let unit = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO units (building_id, designation) VALUES ($1, '1A') RETURNING id",
    )
    .bind(building)
    .fetch_one(pool)
    .await
    .expect("unit");
    let (resident, _email) = seed_user(pool, label).await;
    seed_membership(pool, org, resident, "resident").await;
    sqlx::query(
        "INSERT INTO unit_residents (unit_id, user_id, resident_type) VALUES ($1, $2, 'tenant')",
    )
    .bind(unit)
    .bind(resident)
    .execute(pool)
    .await
    .expect("residency");

    let (year, month) = this_month();
    let mut conn = pool.acquire().await.expect("acquire seed conn");
    db::set_request_context(&mut *conn, Some(org), Some(resident), false)
        .await
        .expect("set seed ctx");
    sqlx::query(
        r#"INSERT INTO person_month_submissions (unit_id, year, month, count, submitted_by)
           VALUES ($1, $2, $3, $4, $5)"#,
    )
    .bind(unit)
    .bind(year)
    .bind(month)
    .bind(count)
    .bind(resident)
    .execute(&mut *conn)
    .await
    .expect("seed submission");
    db::clear_request_context(&mut *conn)
        .await
        .expect("clear seed ctx");

    (org, building, unit, resident)
}

/// Cross-tenant isolation must hold at the database, not only in the handler
/// layer: `person_month_submissions` is `FORCE ROW LEVEL SECURITY` (migration
/// 00251), so even the api-server's table-owner connection is subject to the
/// `person_month_submissions_org` policy. An org-B RLS context must never see
/// an org-A submission row.
///
/// Regression guard: before FORCE (ENABLE only), the owner connection bypassed
/// RLS and saw every tenant's rows — this test would then observe two rows
/// under org B and fail.
///
/// Why this test switches roles
/// ----------------------------
/// `#[sqlx::test]` connects as the Postgres SUPERUSER, which bypasses RLS
/// entirely — even `FORCE ROW LEVEL SECURITY` does NOT apply to a superuser or
/// the table owner. To exercise the policy the assertions run under a plain
/// `NOSUPERUSER NOBYPASSRLS` role reached with `SET ROLE`, mirroring the
/// established pattern in `esg_force_rls_cross_tenant_tests.rs` and
/// `payment_webhook_dead_letter_rls_manager_tests.rs`. The `app.current_org_id`
/// GUC is set with `set_request_context` (which survives `SET ROLE`) while
/// still the superuser, then the role is assumed for the read.
///
/// The role needs SELECT on `person_month_submissions` plus the tables the
/// `person_month_submissions_org` policy's EXISTS subquery reads (`units`,
/// `buildings`), and EXECUTE on the RLS helper functions those tables'
/// tenant-isolation policies call.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn force_rls_hides_other_tenant_submissions(pool: PgPool) {
    let (org_a, _building_a, unit_a, _res_a) =
        seed_tenant_with_submission(&pool, "cross-a", 2).await;
    let (org_b, _building_b, unit_b, _res_b) =
        seed_tenant_with_submission(&pool, "cross-b", 5).await;

    // --- Create a NOSUPERUSER NOBYPASSRLS role so FORCE actually binds. ---
    let role = format!("pm_sub_rls_test_{}", &Uuid::new_v4().to_string()[..8]);
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"
    )))
    .execute(&pool)
    .await
    .expect("create test role");
    for stmt in [
        format!("GRANT SELECT ON person_month_submissions TO \"{role}\""),
        format!("GRANT SELECT ON units TO \"{role}\""),
        format!("GRANT SELECT ON buildings TO \"{role}\""),
        format!("GRANT SELECT ON organizations TO \"{role}\""),
        format!(
            "GRANT EXECUTE ON FUNCTION get_current_org_id(), is_super_admin(), \
             get_current_org_not_deleted() TO \"{role}\""
        ),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(stmt))
            .execute(&pool)
            .await
            .expect("grant to test role");
    }

    // All role-bound work runs on ONE dedicated connection: the context GUC and
    // SET ROLE are session state that must stay together.
    let mut conn = pool.acquire().await.expect("acquire");

    // Open org B's RLS context (as superuser), then assume the unprivileged
    // role so FORCE RLS applies, and read the submissions table directly.
    db::set_request_context(&mut *conn, Some(org_b), None, false)
        .await
        .expect("set ctx B");
    sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
        .execute(&mut *conn)
        .await
        .expect("set role B");
    let visible: Vec<(Uuid, Uuid)> =
        sqlx::query_as("SELECT id, unit_id FROM person_month_submissions ORDER BY submitted_at")
            .fetch_all(&mut *conn)
            .await
            .expect("select under org B");
    sqlx::query("RESET ROLE")
        .execute(&mut *conn)
        .await
        .expect("reset role B");

    assert_eq!(
        visible.len(),
        1,
        "org B must see exactly its own submission, not org A's: {visible:?}"
    );
    assert_eq!(visible[0].1, unit_b, "the visible row is org B's unit");
    assert!(
        !visible.iter().any(|(_, u)| *u == unit_a),
        "org A's submission must be invisible to an org B context"
    );

    // Isolation is symmetric: org A's context sees exactly org A's row.
    db::set_request_context(&mut *conn, Some(org_a), None, false)
        .await
        .expect("set ctx A");
    sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
        .execute(&mut *conn)
        .await
        .expect("set role A");
    let visible_a: Vec<Uuid> = sqlx::query_scalar("SELECT unit_id FROM person_month_submissions")
        .fetch_all(&mut *conn)
        .await
        .expect("select under org A");
    sqlx::query("RESET ROLE")
        .execute(&mut *conn)
        .await
        .expect("reset role A");
    db::clear_request_context(&mut *conn)
        .await
        .expect("clear ctx A");
    assert_eq!(
        visible_a,
        vec![unit_a],
        "org A must see only its own submission"
    );

    // Release the dedicated connection before dropping the role so no session
    // still holds it.
    drop(conn);

    // --- Cleanup: DROP OWNED severs every grant (incl. function EXECUTE that a
    // plain REVOKE misses) so DROP ROLE does not fail with dependent objects.
    for stmt in [
        format!("DROP OWNED BY \"{role}\""),
        format!("DROP ROLE IF EXISTS \"{role}\""),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(stmt))
            .execute(&pool)
            .await
            .ok();
    }
}
