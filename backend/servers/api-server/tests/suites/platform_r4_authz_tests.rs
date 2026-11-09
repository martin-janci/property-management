//! Round-4 platform authorization tests.
//!
//! Every caller here holds a *login-format* access token (no `role` /
//! `tenant_id` claim — exactly what real users get), so each decision must come
//! from the database: membership + `organization_members.role_type`.
//!
//! Per handler group: another org's ids answer 404, a tenant/resident gets 403
//! on manager-only mutations, and the owning org's manager succeeds.

#![allow(dead_code)]

use api_server::services::JwtService;
use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{seed_membership, seed_org, TestApp, TestConfig, TestResponse};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

struct Actor {
    id: Uuid,
    token: String,
}

async fn seed_user(pool: &PgPool, label: &str) -> (Uuid, String) {
    let email = format!("r4-{label}-{}@r4-platform.test", Uuid::new_v4());
    let id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'hash', 'R4 Platform User', 'active', NOW())
           RETURNING id"#,
    )
    .bind(&email)
    .fetch_one(pool)
    .await
    .expect("seed user");
    (id, email)
}

/// Login-format token: `org_id = None`, `roles = None`.
fn login_token(user_id: Uuid, email: &str) -> String {
    let jwt = JwtService::new(&TestConfig::default().jwt_secret).expect("jwt service");
    jwt.generate_access_token(user_id, email, "R4 Platform User", None, None)
        .expect("mint token")
}

/// A user who is an active member of `org` with `role_type = role`.
async fn member(pool: &PgPool, org: Uuid, role: &str) -> Actor {
    let (id, email) = seed_user(pool, role).await;
    seed_membership(pool, org, id, role).await;
    Actor {
        id,
        token: login_token(id, &email),
    }
}

async fn call(
    app: &TestApp,
    method: &str,
    uri: &str,
    actor: &Actor,
    org: Uuid,
    body: Option<serde_json::Value>,
) -> TestResponse {
    let builder = match method {
        "GET" => app.get(uri),
        "POST" => app.post(uri),
        "PUT" => app.put(uri),
        "DELETE" => app.delete(uri),
        other => panic!("unsupported method {other}"),
    };
    let mut builder = builder.bearer(&actor.token).tenant(org);
    if let Some(b) = body {
        builder = builder.json(b);
    }
    app.execute(builder.build()).await
}

fn expect(resp: &TestResponse, status: StatusCode, ctx: &str) {
    assert_eq!(resp.status, status, "{ctx}: body={}", resp.text());
}

// ---------------------------------------------------------------------------
// Government portal seeds
// ---------------------------------------------------------------------------

async fn gp_connection(pool: &PgPool, org: Uuid, user: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO government_portal_connections
               (organization_id, portal_type, portal_name, country_code, created_by)
           VALUES ($1, 'tax_authority'::government_portal_type, 'Tax Portal', 'SK', $2)
           RETURNING id"#,
    )
    .bind(org)
    .bind(user)
    .fetch_one(pool)
    .await
    .expect("seed connection")
}

async fn gp_template(pool: &PgPool) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO regulatory_report_templates
               (template_code, template_name, portal_type, country_code,
                schema_version, field_mappings, validation_rules, effective_from)
           VALUES ('R4_SK_VAT', 'R4 SK VAT', 'tax_authority'::government_portal_type, 'SK',
                   '1.0', '{}', '[]', '2024-01-01')
           ON CONFLICT (template_code) DO UPDATE SET template_name = EXCLUDED.template_name
           RETURNING id"#,
    )
    .fetch_one(pool)
    .await
    .expect("seed template")
}

async fn gp_submission(pool: &PgPool, org: Uuid, user: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO regulatory_submissions
               (organization_id, report_type, report_period_start, report_period_end,
                report_data, submission_reference, prepared_by)
           VALUES ($1, 'VAT', '2024-01-01', '2024-03-31', '{"v": "original"}', $3, $2)
           RETURNING id"#,
    )
    .bind(org)
    .bind(user)
    .bind(format!("REF-{}", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed submission")
}

async fn gp_attachment(pool: &PgPool, submission: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO regulatory_submission_attachments
               (submission_id, file_name, file_type, file_size, file_url, attachment_type)
           VALUES ($1, 'r.pdf', 'application/pdf', 10, 'https://s3.example.com/r.pdf', 'supporting_document')
           RETURNING id"#,
    )
    .bind(submission)
    .fetch_one(pool)
    .await
    .expect("seed attachment")
}

async fn gp_schedule(
    pool: &PgPool,
    org: Uuid,
    connection: Uuid,
    template: Uuid,
    user: Uuid,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO regulatory_submission_schedules
               (organization_id, portal_connection_id, template_id, created_by)
           VALUES ($1, $2, $3, $4)
           RETURNING id"#,
    )
    .bind(org)
    .bind(connection)
    .bind(template)
    .bind(user)
    .fetch_one(pool)
    .await
    .expect("seed schedule")
}

async fn count(pool: &PgPool, sql: &'static str, id: Uuid) -> i64 {
    sqlx::query_scalar::<_, i64>(sql)
        .bind(id)
        .fetch_one(pool)
        .await
        .expect("count")
}

fn attachment_body() -> serde_json::Value {
    json!({
        "fileName": "x.pdf",
        "fileType": "application/pdf",
        "fileSize": 1,
        "fileUrl": "https://s3.example.com/x.pdf",
        "attachmentType": "supporting_document"
    })
}

