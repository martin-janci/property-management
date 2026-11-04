//! Round-4 authz regressions for the fault workflow (audit4-operations).
//!
//! * `update_fault` / `triage_fault` / `update_status` / `get_ai_suggestion`
//!   loaded the fault with `find_by_id_rls` (WHERE id = $1 only, RLS not
//!   FORCEd), so a manager of org B could mutate org A's fault by id → now 404.
//! * `delete_attachment` deleted by attachment id only → a foreign attachment
//!   id under the caller's own fault is now 404 and the row survives; a
//!   resident cannot delete someone else's attachment (403).
//! * `confirm_fault` / `reopen_fault` are reporter-or-manager only, and
//!   `add_work_note` is assignee-or-manager only (403 for other residents).

#![allow(dead_code)]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{create_authenticated_user, seed_org, TestApp, TestUser};

fn inject_tenant(mut req: Request<Body>, org_id: Uuid) -> Request<Body> {
    use api_core::middleware::host_tenant::{ResolvedTenant, TenantSource};
    req.extensions_mut().insert(ResolvedTenant {
        organization_id: org_id,
        source: TenantSource::Subdomain,
    });
    req
}

/// Register a user and give them `role_type` (organization_members) plus the
/// matching `user_memberships` role in `org_id`. Returns (token, user_id).
async fn member(app: &TestApp, org_id: Uuid, role_type: &str, um_role: &str) -> (String, Uuid) {
    let user = TestUser::new();
    let (token, _) = create_authenticated_user(app, &user).await;
    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .unwrap();
    crate::common::seed_membership(&app.pool, org_id, user_id, role_type).await;
    sqlx::query(
        "INSERT INTO user_memberships (user_id, organization_id, role) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
    )
    .bind(user_id)
    .bind(org_id)
    .bind(um_role)
    .execute(&app.pool)
    .await
    .unwrap();
    (token, user_id)
}

async fn manager(app: &TestApp, org_id: Uuid) -> (String, Uuid) {
    member(app, org_id, "org_admin", "OrgAdmin").await
}

async fn resident(app: &TestApp, org_id: Uuid) -> (String, Uuid) {
    member(app, org_id, "resident", "Resident").await
}

