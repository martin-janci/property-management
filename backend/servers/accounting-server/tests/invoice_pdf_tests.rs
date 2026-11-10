//! HTTP-level integration tests for `get_invoice_pdf`
//! (`GET /api/v1/invoices/{id}/pdf`, UC-ACC-05.9). These drive the real router
//! through the auth/RLS extractors (mint an `api_core` bearer + `X-Tenant-ID`,
//! seed org/user/membership on the pool).
//!
//! These live in `tests/` (an integration-test crate that links the lib once)
//! rather than inline under `src/`. The crate uses the reality-server-style
//! lib/bin split where `main.rs` re-declares the same modules as `lib.rs`, so a
//! `#[cfg(test)] mod` inside `src/` compiles into BOTH the lib and the bin unit
//! test binaries. nextest then runs each `#[sqlx::test]` twice, and sqlx derives
//! the scratch-DB name from the test path — the two copies race to
//! `CREATE DATABASE` the same name and collide (`23505` / `55006`). An
//! integration-test binary is compiled exactly once, so the DB tests run once.
//! This mirrors reality-server, which keeps all its `#[sqlx::test]` suites under
//! `tests/`.
//!
//! NOTE on cross-tenant isolation: `#[sqlx::test]` connects as the `postgres`
//! superuser (see `backend.yml` `DATABASE_URL`), which BYPASSES row-level
//! security even under `FORCE ROW LEVEL SECURITY`. Cross-org IDOR is therefore
//! verified by the dedicated `rls-smoke-test` CI job (non-superuser
//! `rls_test_runner`), exactly as the api-server `accounting_invoices_tests`
//! suite documents. Here the 404 path is exercised with an unknown id, which is
//! role-agnostic and RLS-agnostic.

use accounting_server::state::AppState;
use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use db::models::accounting::{CreateInvoice, CreateInvoiceItem, InvoiceStatus};
use rust_decimal::Decimal;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

const TEST_JWT_SECRET: &str =
    "test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes";

/// `AuthUser` caches its `DecodingKey` from `JWT_SECRET` on first use
/// (`OnceLock`), so every test in this binary must agree on the secret. Set
/// it before building any request.
fn init_jwt_secret() {
    if std::env::var("JWT_SECRET").is_err() {
        std::env::set_var("JWT_SECRET", TEST_JWT_SECRET);
    }
}

/// Liberation Sans must be on disk for the 200 path to actually render a
/// PDF; CI installs `fonts-liberation`. Absent (bare dev host), skip the
/// render assertion rather than fail — matching the renderer smoke tests.
fn fonts_available() -> bool {
    std::path::Path::new(&accounting_server::pdf::font_dir())
        .join("LiberationSans-Regular.ttf")
        .exists()
}

fn mint(user_id: Uuid, org_id: Uuid, role: common::tenant::TenantRole) -> String {
    use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
    let now = chrono::Utc::now().timestamp();
    let claims = api_core::Claims {
        sub: user_id,
        exp: now + 3600,
        iat: now,
        token_type: "access".to_string(),
        tenant_id: Some(org_id),
        role: Some(role),
        email: "acc-pdf@test.internal".to_string(),
        name: "Acc PDF Test".to_string(),
    };
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(TEST_JWT_SECRET.as_bytes()),
    )
    .expect("mint token")
}