// ---------------------------------------------------------------------------
// Government portal
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn gov_portal_other_org_ids_are_404_and_untouched(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "gp-a").await;
    let org_b = seed_org(&pool, "gp-b").await;
    let mgr_a = member(&pool, org_a, "manager").await;
    let mgr_b = member(&pool, org_b, "manager").await;

    let sub_a = gp_submission(&pool, org_a, mgr_a.id).await;
    let sub_b = gp_submission(&pool, org_b, mgr_b.id).await;
    let att_b = gp_attachment(&pool, sub_b).await;
    let conn_b = gp_connection(&pool, org_b, mgr_b.id).await;
    let tpl = gp_template(&pool).await;
    let sched_b = gp_schedule(&pool, org_b, conn_b, tpl, mgr_b.id).await;

    let base = "/api/v1/government-portal";
    let cases: Vec<(&str, String, Option<serde_json::Value>)> = vec![
        ("GET", format!("{base}/submissions/{sub_b}"), None),
        ("GET", format!("{base}/submissions/{sub_b}/audit"), None),
        (
            "GET",
            format!("{base}/submissions/{sub_b}/attachments"),
            None,
        ),
        (
            "PUT",
            format!("{base}/submissions/{sub_b}"),
            Some(json!({ "reportData": { "v": "hijacked" } })),
        ),
        ("POST", format!("{base}/submissions/{sub_b}/validate"), None),
        ("POST", format!("{base}/submissions/{sub_b}/submit"), None),
        ("POST", format!("{base}/submissions/{sub_b}/cancel"), None),
        (
            "POST",
            format!("{base}/submissions/{sub_b}/attachments"),
            Some(attachment_body()),
        ),
        (
            "DELETE",
            format!("{base}/submissions/{sub_b}/attachments/{att_b}"),
            None,
        ),
        // Own submission in the path, foreign attachment id.
        (
            "DELETE",
            format!("{base}/submissions/{sub_a}/attachments/{att_b}"),
            None,
        ),
        (
            "PUT",
            format!("{base}/schedules/{sched_b}"),
            Some(json!({ "autoSubmit": true })),
        ),
        ("DELETE", format!("{base}/schedules/{sched_b}"), None),
        ("GET", format!("{base}/connections/{conn_b}"), None),
        ("DELETE", format!("{base}/connections/{conn_b}"), None),
    ];
    for (method, uri, body) in cases {
        let resp = call(&app, method, &uri, &mgr_a, org_a, body).await;
        expect(&resp, StatusCode::NOT_FOUND, &format!("{method} {uri}"));
    }

    // A foreign portal connection cannot be referenced from a new submission.
    let resp = call(
        &app,
        "POST",
        &format!("{base}/submissions"),
        &mgr_a,
        org_a,
        Some(json!({
            "portalConnectionId": conn_b,
            "reportType": "VAT",
            "reportPeriodStart": "2024-01-01",
            "reportPeriodEnd": "2024-03-31",
            "reportData": {}
        })),
    )
    .await;
    expect(
        &resp,
        StatusCode::NOT_FOUND,
        "create submission with foreign connection",
    );

    // Org B's data is untouched.
    let (status, data): (String, serde_json::Value) = sqlx::query_as(
        "SELECT status::text, report_data FROM regulatory_submissions WHERE id = $1",
    )
    .bind(sub_b)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(status, "draft");
    assert_eq!(data["v"], "original");
    assert_eq!(
        count(
            &pool,
            "SELECT COUNT(*) FROM regulatory_submission_attachments WHERE submission_id = $1",
            sub_b
        )
        .await,
        1
    );
    assert_eq!(
        count(
            &pool,
            "SELECT COUNT(*) FROM regulatory_submission_schedules WHERE id = $1 AND auto_submit = false",
            sched_b
        )
        .await,
        1
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn gov_portal_x_tenant_id_of_foreign_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "gp-hdr-a").await;
    let org_b = seed_org(&pool, "gp-hdr-b").await;
    let mgr_a = member(&pool, org_a, "manager").await;

    // Plain TenantExtractor trusted the header; membership is now verified.
    let resp = call(
        &app,
        "GET",
        "/api/v1/government-portal/connections",
        &mgr_a,
        org_b,
        None,
    )
    .await;
    expect(
        &resp,
        StatusCode::FORBIDDEN,
        "list connections of foreign org",
    );

    let resp = call(
        &app,
        "POST",
        "/api/v1/government-portal/connections",
        &mgr_a,
        org_b,
        Some(json!({ "portalType": "tax_authority", "portalName": "Evil" })),
    )
    .await;
    expect(
        &resp,
        StatusCode::FORBIDDEN,
        "create connection in foreign org",
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn gov_portal_resident_cannot_mutate(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "gp-res").await;
    let mgr = member(&pool, org, "manager").await;
    let resident = member(&pool, org, "resident").await;
    let sub = gp_submission(&pool, org, mgr.id).await;
    let att = gp_attachment(&pool, sub).await;
    let conn = gp_connection(&pool, org, mgr.id).await;
    let tpl = gp_template(&pool).await;
    let sched = gp_schedule(&pool, org, conn, tpl, mgr.id).await;

    let base = "/api/v1/government-portal";
    let cases: Vec<(&str, String, Option<serde_json::Value>)> = vec![
        (
            "POST",
            format!("{base}/connections"),
            Some(json!({ "portalType": "tax_authority", "portalName": "X" })),
        ),
        ("DELETE", format!("{base}/connections/{conn}"), None),
        ("POST", format!("{base}/connections/{conn}/test"), None),
        (
            "POST",
            format!("{base}/submissions"),
            Some(json!({
                "reportType": "VAT",
                "reportPeriodStart": "2024-01-01",
                "reportPeriodEnd": "2024-03-31",
                "reportData": {}
            })),
        ),
        (
            "PUT",
            format!("{base}/submissions/{sub}"),
            Some(json!({ "reportData": { "v": "resident" } })),
        ),
        ("POST", format!("{base}/submissions/{sub}/validate"), None),
        ("POST", format!("{base}/submissions/{sub}/cancel"), None),
        (
            "POST",
            format!("{base}/submissions/{sub}/attachments"),
            Some(attachment_body()),
        ),
        (
            "DELETE",
            format!("{base}/submissions/{sub}/attachments/{att}"),
            None,
        ),
        (
            "POST",
            format!("{base}/schedules"),
            Some(json!({ "portalConnectionId": conn, "templateId": tpl })),
        ),
        (
            "PUT",
            format!("{base}/schedules/{sched}"),
            Some(json!({ "autoSubmit": true })),
        ),
        ("DELETE", format!("{base}/schedules/{sched}"), None),
    ];
    for (method, uri, body) in cases {
        let resp = call(&app, method, &uri, &resident, org, body).await;
        expect(
            &resp,
            StatusCode::FORBIDDEN,
            &format!("resident {method} {uri}"),
        );
    }

    // Members can still read their own org's filings.
    let resp = call(
        &app,
        "GET",
        &format!("{base}/submissions/{sub}"),
        &resident,
        org,
        None,
    )
    .await;
    expect(&resp, StatusCode::OK, "resident reads own-org submission");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn gov_portal_manager_manages_own_org(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "gp-ok").await;
    let mgr = member(&pool, org, "manager").await;
    let sub = gp_submission(&pool, org, mgr.id).await;
    let att = gp_attachment(&pool, sub).await;
    let conn = gp_connection(&pool, org, mgr.id).await;
    let tpl = gp_template(&pool).await;
    let sched = gp_schedule(&pool, org, conn, tpl, mgr.id).await;
    let base = "/api/v1/government-portal";

    let resp = call(
        &app,
        "PUT",
        &format!("{base}/submissions/{sub}"),
        &mgr,
        org,
        Some(json!({ "reportData": { "v": "updated" } })),
    )
    .await;
    expect(&resp, StatusCode::OK, "update own submission");

    let resp = call(
        &app,
        "POST",
        &format!("{base}/submissions/{sub}/validate"),
        &mgr,
        org,
        None,
    )
    .await;
    expect(&resp, StatusCode::OK, "validate own submission");

    let resp = call(
        &app,
        "GET",
        &format!("{base}/submissions/{sub}/audit"),
        &mgr,
        org,
        None,
    )
    .await;
    expect(&resp, StatusCode::OK, "read own audit trail");

    let resp = call(
        &app,
        "DELETE",
        &format!("{base}/submissions/{sub}/attachments/{att}"),
        &mgr,
        org,
        None,
    )
    .await;
    expect(&resp, StatusCode::NO_CONTENT, "delete own attachment");

    let resp = call(
        &app,
        "PUT",
        &format!("{base}/schedules/{sched}"),
        &mgr,
        org,
        Some(json!({ "autoSubmit": true })),
    )
    .await;
    expect(&resp, StatusCode::OK, "update own schedule");

    let resp = call(
        &app,
        "DELETE",
        &format!("{base}/schedules/{sched}"),
        &mgr,
        org,
        None,
    )
    .await;
    expect(&resp, StatusCode::NO_CONTENT, "delete own schedule");
}

