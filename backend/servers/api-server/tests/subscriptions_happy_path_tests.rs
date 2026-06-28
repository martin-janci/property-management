//! Happy path integration tests for the subscription endpoints (`/api/v1/subscriptions/*`).
//!
//! Exercises 25+ different endpoints covering plans, coupons, payment methods, organization subscriptions,
//! usage tracking, invoices, and statistics.

#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::{create_authenticated_user_with_org, TestApp, TestConfig, TestUser};

#[derive(Serialize)]
struct TestClaims {
    sub: String,
    email: String,
    name: String,
    exp: i64,
    iat: i64,
    jti: String,
    token_type: String,
    org_id: Option<String>,
    roles: Option<Vec<String>>,
    kind: Option<String>,
}

fn admin_token(user_id: Uuid, org_id: Uuid) -> String {
    let now = Utc::now().timestamp();
    let claims = TestClaims {
        sub: user_id.to_string(),
        email: "admin@example.com".to_string(),
        name: "Admin User".to_string(),
        exp: now + 3600,
        iat: now,
        jti: Uuid::new_v4().to_string(),
        token_type: "access".to_string(),
        org_id: Some(org_id.to_string()),
        roles: Some(vec!["SuperAdministrator".to_string()]),
        kind: Some("platform".to_string()),
    };
    let secret = TestConfig::default().jwt_secret;
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("encode admin JWT")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn subscriptions_endpoints_happy_path(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "subhappy").await;
    let session = app.session(token.clone(), org_id);

    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("resolve user id");

    let admin_tok = admin_token(user_id, org_id);
    let admin_session = app.session(admin_tok, org_id);

    // ========================================================================
    // 1. SUBSCRIPTION PLANS
    // ========================================================================

    // 1.1 POST /api/v1/subscriptions/plans -> create_plan (Admin only)
    let create_plan_payload = json!({
        "name": "enterprise-plan",
        "display_name": "Enterprise Plan",
        "description": "Unlimited buildings and users",
        "monthly_price": "299.00",
        "annual_price": "2999.00",
        "currency": "USD",
        "max_buildings": 100,
        "max_units": 1000,
        "max_users": 50,
        "max_storage_gb": 500,
        "trial_days": 14,
        "sort_order": 30
    });
    let resp = app
        .execute(
            admin_session
                .post("/api/v1/subscriptions/plans")
                .json(&create_plan_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::CREATED);
    let plan_a = resp.json_value();
    let plan_a_id = Uuid::parse_str(plan_a["id"].as_str().unwrap()).unwrap();

    // Create Plan B to test DELETE
    let create_plan_b_payload = json!({
        "name": "delete-me-plan",
        "display_name": "Temporary Plan",
        "monthly_price": "10.00",
        "currency": "USD"
    });
    let resp_b = app
        .execute(
            admin_session
                .post("/api/v1/subscriptions/plans")
                .json(&create_plan_b_payload)
                .build(),
        )
        .await;
    resp_b.assert_status(StatusCode::CREATED);
    let plan_b = resp_b.json_value();
    let plan_b_id = Uuid::parse_str(plan_b["id"].as_str().unwrap()).unwrap();

    // 1.2 GET /api/v1/subscriptions/plans -> list_plans
    let resp = app
        .execute(admin_session.get("/api/v1/subscriptions/plans").build())
        .await;
    resp.assert_status(StatusCode::OK);

    // 1.3 GET /api/v1/subscriptions/plans/{id} -> get_plan
    let resp = app
        .execute(
            admin_session
                .get(&format!("/api/v1/subscriptions/plans/{plan_b_id}"))
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 1.4 PATCH /api/v1/subscriptions/plans/{id} -> update_plan
    let update_plan_payload = json!({
        "display_name": "Updated Temporary Plan"
    });
    let resp = app
        .execute(
            admin_session
                .patch(&format!("/api/v1/subscriptions/plans/{plan_b_id}"))
                .json(&update_plan_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 1.5 DELETE /api/v1/subscriptions/plans/{id} -> delete_plan
    let resp = app
        .execute(
            admin_session
                .delete(&format!("/api/v1/subscriptions/plans/{plan_b_id}"))
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::NO_CONTENT);

    // 1.6 GET /api/v1/subscriptions/plans/public -> list_public_plans
    let resp = app
        .execute(session.get("/api/v1/subscriptions/plans/public").build())
        .await;
    resp.assert_status(StatusCode::OK);

    // ========================================================================
    // 2. PAYMENT METHODS
    // ========================================================================

    // 2.1 POST /api/v1/subscriptions/payment-methods -> create_payment_method
    let create_pm_payload = json!({
        "method_type": "card",
        "stripe_payment_method_id": "pm_mock_card",
        "is_default": true,
        "billing_address": {
            "country": "SK",
            "city": "Bratislava"
        }
    });
    // `create_payment_method` reads the org from the `organization_id` query
    // param (OrgQuery) and validates it against the RLS tenant.
    let resp = app
        .execute(
            session
                .post(&format!(
                    "/api/v1/subscriptions/payment-methods?organization_id={org_id}"
                ))
                .json(&create_pm_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::CREATED);
    let pm = resp.json_value();
    let pm_id = Uuid::parse_str(pm["id"].as_str().unwrap()).unwrap();

    // 2.2 GET /api/v1/subscriptions/payment-methods -> list_payment_methods
    let resp = app
        .execute(
            session
                .get(&format!(
                    "/api/v1/subscriptions/payment-methods?organization_id={org_id}"
                ))
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 2.3 POST /api/v1/subscriptions/payment-methods/{id}/default -> set_default_payment_method
    // Handler returns 204 NO_CONTENT (not 200), and requires organization_id.
    let resp = app
        .execute(
            session
                .post(&format!(
                    "/api/v1/subscriptions/payment-methods/{pm_id}/default?organization_id={org_id}"
                ))
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::NO_CONTENT);

    // ========================================================================
    // 3. ORGANIZATION SUBSCRIPTIONS
    // ========================================================================

    // 3.1 POST /api/v1/subscriptions/ -> create_subscription
    // `CreateSubscriptionRequest` carries `organization_id` at the top level and
    // flattens `CreateOrganizationSubscription` (plan_id, billing_cycle, ...).
    let create_sub_payload = json!({
        "organization_id": org_id,
        "plan_id": plan_a_id,
        "billing_cycle": "monthly",
        "start_trial": true,
        "payment_method_id": pm_id
    });
    let resp = app
        .execute(
            session
                .post("/api/v1/subscriptions")
                .json(&create_sub_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::CREATED);
    let sub = resp.json_value();
    let sub_id = Uuid::parse_str(sub["id"].as_str().unwrap()).unwrap();

    // 3.2 GET /api/v1/subscriptions/ -> get_subscription (OrgQuery)
    let resp = app
        .execute(
            session
                .get(&format!("/api/v1/subscriptions?organization_id={org_id}"))
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 3.3 GET /api/v1/subscriptions/with-plan -> get_subscription_with_plan (OrgQuery)
    let resp = app
        .execute(
            session
                .get(&format!(
                    "/api/v1/subscriptions/with-plan?organization_id={org_id}"
                ))
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 3.4 PATCH /api/v1/subscriptions/{id} -> update_subscription
    let update_sub_payload = json!({
        "billing_cycle": "annual"
    });
    let resp = app
        .execute(
            session
                .patch(&format!("/api/v1/subscriptions/{sub_id}"))
                .json(&update_sub_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 3.5 POST /api/v1/subscriptions/{id}/change-plan -> change_plan
    let change_plan_payload = json!({
        "new_plan_id": plan_a_id,
        "billing_cycle": "monthly"
    });
    let resp = app
        .execute(
            session
                .post(&format!("/api/v1/subscriptions/{sub_id}/change-plan"))
                .json(&change_plan_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 3.6 POST /api/v1/subscriptions/{id}/cancel -> cancel_subscription
    let cancel_payload = json!({
        "cancel_at_period_end": true,
        "cancellation_reason": "No longer needed"
    });
    let resp = app
        .execute(
            session
                .post(&format!("/api/v1/subscriptions/{sub_id}/cancel"))
                .json(&cancel_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 3.7 POST /api/v1/subscriptions/{id}/reactivate -> reactivate_subscription
    let resp = app
        .execute(
            session
                .post(&format!("/api/v1/subscriptions/{sub_id}/reactivate"))
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // ========================================================================
    // 4. USAGE RECORDING
    // ========================================================================

    // 4.1 POST /api/v1/subscriptions/usage -> record_usage
    // `RecordUsageRequest` carries `organization_id` at the top level and
    // flattens `CreateUsageRecord` (metric_type, quantity, unit).
    let create_usage_payload = json!({
        "organization_id": org_id,
        "metric_type": "api_calls",
        "quantity": "15.00",
        "unit": "calls"
    });
    let resp = app
        .execute(
            session
                .post("/api/v1/subscriptions/usage")
                .json(&create_usage_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::CREATED);

    // 4.2 GET /api/v1/subscriptions/usage/summary -> get_usage_summary (UsageSummaryQuery)
    let resp = app
        .execute(
            session
                .get(&format!(
                    "/api/v1/subscriptions/usage/summary?organization_id={org_id}"
                ))
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 4.3 GET /api/v1/subscriptions/usage/current -> get_current_usage (OrgQuery)
    let resp = app
        .execute(
            session
                .get(&format!(
                    "/api/v1/subscriptions/usage/current?organization_id={org_id}"
                ))
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // ========================================================================
    // 5. COUPONS
    // ========================================================================

    // 5.1 POST /api/v1/subscriptions/coupons -> create_coupon
    let create_coupon_payload = json!({
        "code": "PROMO2026",
        "name": "Summer Promo 2026",
        "description": "20% off",
        "discount_type": "percentage",
        "discount_value": "20.00",
        "duration": "once"
    });
    let resp = app
        .execute(
            admin_session
                .post("/api/v1/subscriptions/coupons")
                .json(&create_coupon_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::CREATED);
    let coupon = resp.json_value();
    let coupon_id = Uuid::parse_str(coupon["id"].as_str().unwrap()).unwrap();

    // 5.2 GET /api/v1/subscriptions/coupons -> list_coupons
    let resp = app
        .execute(admin_session.get("/api/v1/subscriptions/coupons").build())
        .await;
    resp.assert_status(StatusCode::OK);

    // 5.3 PATCH /api/v1/subscriptions/coupons/{id} -> update_coupon
    let update_coupon_payload = json!({
        "name": "Summer Promo 2026 Extended"
    });
    let resp = app
        .execute(
            admin_session
                .patch(&format!("/api/v1/subscriptions/coupons/{coupon_id}"))
                .json(&update_coupon_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 5.4 POST /api/v1/subscriptions/coupons/redeem -> redeem_coupon
    // Handler reads org from the `organization_id` query param (OrgQuery) and
    // the request body field is `coupon_code` (RedeemCouponRequest).
    let redeem_payload = json!({
        "coupon_code": "PROMO2026"
    });
    let resp = app
        .execute(
            session
                .post(&format!(
                    "/api/v1/subscriptions/coupons/redeem?organization_id={org_id}"
                ))
                .json(&redeem_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // ========================================================================
    // 6. INVOICES
    // ========================================================================

    // Direct seed a mock subscription invoice since there's no billing scheduler active in tests
    let invoice_id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO subscription_invoices
               (id, organization_id, subscription_id, invoice_number, invoice_date, due_date, subtotal, total_amount, currency, status)
           VALUES ($1, $2, $3, 'SUB-INV-1234', '2026-06-27', '2026-07-27', 10.00, 10.00, 'USD', 'draft')"#
    )
    .bind(invoice_id)
    .bind(org_id)
    .bind(sub_id)
    .execute(&app.pool)
    .await
    .expect("seed subscription invoice");

    // 6.1 GET /api/v1/subscriptions/invoices -> list_invoices (ListInvoicesQuery)
    let resp = app
        .execute(
            session
                .get(&format!(
                    "/api/v1/subscriptions/invoices?organization_id={org_id}"
                ))
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 6.2 GET /api/v1/subscriptions/invoices/{id} -> get_invoice
    let resp = app
        .execute(
            session
                .get(&format!("/api/v1/subscriptions/invoices/{invoice_id}"))
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 6.3 GET /api/v1/subscriptions/invoices/{id}/line-items -> get_invoice_line_items
    let resp = app
        .execute(
            session
                .get(&format!(
                    "/api/v1/subscriptions/invoices/{invoice_id}/line-items"
                ))
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 6.4 POST /api/v1/subscriptions/invoices/{id}/pay -> mark_invoice_paid
    let resp = app
        .execute(
            session
                .post(&format!("/api/v1/subscriptions/invoices/{invoice_id}/pay"))
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 6.5 POST /api/v1/subscriptions/invoices/{id}/void -> void_invoice
    let resp = app
        .execute(
            session
                .post(&format!("/api/v1/subscriptions/invoices/{invoice_id}/void"))
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // ========================================================================
    // 7. STATISTICS
    // ========================================================================

    // 7.1 GET /api/v1/subscriptions/statistics -> get_statistics (Admin only)
    let resp = app
        .execute(
            admin_session
                .get("/api/v1/subscriptions/statistics")
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // ========================================================================
    // CLEANUP
    // ========================================================================
    // Clean up created payment methods to avoid FK constraint errors in DB drop
    let resp = app
        .execute(
            session
                .delete(&format!(
                    "/api/v1/subscriptions/payment-methods/{pm_id}?organization_id={org_id}"
                ))
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::NO_CONTENT);
}
