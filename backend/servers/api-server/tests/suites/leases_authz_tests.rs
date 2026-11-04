//! Authorization + org-ownership regression tests for the leases router
//! (`servers/api-server/src/routes/leases.rs`) and violation evidence paths.
//!
//! Covers:
//! - manager-only lease/template/screening/payment mutations reject a
//!   non-manager member with 403;
//! - id-path handlers return 404 for another org's ids (the integration-test
//!   pool is a superuser, so RLS does not bind — the explicit app-layer org
//!   guard is what is under test);
//! - `POST /leases/{id}/payments/{payment_id}` records a payment and validates
//!   that the payment belongs to the lease; the deprecated
//!   `POST /leases/{id}/payments` (where `{id}` is the payment id) still works;
//! - `POST /violations/{id}/evidence` rejects a `storage_path` outside the
//!   caller org's storage prefix with 400.
//!
//! `RlsConnection` resolves the caller's role from `organization_members`, so
//! every caller gets a membership row plus a JWT bound to the org.

#![allow(dead_code)]

use axum::http::StatusCode;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{seed_membership, seed_org, TestApp, TestConfig, TestResponse};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct TestClaims {
    sub: Uuid,
    exp: i64,
    iat: i64,
    token_type: String,
    tenant_id: Option<Uuid>,
    role: Option<String>,
    email: String,
    name: String,
}

fn mint(user_id: Uuid, org_id: Uuid, role: &str, email: &str) -> String {
    let now = chrono::Utc::now().timestamp();
    let claims = TestClaims {
        sub: user_id,
        exp: now + 3600,
        iat: now,
        token_type: "access".to_string(),
        tenant_id: Some(org_id),
        role: Some(role.to_string()),
        email: email.to_string(),
        name: format!("Lease authz {role}"),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(TestConfig::default().jwt_secret.as_bytes()),
    )
    .expect("encode test JWT")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'test_hash', 'Lease Authz User', 'active', NOW()) RETURNING id"#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// A member of `org` with `role`; returns `(user_id, token)`.
async fn member(pool: &PgPool, org: Uuid, role: &str) -> (Uuid, String) {
    member_with_email(
        pool,
        org,
        role,
        &format!("{role}-{}@lease-authz.test", Uuid::new_v4()),
    )
    .await
}

async fn member_with_email(pool: &PgPool, org: Uuid, role: &str, email: &str) -> (Uuid, String) {
    let uid = seed_user(pool, email).await;
    seed_membership(pool, org, uid, role).await;
    (uid, mint(uid, org, role, email))
}

/// One org's worth of lease data.
struct Fx {
    org: Uuid,
    lease: Uuid,
    payment: Uuid,
    template: Uuid,
    application: Uuid,
    applicant_email: String,
    screening: Uuid,
    violation: Uuid,
}