// ---------------------------------------------------------------------------
// Organization members: role_type escalation
// ---------------------------------------------------------------------------

async fn wildcard_role(pool: &PgPool, org: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO roles (organization_id, name, permissions, is_system)
           VALUES ($1, $2, '["*"]'::jsonb, FALSE) RETURNING id"#,
    )
    .bind(org)
    .bind(format!("R4 Admin {}", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed role")
}

/// Member with `role_type` and a wildcard-permission role row.
async fn permissioned_member(pool: &PgPool, org: Uuid, role_type: &str, role_id: Uuid) -> Actor {
    let (id, email) = seed_user(pool, role_type).await;
    sqlx::query(
        r#"INSERT INTO organization_members (organization_id, user_id, role_id, role_type, status)
           VALUES ($1, $2, $3, $4, 'active')"#,
    )
    .bind(org)
    .bind(id)
    .bind(role_id)
    .bind(role_type)
    .execute(pool)
    .await
    .expect("seed member");
    Actor {
        id,
        token: login_token(id, &email),
    }
}

async fn role_type_of(pool: &PgPool, org: Uuid, user: Uuid) -> String {
    sqlx::query_scalar::<_, String>(
        "SELECT role_type FROM organization_members WHERE organization_id = $1 AND user_id = $2",
    )
    .bind(org)
    .bind(user)
    .fetch_one(pool)
    .await
    .expect("role_type")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn members_cannot_escalate_role_type(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "mem-esc").await;
    let role = wildcard_role(&pool, org).await;
    let mgr = permissioned_member(&pool, org, "manager", role).await;
    let target = permissioned_member(&pool, org, "resident", role).await;
    let admin = permissioned_member(&pool, org, "org_admin", role).await;

    // Own role: rejected even to a "lower" value.
    for rt in ["super_admin", "org_admin", "manager"] {
        let resp = call(
            &app,
            "PUT",
            &format!("/api/v1/organizations/{org}/members/{}", mgr.id),
            &mgr,
            org,
            Some(json!({ "role_id": role, "role_type": rt })),
        )
        .await;
        expect(
            &resp,
            StatusCode::FORBIDDEN,
            &format!("self role change to {rt}"),
        );
    }
    assert_eq!(role_type_of(&pool, org, mgr.id).await, "manager");

    // Above own level, or a platform role: rejected.
    for rt in ["super_admin", "SuperAdmin", "platform_admin", "org_admin"] {
        let resp = call(
            &app,
            "PUT",
            &format!("/api/v1/organizations/{org}/members/{}", target.id),
            &mgr,
            org,
            Some(json!({ "role_id": role, "role_type": rt })),
        )
        .await;
        expect(&resp, StatusCode::FORBIDDEN, &format!("assign {rt}"));
    }
    assert_eq!(role_type_of(&pool, org, target.id).await, "resident");

    // A member who outranks the caller cannot be touched.
    let resp = call(
        &app,
        "PUT",
        &format!("/api/v1/organizations/{org}/members/{}", admin.id),
        &mgr,
        org,
        Some(json!({ "role_id": role, "role_type": "resident" })),
    )
    .await;
    expect(
        &resp,
        StatusCode::FORBIDDEN,
        "demote a higher-ranked member",
    );

    // Adding an accomplice with a platform role is rejected.
    let (newbie, _) = seed_user(&pool, "newbie").await;
    let resp = call(
        &app,
        "POST",
        &format!("/api/v1/organizations/{org}/members"),
        &mgr,
        org,
        Some(json!({ "user_id": newbie, "role_id": role, "role_type": "super_admin" })),
    )
    .await;
    expect(&resp, StatusCode::FORBIDDEN, "add member as super_admin");

    // At or below own level works.
    let resp = call(
        &app,
        "PUT",
        &format!("/api/v1/organizations/{org}/members/{}", target.id),
        &mgr,
        org,
        Some(json!({ "role_id": role, "role_type": "owner" })),
    )
    .await;
    expect(&resp, StatusCode::OK, "assign owner");
    assert_eq!(role_type_of(&pool, org, target.id).await, "owner");

    let resp = call(
        &app,
        "POST",
        &format!("/api/v1/organizations/{org}/members"),
        &mgr,
        org,
        Some(json!({ "user_id": newbie, "role_id": role, "role_type": "tenant" })),
    )
    .await;
    expect(&resp, StatusCode::CREATED, "add member as tenant");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn suspended_admin_cannot_update_organization(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "org-susp").await;
    let other = seed_org(&pool, "org-susp-home").await;
    let role = wildcard_role(&pool, org).await;
    let admin = permissioned_member(&pool, org, "org_admin", role).await;
    sqlx::query(
        "UPDATE organization_members SET status = 'suspended' WHERE organization_id = $1 AND user_id = $2",
    )
    .bind(org)
    .bind(admin.id)
    .execute(&pool)
    .await
    .unwrap();
    // RlsConnection needs an active membership somewhere: the caller's home org.
    seed_membership(&pool, other, admin.id, "resident").await;

    let resp = call(
        &app,
        "PUT",
        &format!("/api/v1/organizations/{org}"),
        &admin,
        other,
        Some(json!({ "name": "Renamed by suspended admin" })),
    )
    .await;
    expect(&resp, StatusCode::FORBIDDEN, "suspended admin updates org");
}

// ---------------------------------------------------------------------------
// RlsConnection handlers: manager gates
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn rls_manager_only_mutations_reject_residents(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "rls-gates").await;
    let resident = member(&pool, org, "resident").await;
    let tenant = member(&pool, org, "tenant").await;
    let id = Uuid::new_v4();

    let cases: Vec<(&str, String, Option<serde_json::Value>)> = vec![
        (
            "PUT",
            "/api/v1/ai/llm/chat/escalation-config".into(),
            Some(json!({
                "confidence_threshold": 0.8,
                "escalation_email": "attacker@example.com",
                "escalation_webhook_url": null,
                "auto_escalate_topics": []
            })),
        ),
        (
            "PUT",
            "/api/v1/ai/sentiment/thresholds".into(),
            Some(json!({ "negative_threshold": 0.99, "positive_threshold": 0.4 })),
        ),
        (
            "POST",
            format!("/api/v1/ai/sentiment/alerts/{id}/acknowledge"),
            None,
        ),
        ("GET", "/api/v1/ai/sentiment/alerts".into(), None),
        ("GET", "/api/v1/ai/sentiment/dashboard".into(), None),
        ("DELETE", format!("/api/v1/ai/workflows/{id}"), None),
        ("DELETE", format!("/api/v1/ai/workflows/actions/{id}"), None),
        (
            "POST",
            format!("/api/v1/esg/metrics/{id}/verify"),
            Some(json!({ "status": "verified" })),
        ),
        ("POST", format!("/api/v1/esg/metrics/{id}/delete"), None),
        ("DELETE", format!("/api/v1/insurance/policies/{id}"), None),
        ("DELETE", format!("/api/v1/insurance/reminders/{id}"), None),
        (
            "POST",
            "/api/v1/data-residency/config".into(),
            Some(json!({
                "primary_region": "eu_central",
                "backup_region": "ch_north",
                "allow_cross_region_access": true,
                "data_type_overrides": [],
                "compliance_notes": null
            })),
        ),
        ("GET", "/api/v1/data-residency/audit".into(), None),
        (
            "POST",
            "/api/v1/data-residency/compliance/verify".into(),
            Some(json!({ "include_details": true })),
        ),
        ("DELETE", format!("/api/v1/ecosystem/webhooks/{id}"), None),
        (
            "POST",
            format!("/api/v1/ecosystem/organizations/{org}/webhooks"),
            Some(json!({
                "name": "exfil",
                "url": "https://attacker.example.com/hook",
                "auth_type": "none",
                "events": ["fault.created"]
            })),
        ),
        (
            "DELETE",
            format!("/api/v1/ecosystem/organizations/{org}/integrations/{id}"),
            None,
        ),
        (
            "DELETE",
            format!("/api/v1/ecosystem/organizations/{org}/prebuilt/quickbooks"),
            None,
        ),
    ];
    for who in [&resident, &tenant] {
        for (method, uri, body) in cases.clone() {
            let resp = call(&app, method, &uri, who, org, body).await;
            expect(
                &resp,
                StatusCode::FORBIDDEN,
                &format!("member {method} {uri}"),
            );
        }
    }

    // The escalation contact was not repointed.
    let hijacked = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM ai_escalation_configs WHERE organization_id = $1 AND escalation_email = 'attacker@example.com'",
    )
    .bind(org)
    .fetch_one(&pool)
    .await
    .unwrap_or(0);
    assert_eq!(hijacked, 0);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn rls_manager_mutations_succeed_for_managers(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "rls-ok").await;
    let mgr = member(&pool, org, "manager").await;
    // (The escalation-config happy path needs the LLM test tables; it is
    // covered by ai_llm_workflow_batch5_tests::update_escalation_config_returns_ok.)

    let resp = call(
        &app,
        "PUT",
        "/api/v1/ai/sentiment/thresholds",
        &mgr,
        org,
        Some(json!({ "negative_threshold": 0.6, "positive_threshold": 0.4 })),
    )
    .await;
    expect(
        &resp,
        StatusCode::OK,
        "manager updates sentiment thresholds",
    );

    let resp = call(&app, "GET", "/api/v1/ai/sentiment/alerts", &mgr, org, None).await;
    expect(&resp, StatusCode::OK, "manager lists sentiment alerts");

    // Data-residency configuration is admin-only (round 5: reads for
    // members, writes for admins), so this write runs as the org admin.
    let admin = member(&pool, org, "org_admin").await;
    let resp = call(
        &app,
        "POST",
        "/api/v1/data-residency/config",
        &admin,
        org,
        Some(json!({
            "primary_region": "eu_central",
            "backup_region": "ch_north",
            "allow_cross_region_access": false,
            "data_type_overrides": [],
            "compliance_notes": null
        })),
    )
    .await;
    expect(&resp, StatusCode::OK, "org admin configures data residency");

    let resp = call(&app, "GET", "/api/v1/data-residency/audit", &mgr, org, None).await;
    expect(&resp, StatusCode::OK, "manager reads residency audit");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn sentiment_alert_of_other_org_is_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "sent-a").await;
    let org_b = seed_org(&pool, "sent-b").await;
    let mgr_a = member(&pool, org_a, "manager").await;
    let alert_b = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO sentiment_alerts (organization_id, alert_type, threshold_breached, current_sentiment)
           VALUES ($1, 'anomaly', 0.5, -0.9) RETURNING id"#,
    )
    .bind(org_b)
    .fetch_one(&pool)
    .await
    .expect("seed alert");

    let resp = call(
        &app,
        "POST",
        &format!("/api/v1/ai/sentiment/alerts/{alert_b}/acknowledge"),
        &mgr_a,
        org_a,
        None,
    )
    .await;
    expect(
        &resp,
        StatusCode::NOT_FOUND,
        "acknowledge other org's alert",
    );
}