async fn seed_fault(pool: &PgPool, org_id: Uuid, reporter_id: Uuid, status: &str) -> Uuid {
    let building_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, name, street, city, postal_code, country)
           VALUES ($1, 'R4 Fault Building', 'St 1', 'Bratislava', '81101', 'Slovakia')
           RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO faults (
               organization_id, building_id, reporter_id,
               title, description, location_description, category, priority, status
           )
           VALUES ($1, $2, $3, 'R4 Fault', 'desc', 'Basement',
                   'plumbing'::fault_category, 'medium'::fault_priority,
                   $4::text::fault_status)
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(building_id)
    .bind(reporter_id)
    .bind(status)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn seed_attachment(pool: &PgPool, fault_id: Uuid, uploaded_by: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO fault_attachments
               (fault_id, filename, original_filename, content_type, size_bytes, storage_url, uploaded_by)
           VALUES ($1, 'f.jpg', 'f.jpg', 'image/jpeg', 10, 'https://s3/f.jpg', $2)
           RETURNING id"#,
    )
    .bind(fault_id)
    .bind(uploaded_by)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn attachment_exists(pool: &PgPool, id: Uuid) -> bool {
    sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM fault_attachments WHERE id = $1)")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn cross_org_manager_cannot_mutate_foreign_fault(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let org_a = seed_org(&app.pool, "r4fa").await;
    let org_b = seed_org(&app.pool, "r4fb").await;
    let (_ta, user_a) = manager(&app, org_a).await;
    let (token_b, _) = manager(&app, org_b).await;
    let fault_a = seed_fault(&app.pool, org_a, user_a, "new").await;

    let cases = vec![
        app.put(&format!("/api/v1/faults/{fault_a}"))
            .json(json!({ "title": "pwned" })),
        app.post(&format!("/api/v1/faults/{fault_a}/triage"))
            .json(json!({ "priority": "high" })),
        app.put(&format!("/api/v1/faults/{fault_a}/status"))
            .json(json!({ "status": "in_progress" })),
        app.post(&format!("/api/v1/faults/{fault_a}/suggest")),
    ];
    for rb in cases {
        let req = inject_tenant(rb.bearer(&token_b).tenant(org_b).build(), org_b);
        let uri = req.uri().to_string();
        let resp = app.execute(req).await;
        assert_eq!(
            resp.status,
            StatusCode::NOT_FOUND,
            "{uri} must 404 cross-org: {}",
            resp.text()
        );
    }

    let title: String = sqlx::query_scalar("SELECT title FROM faults WHERE id = $1")
        .bind(fault_a)
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(title, "R4 Fault");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn owner_org_manager_can_update_fault(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let org_a = seed_org(&app.pool, "r4fo").await;
    let (token_a, _) = manager(&app, org_a).await;
    let (_tr, reporter) = resident(&app, org_a).await;
    let fault_a = seed_fault(&app.pool, org_a, reporter, "new").await;

    let req = app
        .put(&format!("/api/v1/faults/{fault_a}"))
        .json(json!({ "title": "Updated by manager" }))
        .bearer(&token_a)
        .tenant(org_a)
        .build();
    let resp = app.execute(inject_tenant(req, org_a)).await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_attachment_is_scoped_to_parent_fault(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let org_a = seed_org(&app.pool, "r4aa").await;
    let org_b = seed_org(&app.pool, "r4ab").await;
    let (_ta, user_a) = manager(&app, org_a).await;
    let (token_b, user_b) = manager(&app, org_b).await;
    let fault_a = seed_fault(&app.pool, org_a, user_a, "new").await;
    let att_a = seed_attachment(&app.pool, fault_a, user_a).await;
    let fault_b = seed_fault(&app.pool, org_b, user_b, "new").await;
    let att_b = seed_attachment(&app.pool, fault_b, user_b).await;

    // Own fault id + foreign attachment id → 404, foreign row survives.
    let req = app
        .delete(&format!("/api/v1/faults/{fault_b}/attachments/{att_a}"))
        .bearer(&token_b)
        .tenant(org_b)
        .build();
    let resp = app.execute(inject_tenant(req, org_b)).await;
    assert_eq!(resp.status, StatusCode::NOT_FOUND, "{}", resp.text());
    assert!(attachment_exists(&app.pool, att_a).await);

    // Owner org manager deletes own attachment → 204.
    let req = app
        .delete(&format!("/api/v1/faults/{fault_b}/attachments/{att_b}"))
        .bearer(&token_b)
        .tenant(org_b)
        .build();
    let resp = app.execute(inject_tenant(req, org_b)).await;
    assert_eq!(resp.status, StatusCode::NO_CONTENT, "{}", resp.text());
    assert!(!attachment_exists(&app.pool, att_b).await);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resident_cannot_delete_others_attachment(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let org = seed_org(&app.pool, "r4ar").await;
    let (_tm, mgr) = manager(&app, org).await;
    let (token_r, _) = resident(&app, org).await;
    let fault = seed_fault(&app.pool, org, mgr, "new").await;
    let att = seed_attachment(&app.pool, fault, mgr).await;

    let req = app
        .delete(&format!("/api/v1/faults/{fault}/attachments/{att}"))
        .bearer(&token_r)
        .tenant(org)
        .build();
    let resp = app.execute(inject_tenant(req, org)).await;
    assert_eq!(resp.status, StatusCode::FORBIDDEN, "{}", resp.text());
    assert!(attachment_exists(&app.pool, att).await);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn non_reporter_resident_cannot_confirm_reopen_or_add_work_note(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let org = seed_org(&app.pool, "r4cr").await;
    let (_t1, reporter) = resident(&app, org).await;
    let (token_other, _) = resident(&app, org).await;
    let fault = seed_fault(&app.pool, org, reporter, "resolved").await;

    let cases = vec![
        app.post(&format!("/api/v1/faults/{fault}/confirm"))
            .json(json!({ "rating": 1 })),
        app.post(&format!("/api/v1/faults/{fault}/reopen"))
            .json(json!({ "reason": "not mine" })),
        app.post(&format!("/api/v1/faults/{fault}/work-notes"))
            .json(json!({ "note": "fake note" })),
    ];
    for rb in cases {
        let req = inject_tenant(rb.bearer(&token_other).tenant(org).build(), org);
        let uri = req.uri().to_string();
        let resp = app.execute(req).await;
        assert_eq!(
            resp.status,
            StatusCode::FORBIDDEN,
            "{uri} must 403 for a non-reporter resident: {}",
            resp.text()
        );
    }
    let status: String = sqlx::query_scalar("SELECT status::text FROM faults WHERE id = $1")
        .bind(fault)
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(status, "resolved");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn reporter_can_confirm_own_fault(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let org = seed_org(&app.pool, "r4cf").await;
    let (token_r, reporter) = resident(&app, org).await;
    let fault = seed_fault(&app.pool, org, reporter, "resolved").await;

    let req = app
        .post(&format!("/api/v1/faults/{fault}/confirm"))
        .json(json!({ "rating": 5 }))
        .bearer(&token_r)
        .tenant(org)
        .build();
    let resp = app.execute(inject_tenant(req, org)).await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
}