async fn arrange(pool: &PgPool, tag: &str) -> Fx {
    let org = seed_org(pool, &format!("lease-authz-{tag}")).await;
    let building: Uuid = sqlx::query_scalar(
        "INSERT INTO buildings (organization_id, name, street, city, postal_code, country, status) \
         VALUES ($1,'B','Main St 1','Testville','00000','SK','active') RETURNING id",
    )
    .bind(org)
    .fetch_one(pool)
    .await
    .expect("seed building");
    let unit: Uuid = sqlx::query_scalar(
        "INSERT INTO units (building_id, designation, floor) VALUES ($1,'U-1',1) RETURNING id",
    )
    .bind(building)
    .fetch_one(pool)
    .await
    .expect("seed unit");
    let lease: Uuid = sqlx::query_scalar(
        "INSERT INTO leases (organization_id, unit_id, landlord_name, tenant_name, tenant_email, \
             start_date, end_date, term_months, monthly_rent, security_deposit) \
         VALUES ($1,$2,'Landlord','Tenant','tenant@lease-authz.test', \
             CURRENT_DATE, CURRENT_DATE + 365, 12, 800.00, 1600.00) RETURNING id",
    )
    .bind(org)
    .bind(unit)
    .fetch_one(pool)
    .await
    .expect("seed lease");
    let payment: Uuid = sqlx::query_scalar(
        "INSERT INTO lease_payments (lease_id, organization_id, due_date, amount, payment_type) \
         VALUES ($1,$2,CURRENT_DATE + 30, 800.00, 'rent') RETURNING id",
    )
    .bind(lease)
    .bind(org)
    .fetch_one(pool)
    .await
    .expect("seed payment");
    let template: Uuid = sqlx::query_scalar(
        "INSERT INTO lease_templates (organization_id, name, content_html) \
         VALUES ($1,'T','<p>T</p>') RETURNING id",
    )
    .bind(org)
    .fetch_one(pool)
    .await
    .expect("seed template");
    let applicant_email = format!("applicant-{}@lease-authz.test", Uuid::new_v4());
    let application: Uuid = sqlx::query_scalar(
        "INSERT INTO tenant_applications (organization_id, unit_id, applicant_name, applicant_email) \
         VALUES ($1,$2,'Applicant',$3) RETURNING id",
    )
    .bind(org)
    .bind(unit)
    .bind(&applicant_email)
    .fetch_one(pool)
    .await
    .expect("seed application");
    let screening: Uuid = sqlx::query_scalar(
        "INSERT INTO tenant_screenings (application_id, organization_id, screening_type) \
         VALUES ($1,$2,'credit_check') RETURNING id",
    )
    .bind(application)
    .bind(org)
    .fetch_one(pool)
    .await
    .expect("seed screening");
    let violation: Uuid = sqlx::query_scalar(
        "INSERT INTO violations (organization_id, violation_number, category, title, description, occurred_at) \
         VALUES ($1, $2, 'noise', 'Loud', 'Noise', NOW()) RETURNING id",
    )
    .bind(org)
    .bind(format!("VIO-LA-{tag}"))
    .fetch_one(pool)
    .await
    .expect("seed violation");
    Fx {
        org,
        lease,
        payment,
        template,
        application,
        applicant_email,
        screening,
        violation,
    }
}

#[derive(Clone, Copy)]
enum M {
    Get,
    Post,
    Put,
    Patch,
}

async fn call(
    app: &TestApp,
    m: M,
    uri: &str,
    token: &str,
    org: Uuid,
    body: serde_json::Value,
) -> TestResponse {
    let b = match m {
        M::Get => app.get(uri),
        M::Post => app.post(uri),
        M::Put => app.put(uri),
        M::Patch => app.patch(uri),
    }
    .bearer(token)
    .tenant(org);
    let b = if matches!(m, M::Get) { b } else { b.json(body) };
    b.send().await
}

fn pay_body() -> serde_json::Value {
    json!({ "paid_amount": "800.00", "payment_method": "bank_transfer" })
}

/// Manager-only mutations on `fx` ids, with a minimal valid body each.
fn manager_mutations(fx: &Fx) -> Vec<(&'static str, M, String, serde_json::Value)> {
    let org = fx.org;
    vec![
        (
            "record_payment(new)",
            M::Post,
            format!("/api/v1/leases/{}/payments/{}", fx.lease, fx.payment),
            pay_body(),
        ),
        (
            "record_payment(legacy)",
            M::Post,
            format!("/api/v1/leases/{}/payments", fx.payment),
            pay_body(),
        ),
        (
            "update_template",
            M::Put,
            format!("/api/v1/leases/templates/{}", fx.template),
            json!({ "organization_id": org, "name": "Hacked" }),
        ),
        (
            "initiate_screening",
            M::Post,
            format!("/api/v1/leases/applications/{}/screening", fx.application),
            json!({ "organization_id": org, "screening_types": ["credit_check"] }),
        ),
        (
            "update_screening_result",
            M::Patch,
            format!("/api/v1/leases/screenings/{}/result", fx.screening),
            json!({ "passed": true }),
        ),
        (
            "update_lease",
            M::Put,
            format!("/api/v1/leases/{}", fx.lease),
            json!({ "notes": "x" }),
        ),
        (
            "terminate_lease",
            M::Post,
            format!("/api/v1/leases/{}/terminate", fx.lease),
            json!({ "termination_reason": "mutual_agreement", "effective_date": "2030-01-01" }),
        ),
        (
            "renew_lease",
            M::Post,
            format!("/api/v1/leases/{}/renew", fx.lease),
            json!({ "new_end_date": "2031-01-01", "term_months": 12 }),
        ),
        (
            "create_amendment",
            M::Post,
            format!("/api/v1/leases/{}/amendments", fx.lease),
            json!({ "title": "A", "description": "d", "changes": {}, "effective_date": "2030-01-01" }),
        ),
        (
            "create_reminder",
            M::Post,
            format!("/api/v1/leases/{}/reminders", fx.lease),
            json!({ "reminder_type": "renewal", "trigger_date": "2030-01-01", "subject": "s", "message": "m", "recipients": [] }),
        ),
    ]
}