// ---------------------------------------------------------------------------
// Voice device unit binding
// ---------------------------------------------------------------------------

async fn seed_building(pool: &PgPool, org: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, street, city, postal_code, country)
           VALUES ($1, 'R4 St 1', 'Bratislava', '81101', 'Slovakia') RETURNING id"#,
    )
    .bind(org)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

async fn seed_unit(pool: &PgPool, building: Uuid, designation: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO units (building_id, designation) VALUES ($1, $2) RETURNING id"#,
    )
    .bind(building)
    .bind(designation)
    .fetch_one(pool)
    .await
    .expect("seed unit")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn voice_device_cannot_bind_foreign_or_neighbour_unit(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "voice-a").await;
    let org_b = seed_org(&pool, "voice-b").await;
    let resident = member(&pool, org_a, "resident").await;
    let building_a = seed_building(&pool, org_a).await;
    let building_b = seed_building(&pool, org_b).await;
    let neighbour_unit = seed_unit(&pool, building_a, "A-2").await;
    let foreign_unit = seed_unit(&pool, building_b, "B-1").await;

    for (unit, ctx) in [
        (neighbour_unit, "neighbour's unit"),
        (foreign_unit, "other org's unit"),
    ] {
        let resp = call(
            &app,
            "POST",
            "/api/v1/ai/llm/voice/devices",
            &resident,
            org_a,
            Some(json!({
                "platform": "alexa",
                "auth_code": "code",
                "unit_id": unit
            })),
        )
        .await;
        expect(&resp, StatusCode::NOT_FOUND, ctx);
    }
    let linked = count(
        &pool,
        "SELECT COUNT(*) FROM voice_assistant_devices WHERE user_id = $1",
        resident.id,
    )
    .await;
    assert_eq!(linked, 0);
}