async fn seed_user(pool: &PgPool) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (email, password_hash, name, status) \
         VALUES ($1, 'x', 'Test User', 'active') RETURNING id",
    )
    .bind(format!("u-{}@test.internal", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_org(pool: &PgPool) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO organizations (name, slug, contact_email, status) \
         VALUES ('Acc PDF Org', $1, $2, 'active') RETURNING id",
    )
    .bind(format!("acc-pdf-{}", Uuid::new_v4()))
    .bind(format!("{}@test-org.internal", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_membership(pool: &PgPool, org_id: Uuid, user_id: Uuid, role_type: &str) {
    sqlx::query(
        "INSERT INTO organization_members (organization_id, user_id, role_type, status) \
         VALUES ($1, $2, $3, 'active') ON CONFLICT DO NOTHING",
    )
    .bind(org_id)
    .bind(user_id)
    .bind(role_type)
    .execute(pool)
    .await
    .expect("seed membership");
}

async fn seed_contact(pool: &PgPool, tenant_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO contact (tenant_id, name) VALUES ($1, 'Customer a.s.') RETURNING id",
    )
    .bind(tenant_id)
    .fetch_one(pool)
    .await
    .expect("seed contact")
}

/// Seed an issued invoice (with one line item) via the same repository the
/// route uses, so the stored rows match production shape. Returns its id.
async fn seed_invoice(state: &AppState, tenant_id: Uuid, contact_id: Uuid) -> Uuid {
    let data = CreateInvoice {
        tenant_id,
        contact_id,
        number: "2026/001".to_string(),
        issue_date: chrono::NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
        due_date: chrono::NaiveDate::from_ymd_opt(2026, 6, 15).unwrap(),
        taxable_supply_date: Some(chrono::NaiveDate::from_ymd_opt(2026, 6, 1).unwrap()),
        currency: "EUR".to_string(),
        variable_symbol: Some("2026001".to_string()),
        status: Some(InvoiceStatus::Issued),
        items: vec![CreateInvoiceItem {
            description: "Consulting".to_string(),
            qty: Decimal::from(1),
            unit_price: Decimal::from(100),
            vat_rate: Decimal::from(23),
            vat_rate_type: None,
        }],
        country: Some("SK".to_string()),
    };
    let mut conn = state.db.acquire().await.expect("acquire conn");
    let invoice = state
        .accounting_repo
        .create_invoice_rls(&mut conn, data)
        .await
        .expect("seed invoice");
    invoice.id
}

fn app(pool: PgPool) -> axum::Router {
    axum::Router::new()
        .nest("/api/v1", accounting_server::routes::api_router())
        .with_state(AppState::new(pool))
}

async fn get_pdf(pool: PgPool, id: Uuid, org_id: Uuid, token: &str) -> axum::response::Response {
    app(pool)
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/invoices/{id}/pdf"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("router response")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_invoice_pdf_returns_pdf_for_manager(pool: PgPool) {
    init_jwt_secret();
    let state = AppState::new(pool.clone());

    let org_id = seed_org(&pool).await;
    let user_id = seed_user(&pool).await;
    seed_membership(&pool, org_id, user_id, "manager").await;
    let contact_id = seed_contact(&pool, org_id).await;
    let invoice_id = seed_invoice(&state, org_id, contact_id).await;

    if !fonts_available() {
        eprintln!(
            "skipping PDF-body assertions: LiberationSans not found in {}",
            accounting_server::pdf::font_dir()
        );
        return;
    }

    let token = mint(user_id, org_id, common::tenant::TenantRole::Manager);
    let resp = get_pdf(pool, invoice_id, org_id, &token).await;

    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
        Some("application/pdf")
    );
    let disposition = resp
        .headers()
        .get(header::CONTENT_DISPOSITION)
        .and_then(|v| v.to_str().ok())
        .expect("content-disposition present");
    // Sanitized filename: `2026/001` -> `2026-001.pdf`.
    assert!(
        disposition.contains("filename=\"2026-001.pdf\""),
        "unexpected disposition: {disposition}"
    );

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("read body");
    assert!(body.len() > 100, "PDF should be non-trivial");
    assert_eq!(&body[..5], b"%PDF-", "output must be a PDF");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_invoice_pdf_404_for_unknown_id(pool: PgPool) {
    init_jwt_secret();

    let org_id = seed_org(&pool).await;
    let user_id = seed_user(&pool).await;
    seed_membership(&pool, org_id, user_id, "manager").await;

    let token = mint(user_id, org_id, common::tenant::TenantRole::Manager);
    let resp = get_pdf(pool, Uuid::new_v4(), org_id, &token).await;

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_invoice_pdf_403_below_read_min_role(pool: PgPool) {
    init_jwt_secret();
    let state = AppState::new(pool.clone());

    let org_id = seed_org(&pool).await;
    let user_id = seed_user(&pool).await;
    // Tenant is below READ_MIN_ROLE (Manager) — the DB-resolved role governs
    // authorization, not the JWT claim.
    seed_membership(&pool, org_id, user_id, "tenant").await;
    let contact_id = seed_contact(&pool, org_id).await;
    let invoice_id = seed_invoice(&state, org_id, contact_id).await;

    let token = mint(user_id, org_id, common::tenant::TenantRole::Tenant);
    let resp = get_pdf(pool, invoice_id, org_id, &token).await;

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}