// ---------------------------------------------------------------------------
// 403: non-manager members cannot perform manager-only mutations
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn tenant_is_forbidden_from_manager_only_lease_mutations(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let fx = arrange(&pool, "t403").await;
    let (_, tenant) = member(&pool, fx.org, "tenant").await;

    let mut cases = manager_mutations(&fx);
    cases.push((
        "create_template",
        M::Post,
        "/api/v1/leases/templates".into(),
        json!({ "organization_id": fx.org, "name": "T", "content_html": "<p/>" }),
    ));
    cases.push((
        "create_lease",
        M::Post,
        "/api/v1/leases".into(),
        json!({
            "organization_id": fx.org,
            "unit_id": Uuid::new_v4(),
            "landlord_name": "L",
            "tenant_name": "T",
            "tenant_email": "t@lease-authz.test",
            "start_date": "2030-01-01",
            "end_date": "2031-01-01",
            "term_months": 12,
            "monthly_rent": "800.00",
            "security_deposit": "1600.00"
        }),
    ));

    for (name, m, uri, body) in cases {
        let resp = call(&app, m, &uri, &tenant, fx.org, body).await;
        assert_eq!(
            resp.status,
            StatusCode::FORBIDDEN,
            "{name}: tenant must get 403, got {} body={}",
            resp.status,
            resp.text()
        );
    }

    // Nothing was written by the rejected payment calls.
    let paid: Option<rust_decimal::Decimal> =
        sqlx::query_scalar("SELECT paid_amount FROM lease_payments WHERE id = $1")
            .bind(fx.payment)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(paid.is_none(), "rejected record_payment must not persist");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resident_cannot_record_consent_or_read_others_application(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let fx = arrange(&pool, "r403").await;
    let (_, resident) = member(&pool, fx.org, "resident").await;

    for (name, m, uri, body) in [
        (
            "record_consent",
            M::Post,
            format!(
                "/api/v1/leases/applications/{}/screening/consent",
                fx.screening
            ),
            json!({}),
        ),
        (
            "get_application",
            M::Get,
            format!("/api/v1/leases/applications/{}", fx.application),
            json!({}),
        ),
        (
            "get_screening",
            M::Get,
            format!("/api/v1/leases/screenings/{}", fx.application),
            json!({}),
        ),
        (
            "list_applications",
            M::Get,
            format!("/api/v1/leases/applications?organization_id={}", fx.org),
            json!({}),
        ),
    ] {
        let resp = call(&app, m, &uri, &resident, fx.org, body).await;
        assert_eq!(
            resp.status,
            StatusCode::FORBIDDEN,
            "{name}: resident must get 403, got {} body={}",
            resp.status,
            resp.text()
        );
    }
}

/// The applicant (authenticated email == applicant_email) may consent to
/// their own screening without being a manager.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn applicant_can_consent_to_own_screening(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let fx = arrange(&pool, "appl").await;
    let (_, applicant) = member_with_email(&pool, fx.org, "tenant", &fx.applicant_email).await;

    let uri = format!(
        "/api/v1/leases/applications/{}/screening/consent",
        fx.screening
    );
    let resp = call(&app, M::Post, &uri, &applicant, fx.org, json!({})).await;
    assert_ne!(
        resp.status,
        StatusCode::FORBIDDEN,
        "applicant must pass: {}",
        resp.text()
    );
    assert_ne!(
        resp.status,
        StatusCode::NOT_FOUND,
        "applicant must pass: {}",
        resp.text()
    );

    let consented: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT consent_received_at FROM tenant_screenings WHERE id = $1")
            .bind(fx.screening)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(consented.is_some(), "consent must be recorded");
}