// ---------------------------------------------------------------------------
// Integrations: manager gates on org channel actions
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn integration_channel_actions_require_manager(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "integ-gate").await;
    let resident = member(&pool, org, "resident").await;
    let base = format!("/api/v1/integrations/organizations/{org}");

    let cases: Vec<(&str, String, Option<serde_json::Value>)> = vec![
        ("POST", format!("{base}/airbnb/token/revoke"), None),
        ("POST", format!("{base}/airbnb/sync"), None),
        ("POST", format!("{base}/airbnb/connect"), Some(json!({}))),
        ("POST", format!("{base}/booking/sync"), None),
    ];
    for (method, uri, body) in cases {
        let resp = call(&app, method, &uri, &resident, org, body).await;
        expect(
            &resp,
            StatusCode::FORBIDDEN,
            &format!("resident {method} {uri}"),
        );
    }

    // Another org's member is still rejected by the membership guard.
    let other = seed_org(&pool, "integ-other").await;
    let outsider = member(&pool, other, "manager").await;
    let resp = call(
        &app,
        "POST",
        &format!("{base}/airbnb/token/revoke"),
        &outsider,
        other,
        None,
    )
    .await;
    expect(&resp, StatusCode::FORBIDDEN, "outsider revokes token");
}

// ---------------------------------------------------------------------------
// Feature analytics: org-scoped, manager-only, DB membership
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn feature_endpoints_use_db_membership(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "feat").await;
    let other = seed_org(&pool, "feat-other").await;
    let resident = member(&pool, org, "resident").await;
    let flag = Uuid::new_v4();

    // Login tokens carry no org claim: the X-Tenant-ID header is honoured.
    let resp = call(
        &app,
        "POST",
        "/api/v1/features/analytics/event",
        &resident,
        org,
        Some(json!({ "feature_key": "new_dashboard", "event_type": "access", "metadata": {} })),
    )
    .await;
    expect(&resp, StatusCode::OK, "member logs feature event");

    // ...but only for an org the caller belongs to.
    let resp = call(
        &app,
        "POST",
        "/api/v1/features/analytics/event",
        &resident,
        other,
        Some(json!({ "feature_key": "new_dashboard", "event_type": "access", "metadata": {} })),
    )
    .await;
    expect(&resp, StatusCode::FORBIDDEN, "event for foreign org");

    // Usage stats are manager-only.
    let resp = call(
        &app,
        "GET",
        &format!("/api/v1/features/analytics/{flag}/stats"),
        &resident,
        org,
        None,
    )
    .await;
    expect(&resp, StatusCode::FORBIDDEN, "resident reads feature stats");
}

// ---------------------------------------------------------------------------
// Organization members: role permission subset
// ---------------------------------------------------------------------------

async fn role_with(pool: &PgPool, org: Uuid, perms: serde_json::Value) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO roles (organization_id, name, permissions, is_system)
           VALUES ($1, $2, $3, FALSE) RETURNING id"#,
    )
    .bind(org)
    .bind(format!("R4 Role {}", Uuid::new_v4()))
    .bind(perms)
    .fetch_one(pool)
    .await
    .expect("seed role")
}

async fn role_id_of(pool: &PgPool, org: Uuid, user: Uuid) -> Option<Uuid> {
    sqlx::query_scalar::<_, Option<Uuid>>(
        "SELECT role_id FROM organization_members WHERE organization_id = $1 AND user_id = $2",
    )
    .bind(org)
    .bind(user)
    .fetch_one(pool)
    .await
    .expect("role_id")
}

/// A member-management permission does not let the caller hand out a role
/// row with permissions they lack (e.g. `["*"]`), whatever `role_type` says.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn members_cannot_assign_role_with_permissions_they_lack(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "mem-perm").await;
    let mgr_role = role_with(
        &pool,
        org,
        json!(["users:create", "users:update", "faults:read"]),
    )
    .await;
    let wildcard = wildcard_role(&pool, org).await;
    let users_star = role_with(&pool, org, json!(["users:*"])).await;
    let narrow = role_with(&pool, org, json!(["faults:read"])).await;

    let mgr = permissioned_member(&pool, org, "manager", mgr_role).await;
    let target = permissioned_member(&pool, org, "resident", narrow).await;

    for (role, label) in [(wildcard, "[*]"), (users_star, "[users:*]")] {
        let resp = call(
            &app,
            "PUT",
            &format!("/api/v1/organizations/{org}/members/{}", target.id),
            &mgr,
            org,
            Some(json!({ "role_id": role, "role_type": "owner" })),
        )
        .await;
        expect(&resp, StatusCode::FORBIDDEN, &format!("update to {label}"));
    }
    assert_eq!(role_id_of(&pool, org, target.id).await, Some(narrow));

    let (newbie, _) = seed_user(&pool, "perm-newbie").await;
    let resp = call(
        &app,
        "POST",
        &format!("/api/v1/organizations/{org}/members"),
        &mgr,
        org,
        Some(json!({ "user_id": newbie, "role_id": wildcard, "role_type": "tenant" })),
    )
    .await;
    expect(&resp, StatusCode::FORBIDDEN, "add member with [*] role");

    // A role whose permissions the caller holds is still assignable.
    let subset = role_with(&pool, org, json!(["users:update", "faults:read"])).await;
    let resp = call(
        &app,
        "PUT",
        &format!("/api/v1/organizations/{org}/members/{}", target.id),
        &mgr,
        org,
        Some(json!({ "role_id": subset, "role_type": "owner" })),
    )
    .await;
    expect(&resp, StatusCode::OK, "update to subset role");
    assert_eq!(role_id_of(&pool, org, target.id).await, Some(subset));

    let resp = call(
        &app,
        "POST",
        &format!("/api/v1/organizations/{org}/members"),
        &mgr,
        org,
        Some(json!({ "user_id": newbie, "role_id": narrow, "role_type": "tenant" })),
    )
    .await;
    expect(&resp, StatusCode::CREATED, "add member with subset role");
}

// ---------------------------------------------------------------------------
// Platform routes: admin-core capability gate
// ---------------------------------------------------------------------------
//
// `/api/v1/infrastructure/*` and the operator routes of `/api/v1/ecosystem/*`
// require platform kind + MFA enrollment + an active capability grant +
// recent MFA (audited), exactly like `/api/v1/platform-admin/*`. A bare
// `principal_kind = 'platform'` row is not enough.

/// A `principal_kind = 'platform'` user with a login-format token (no role
/// claims). The INSERT bypasses the BEFORE-UPDATE principal_kind guard.
async fn platform_user(pool: &PgPool, label: &str) -> Actor {
    let email = format!("r4-plat-{label}-{}@r4-platform.test", Uuid::new_v4());
    let id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
           VALUES ($1, 'hash', 'R4 Platform Operator', 'active', NOW(), 'platform')
           RETURNING id"#,
    )
    .bind(&email)
    .fetch_one(pool)
    .await
    .expect("seed platform user");
    Actor {
        id,
        token: login_token(id, &email),
    }
}

async fn enroll_mfa(pool: &PgPool, user: Uuid) {
    sqlx::query(
        r#"INSERT INTO user_2fa (user_id, secret, enabled, enabled_at, backup_codes, backup_codes_remaining)
           VALUES ($1, 'unused-secret', true, NOW(), '[]'::jsonb, 0)
           ON CONFLICT (user_id) DO UPDATE SET enabled = true, enabled_at = NOW()"#,
    )
    .bind(user)
    .execute(pool)
    .await
    .expect("enroll mfa");
}

/// Grant `capability` with the default `mfa_required = TRUE`, so the gate
/// also demands a recent MFA verification.
async fn grant(pool: &PgPool, user: Uuid, granted_by: Uuid, capability: &str) {
    sqlx::query(
        r#"INSERT INTO capability_grants (user_id, capability, granted_by, note)
           VALUES ($1, $2, $3, 'r4-platform-test')"#,
    )
    .bind(user)
    .bind(capability)
    .bind(granted_by)
    .execute(pool)
    .await
    .expect("grant capability");
}