// ---------------------------------------------------------------------------
// 404: another org's ids are indistinguishable from missing ones
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn cross_org_ids_are_not_found(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let victim = arrange(&pool, "victim").await;
    let attacker_org = seed_org(&pool, "lease-authz-attacker").await;
    let (_, mgr) = member(&pool, attacker_org, "manager").await;

    // Mutations carry the attacker's own org id in the body where required.
    let mut cases: Vec<(&'static str, M, String, serde_json::Value)> = manager_mutations(&victim)
        .into_iter()
        .map(|(n, m, u, mut b)| {
            if let Some(o) = b.get_mut("organization_id") {
                *o = json!(attacker_org);
            }
            (n, m, u, b)
        })
        .collect();
    let reads = [
        ("get_lease", format!("/api/v1/leases/{}", victim.lease)),
        (
            "list_payments",
            format!("/api/v1/leases/{}/payments", victim.lease),
        ),
        (
            "list_amendments",
            format!("/api/v1/leases/{}/amendments", victim.lease),
        ),
        (
            "list_reminders",
            format!("/api/v1/leases/{}/reminders", victim.lease),
        ),
        (
            "get_template",
            format!("/api/v1/leases/templates/{}", victim.template),
        ),
        (
            "get_application",
            format!("/api/v1/leases/applications/{}", victim.application),
        ),
        (
            "get_screening",
            format!("/api/v1/leases/screenings/{}", victim.application),
        ),
    ];
    for (n, u) in reads {
        cases.push((n, M::Get, u, json!({})));
    }
    cases.push((
        "update_application",
        M::Put,
        format!("/api/v1/leases/applications/{}", victim.application),
        json!({ "applicant_name": "x" }),
    ));
    cases.push((
        "submit_application",
        M::Post,
        format!("/api/v1/leases/applications/{}/submit", victim.application),
        json!({}),
    ));
    cases.push((
        "review_application",
        M::Post,
        format!("/api/v1/leases/applications/{}/review", victim.application),
        json!({ "status": "approved" }),
    ));
    cases.push((
        "record_consent",
        M::Post,
        format!(
            "/api/v1/leases/applications/{}/screening/consent",
            victim.screening
        ),
        json!({}),
    ));
    cases.push((
        "send_for_signature",
        M::Post,
        format!("/api/v1/leases/{}/send-for-signature", victim.lease),
        json!({ "document_id": Uuid::new_v4() }),
    ));

    for (name, m, uri, body) in cases {
        let resp = call(&app, m, &uri, &mgr, attacker_org, body).await;
        assert_eq!(
            resp.status,
            StatusCode::NOT_FOUND,
            "{name}: cross-org id must be 404, got {} body={}",
            resp.status,
            resp.text()
        );
    }

    let paid: Option<rust_decimal::Decimal> =
        sqlx::query_scalar("SELECT paid_amount FROM lease_payments WHERE id = $1")
            .bind(victim.payment)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(paid.is_none(), "cross-org record_payment must not persist");
}

// ---------------------------------------------------------------------------
// Payments: new route + lease<->payment validation + legacy route
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn manager_records_payment_on_new_route(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let fx = arrange(&pool, "pnew").await;
    let (_, mgr) = member(&pool, fx.org, "manager").await;

    let uri = format!("/api/v1/leases/{}/payments/{}", fx.lease, fx.payment);
    let resp = call(&app, M::Post, &uri, &mgr, fx.org, pay_body()).await;
    assert_eq!(resp.status, StatusCode::OK, "body={}", resp.text());
    let v = resp.json_value();
    assert_eq!(v["id"], json!(fx.payment));
    assert_eq!(v["lease_id"], json!(fx.lease));
    assert!(!v["paid_at"].is_null(), "paid_at must be set");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn new_payment_route_rejects_payment_of_another_lease(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let fx = arrange(&pool, "pmis").await;
    let (_, mgr) = member(&pool, fx.org, "manager").await;

    // A second lease in the same org with its own payment.
    let other_lease: Uuid = sqlx::query_scalar(
        "INSERT INTO leases (organization_id, unit_id, landlord_name, tenant_name, tenant_email, \
             start_date, end_date, term_months, monthly_rent, security_deposit) \
         SELECT organization_id, unit_id, 'L', 'T2', 't2@lease-authz.test', start_date, end_date, \
             term_months, monthly_rent, security_deposit FROM leases WHERE id = $1 RETURNING id",
    )
    .bind(fx.lease)
    .fetch_one(&pool)
    .await
    .unwrap();

    // fx.payment belongs to fx.lease, not other_lease → 404.
    let uri = format!("/api/v1/leases/{other_lease}/payments/{}", fx.payment);
    let resp = call(&app, M::Post, &uri, &mgr, fx.org, pay_body()).await;
    assert_eq!(resp.status, StatusCode::NOT_FOUND, "body={}", resp.text());

    // Unknown payment id under a valid lease → 404.
    let uri = format!("/api/v1/leases/{}/payments/{}", fx.lease, Uuid::new_v4());
    let resp = call(&app, M::Post, &uri, &mgr, fx.org, pay_body()).await;
    assert_eq!(resp.status, StatusCode::NOT_FOUND, "body={}", resp.text());

    let paid: Option<rust_decimal::Decimal> =
        sqlx::query_scalar("SELECT paid_amount FROM lease_payments WHERE id = $1")
            .bind(fx.payment)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(
        paid.is_none(),
        "mismatched lease must not persist a payment"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn legacy_payment_route_still_works(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let fx = arrange(&pool, "pold").await;
    let (_, mgr) = member(&pool, fx.org, "manager").await;

    // Deprecated: `{id}` is the payment id.
    let uri = format!("/api/v1/leases/{}/payments", fx.payment);
    let resp = call(&app, M::Post, &uri, &mgr, fx.org, pay_body()).await;
    assert_eq!(resp.status, StatusCode::OK, "body={}", resp.text());
    assert_eq!(resp.json_value()["id"], json!(fx.payment));
}

// ---------------------------------------------------------------------------
// Violation evidence storage_path scope
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn evidence_storage_path_must_be_in_caller_org(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let fx = arrange(&pool, "evid").await;
    let (_, mgr) = member(&pool, fx.org, "manager").await;
    let uri = format!("/api/v1/violations/{}/evidence", fx.violation);

    for bad in [
        format!("{}/2026/01/x_photo.jpg", Uuid::new_v4()),
        "messages/abc/photo.jpg".to_string(),
        format!("{}/../{}/photo.jpg", fx.org, Uuid::new_v4()),
        "photo.jpg".to_string(),
    ] {
        let resp = call(
            &app,
            M::Post,
            &uri,
            &mgr,
            fx.org,
            json!({ "file_name": "photo.jpg", "file_type": "image/jpeg", "storage_path": bad }),
        )
        .await;
        assert_eq!(
            resp.status,
            StatusCode::BAD_REQUEST,
            "storage_path {bad:?} must be 400, body={}",
            resp.text()
        );
    }

    let ok_path = format!("{}/2026/01/{}_photo.jpg", fx.org, Uuid::new_v4());
    let resp = call(
        &app,
        M::Post,
        &uri,
        &mgr,
        fx.org,
        json!({ "file_name": "photo.jpg", "file_type": "image/jpeg", "storage_path": ok_path }),
    )
    .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "in-org path accepted, body={}",
        resp.text()
    );

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM violation_evidence WHERE violation_id = $1")
            .bind(fx.violation)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 1, "only the in-org evidence row is persisted");
}

// ---------------------------------------------------------------------------
// Lease lifecycle mutations: validation + 400/404 instead of 500
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn lease_mutations_validate_input_and_map_errors(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let fx = arrange(&pool, "life").await;
    let (_, mgr) = member(&pool, fx.org, "manager").await;
    let lease = fx.lease;
    let org = fx.org;

    // Unknown lease id → 404 on every mutation.
    let missing = Uuid::new_v4();
    for (name, m, uri, body) in [
        (
            "update",
            M::Put,
            format!("/api/v1/leases/{missing}"),
            json!({ "notes": "x" }),
        ),
        (
            "terminate",
            M::Post,
            format!("/api/v1/leases/{missing}/terminate"),
            json!({ "termination_reason": "other" }),
        ),
        (
            "renew",
            M::Post,
            format!("/api/v1/leases/{missing}/renew"),
            json!({ "new_end_date": "2040-01-01", "term_months": 12 }),
        ),
        (
            "amend",
            M::Post,
            format!("/api/v1/leases/{missing}/amendments"),
            json!({ "title": "A", "changes": {}, "effective_date": "2030-01-01" }),
        ),
    ] {
        let resp = call(&app, m, &uri, &mgr, org, body).await;
        assert_eq!(
            resp.status,
            StatusCode::NOT_FOUND,
            "{name}: {}",
            resp.text()
        );
    }

    // Invalid termination reason → 400 (was a 500 from the enum cast).
    let resp = call(
        &app,
        M::Post,
        &format!("/api/v1/leases/{lease}/terminate"),
        &mgr,
        org,
        json!({ "termination_reason": "Mutual agreement" }),
    )
    .await;
    assert_eq!(resp.status, StatusCode::BAD_REQUEST, "{}", resp.text());

    // Renewal end date not after the current end date → 400.
    let current_end: chrono::NaiveDate =
        sqlx::query_scalar("SELECT end_date FROM leases WHERE id = $1")
            .bind(lease)
            .fetch_one(&pool)
            .await
            .unwrap();
    for bad in [current_end, current_end - chrono::Duration::days(30)] {
        let resp = call(
            &app,
            M::Post,
            &format!("/api/v1/leases/{lease}/renew"),
            &mgr,
            org,
            json!({ "new_end_date": bad, "term_months": 12 }),
        )
        .await;
        assert_eq!(
            resp.status,
            StatusCode::BAD_REQUEST,
            "renew to {bad}: {}",
            resp.text()
        );
    }

    // Empty amendment title → 400; a valid amendment → 201.
    let amend_uri = format!("/api/v1/leases/{lease}/amendments");
    let resp = call(
        &app,
        M::Post,
        &amend_uri,
        &mgr,
        org,
        json!({ "title": "  ", "changes": {}, "effective_date": "2030-01-01" }),
    )
    .await;
    assert_eq!(resp.status, StatusCode::BAD_REQUEST, "{}", resp.text());
    let resp = call(
        &app,
        M::Post,
        &amend_uri,
        &mgr,
        org,
        json!({ "title": "Rent change", "changes": {}, "effective_date": "2030-01-01" }),
    )
    .await;
    assert_eq!(resp.status, StatusCode::CREATED, "{}", resp.text());

    // Valid update → 200.
    let resp = call(
        &app,
        M::Put,
        &format!("/api/v1/leases/{lease}"),
        &mgr,
        org,
        json!({ "notes": "updated" }),
    )
    .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());

    // Valid renewal → 201; renewing the now-renewed lease again → 400.
    let renew_uri = format!("/api/v1/leases/{lease}/renew");
    let new_end = current_end + chrono::Duration::days(365);
    let resp = call(
        &app,
        M::Post,
        &renew_uri,
        &mgr,
        org,
        json!({ "new_end_date": new_end, "term_months": 12 }),
    )
    .await;
    assert_eq!(resp.status, StatusCode::CREATED, "{}", resp.text());
    let renewed_id = resp.json_value()["id"].as_str().unwrap().to_string();
    let resp = call(
        &app,
        M::Post,
        &renew_uri,
        &mgr,
        org,
        json!({ "new_end_date": new_end, "term_months": 12 }),
    )
    .await;
    assert_eq!(resp.status, StatusCode::BAD_REQUEST, "{}", resp.text());

    // Terminate the new lease → 200; terminating it again → 400.
    let term_uri = format!("/api/v1/leases/{renewed_id}/terminate");
    let body = json!({ "termination_reason": "mutual_agreement" });
    let resp = call(&app, M::Post, &term_uri, &mgr, org, body.clone()).await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
    let resp = call(&app, M::Post, &term_uri, &mgr, org, body).await;
    assert_eq!(resp.status, StatusCode::BAD_REQUEST, "{}", resp.text());
}