async fn verify_mfa_now(pool: &PgPool, user: Uuid) {
    sqlx::query(
        "INSERT INTO two_factor_auth_verifications (user_id, method, verified_at) VALUES ($1, 'totp', NOW())",
    )
    .bind(user)
    .execute(pool)
    .await
    .expect("record mfa verification");
}

/// Capability-gate audit rows for `user` with the given outcome.
async fn capability_audits(pool: &PgPool, user: Uuid, outcome: &str) -> i64 {
    sqlx::query_scalar::<_, i64>(
        r#"SELECT COUNT(*) FROM audit_logs
           WHERE user_id = $1 AND details->>'source' = 'admin_core'
             AND details->>'outcome' = $2"#,
    )
    .bind(user)
    .bind(outcome)
    .fetch_one(pool)
    .await
    .expect("count audit rows")
}

/// Request without an `X-Tenant-ID` header (platform routes are global).
async fn call_global(
    app: &TestApp,
    method: &str,
    uri: &str,
    actor: &Actor,
    body: Option<serde_json::Value>,
) -> TestResponse {
    let builder = match method {
        "GET" => app.get(uri),
        "POST" => app.post(uri),
        "PUT" => app.put(uri),
        "DELETE" => app.delete(uri),
        other => panic!("unsupported method {other}"),
    };
    let mut builder = builder.bearer(&actor.token);
    if let Some(b) = body {
        builder = builder.json(b);
    }
    app.execute(builder.build()).await
}

async fn flag_count(pool: &PgPool, key: &str) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM infrastructure_feature_flags WHERE key = $1")
        .bind(key)
        .fetch_one(pool)
        .await
        .expect("count flags")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn infrastructure_requires_capability_grant_and_recent_mfa(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let granter = platform_user(&pool, "granter").await;
    let op = platform_user(&pool, "op").await;
    let key = format!("r4_flag_{}", Uuid::new_v4().simple());
    let flag = json!({
        "key": key,
        "name": "R4 flag",
        "enabled": false,
        "default_value": false,
        "value_type": "Boolean",
        "environment": "production"
    });
    let flags = "/api/v1/infrastructure/feature-flags";

    // Bare platform principal: no MFA enrollment, no grant.
    for (method, uri, body) in [
        ("GET", flags, None),
        ("POST", flags, Some(flag.clone())),
        ("GET", "/api/v1/infrastructure/dashboard", None),
        ("GET", "/api/v1/infrastructure/traces", None),
        ("GET", "/api/v1/infrastructure/jobs", None),
    ] {
        let resp = call_global(&app, method, uri, &op, body).await;
        expect(
            &resp,
            StatusCode::FORBIDDEN,
            &format!("bare {method} {uri}"),
        );
    }

    // Enrolled, still no grant.
    enroll_mfa(&pool, op.id).await;
    let resp = call_global(&app, "POST", flags, &op, Some(flag.clone())).await;
    expect(&resp, StatusCode::FORBIDDEN, "enrolled, no grant");
    assert_eq!(resp.json_value()["error"], json!("missing_capability"));

    // Granted, but no recent MFA verification.
    grant(&pool, op.id, granter.id, "feature_flags_write").await;
    let resp = call_global(&app, "POST", flags, &op, Some(flag.clone())).await;
    expect(&resp, StatusCode::UNAUTHORIZED, "granted, stale MFA");
    assert_eq!(flag_count(&pool, &key).await, 0, "no flag on any denial");

    // Grant + recent MFA: allowed.
    verify_mfa_now(&pool, op.id).await;
    let resp = call_global(&app, "POST", flags, &op, Some(flag.clone())).await;
    expect(&resp, StatusCode::CREATED, "granted + recent MFA");
    assert_eq!(flag_count(&pool, &key).await, 1);
    let resp = call_global(&app, "GET", flags, &op, None).await;
    expect(&resp, StatusCode::OK, "list flags with grant");

    // The grant is per capability: FeatureFlagsWrite does not open AuditRead
    // routes.
    let resp = call_global(&app, "GET", "/api/v1/infrastructure/traces", &op, None).await;
    expect(&resp, StatusCode::FORBIDDEN, "traces without audit_read");
    grant(&pool, op.id, granter.id, "audit_read").await;
    let resp = call_global(&app, "GET", "/api/v1/infrastructure/traces", &op, None).await;
    expect(&resp, StatusCode::OK, "traces with audit_read");

    // Every decision is audited.
    assert!(capability_audits(&pool, op.id, "denied").await >= 8);
    assert!(capability_audits(&pool, op.id, "allowed").await >= 3);

    // An ordinary org member is rejected as before.
    let org = seed_org(&pool, "infra-mgr").await;
    let mgr = member(&pool, org, "manager").await;
    let resp = call(&app, "POST", flags, &mgr, org, Some(flag)).await;
    expect(&resp, StatusCode::FORBIDDEN, "org manager");
}

fn marketplace_body() -> serde_json::Value {
    json!({
        "slug": format!("r4-int-{}", Uuid::new_v4()),
        "name": "R4 Integration",
        "description": "R4 capability gate",
        "category": "other",
        "vendor_name": "R4 Vendor",
        "version": "1.0.0"
    })
}

async fn developer_account(pool: &PgPool, user: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO developer_accounts (user_id, email, status)
           VALUES ($1, $2, 'approved') RETURNING id"#,
    )
    .bind(user)
    .bind(format!("dev-{}@r4-platform.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed developer account")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ecosystem_operator_routes_require_capability(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let granter = platform_user(&pool, "eco-granter").await;
    let op = platform_user(&pool, "eco-op").await;
    let (dev_id, dev_email) = seed_user(&pool, "developer").await;
    let dev = Actor {
        id: dev_id,
        token: login_token(dev_id, &dev_email),
    };
    let account = developer_account(&pool, dev.id).await;
    let keys = format!("/api/v1/ecosystem/developers/{account}/keys");

    // Bare platform principal: catalog writes, registration review and
    // portal stats are refused; someone else's developer keys are not
    // visible (the owner check applies).
    let resp = call_global(
        &app,
        "POST",
        "/api/v1/ecosystem/marketplace",
        &op,
        Some(marketplace_body()),
    )
    .await;
    expect(&resp, StatusCode::FORBIDDEN, "bare: create marketplace");
    let resp = call_global(&app, "GET", "/api/v1/ecosystem/portal/stats", &op, None).await;
    expect(&resp, StatusCode::FORBIDDEN, "bare: portal stats");
    let resp = call_global(
        &app,
        "POST",
        &format!("/api/v1/ecosystem/developers/{account}/review"),
        &op,
        Some(json!({ "status": "suspended" })),
    )
    .await;
    expect(&resp, StatusCode::FORBIDDEN, "bare: review registration");
    let resp = call_global(&app, "GET", &keys, &op, None).await;
    expect(&resp, StatusCode::FORBIDDEN, "bare: other developer's keys");

    // The owner keeps access to their own keys.
    let resp = call_global(&app, "GET", &keys, &dev, None).await;
    expect(&resp, StatusCode::OK, "owner: own keys");

    // Grants without a recent MFA verification are still refused.
    enroll_mfa(&pool, op.id).await;
    grant(&pool, op.id, granter.id, "site_settings_write").await;
    grant(&pool, op.id, granter.id, "oauth_client_write").await;
    let resp = call_global(
        &app,
        "POST",
        "/api/v1/ecosystem/marketplace",
        &op,
        Some(marketplace_body()),
    )
    .await;
    expect(
        &resp,
        StatusCode::UNAUTHORIZED,
        "granted, stale MFA: create marketplace",
    );
    let resp = call_global(&app, "GET", &keys, &op, None).await;
    expect(
        &resp,
        StatusCode::FORBIDDEN,
        "granted, stale MFA: other developer's keys",
    );

    // Grant + recent MFA: allowed.
    verify_mfa_now(&pool, op.id).await;
    let resp = call_global(
        &app,
        "POST",
        "/api/v1/ecosystem/marketplace",
        &op,
        Some(marketplace_body()),
    )
    .await;
    expect(&resp, StatusCode::OK, "granted + MFA: create marketplace");
    let resp = call_global(&app, "GET", &keys, &op, None).await;
    expect(
        &resp,
        StatusCode::OK,
        "granted + MFA: other developer's keys",
    );
    // Portal stats need audit_read, which this operator lacks.
    let resp = call_global(&app, "GET", "/api/v1/ecosystem/portal/stats", &op, None).await;
    expect(&resp, StatusCode::FORBIDDEN, "no audit_read: portal stats");

    // A developer is never an operator.
    let resp = call_global(
        &app,
        "POST",
        "/api/v1/ecosystem/marketplace",
        &dev,
        Some(marketplace_body()),
    )
    .await;
    expect(
        &resp,
        StatusCode::FORBIDDEN,
        "developer: create marketplace",
    );
}

/// Gap audit R7 (round-4 leftover): peers below admin level cannot manage each
/// other, a manager cannot remove an org admin, and the last org admin can be
/// neither demoted nor removed.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn members_peer_and_last_admin_guards(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "mem-peer").await;
    let role = wildcard_role(&pool, org).await;
    let mgr = permissioned_member(&pool, org, "manager", role).await;
    let peer = permissioned_member(&pool, org, "manager", role).await;
    let admin = permissioned_member(&pool, org, "org_admin", role).await;

    // Manager → manager peer: role change and removal are rejected.
    let resp = call(
        &app,
        "PUT",
        &format!("/api/v1/organizations/{org}/members/{}", peer.id),
        &mgr,
        org,
        Some(json!({ "role_id": role, "role_type": "resident" })),
    )
    .await;
    expect(&resp, StatusCode::FORBIDDEN, "demote a peer manager");
    assert_eq!(role_type_of(&pool, org, peer.id).await, "manager");
    let resp = call(
        &app,
        "DELETE",
        &format!("/api/v1/organizations/{org}/members/{}", peer.id),
        &mgr,
        org,
        None,
    )
    .await;
    expect(&resp, StatusCode::FORBIDDEN, "remove a peer manager");

    // Manager → org admin removal is rejected (used to need only users:delete).
    let resp = call(
        &app,
        "DELETE",
        &format!("/api/v1/organizations/{org}/members/{}", admin.id),
        &mgr,
        org,
        None,
    )
    .await;
    expect(&resp, StatusCode::FORBIDDEN, "manager removes org admin");

    // The only admin cannot leave the org.
    let resp = call(
        &app,
        "DELETE",
        &format!("/api/v1/organizations/{org}/members/{}", admin.id),
        &admin,
        org,
        None,
    )
    .await;
    expect(&resp, StatusCode::CONFLICT, "last admin leaves");

    // With a second admin, the first may hand over and be demoted by the peer.
    let admin2 = permissioned_member(&pool, org, "org_admin", role).await;
    let resp = call(
        &app,
        "PUT",
        &format!("/api/v1/organizations/{org}/members/{}", admin.id),
        &admin2,
        org,
        Some(json!({ "role_id": role, "role_type": "manager" })),
    )
    .await;
    expect(&resp, StatusCode::OK, "admin demotes peer admin");
    // ...but now admin2 is the last admin and cannot be demoted by anyone.
    let resp = call(
        &app,
        "DELETE",
        &format!("/api/v1/organizations/{org}/members/{}", admin2.id),
        &admin2,
        org,
        None,
    )
    .await;
    expect(&resp, StatusCode::CONFLICT, "new last admin leaves");
}
