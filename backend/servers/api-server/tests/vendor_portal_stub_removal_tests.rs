//! Regression tests for the vendor-portal stub-removal fix (Epic 78, issue #828).
//!
//! Audit history: every `/api/v1/vendor-portal/*` handler returned hard-coded
//! sample data — fabricated `VendorJob`s with invented `contact_phone` /
//! `building_address`, fake invoices, fake access codes — without ever touching
//! the database (`State(_state)` was unused). Any authenticated staff principal
//! received this fabricated PII as if it were live data, and there was no
//! vendor-identity scoping (no `vendor_staff` schema exists to bind a user to a
//! specific vendor company).
//!
//! The fix removes all fabricated responses: behind the `require_vendor_principal`
//! gate every handler now returns `501 Not Implemented` (mirroring the
//! developer-portal fix in `routes/public_api.rs`, #851) until the
//! vendor-identity schema lands.
//!
//! Two layers of assertion:
//!
//!   * **HTTP** — exercises the live router. Since PAP-33 the vendor-portal
//!     roadmap stub is unmounted from `lib.rs`, so every
//!     `/api/v1/vendor-portal/*` request 404s at the router. The
//!     security-meaningful invariant we pin here is the one the bug violated:
//!     **the endpoints never return a `200` carrying fabricated PII** (no
//!     `contact_phone` / `building_address`), and callers — authenticated or
//!     not — get a non-2xx response.
//!
//!   * **Repository** — the scoping primitive a future vendor-scoped portal must
//!     build on. `WorkOrderRepository::list` is org-scoped: a member of Org B
//!     must not see Org A's jobs, and Org A sees its own. (Repo-layer because
//!     the IDOR fix lives in the WHERE clause; see
//!     `ai_llm_cross_tenant_idor_tests.rs` for the same rationale.)

#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use sqlx::PgPool;
use uuid::Uuid;

use common::{TestApp, TestConfig};
use db::models::WorkOrderQuery;
use db::repositories::WorkOrderRepository;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active') RETURNING id
        "#,
    )
    .bind(format!("VendorPortal Org {slug}"))
    .bind(format!("vendor-portal-org-{slug}"))
    .bind(format!("{slug}@vendor-portal.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'VendorPortal User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_membership(pool: &PgPool, org_id: Uuid, user_id: Uuid) {
    sqlx::query(
        r#"
        INSERT INTO organization_members (organization_id, user_id, role_type, status, joined_at)
        VALUES ($1, $2, 'org_admin', 'active', NOW())
        "#,
    )
    .bind(org_id)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("seed membership");
}

/// Seed a building in `org_id` and return its id (work orders require one).
async fn seed_building(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO buildings (organization_id, street, city, postal_code, country)
        VALUES ($1, '123 Sunset Blvd', 'Bratislava', '81101', 'SK')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

/// Seed a work order ("job") in `org_id` / `building_id`.
async fn seed_work_order(pool: &PgPool, org_id: Uuid, building_id: Uuid, creator: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO work_orders
            (organization_id, building_id, title, description, priority, work_type,
             status, source, created_by)
        VALUES ($1, $2, 'Fix leaky faucet', 'Kitchen faucet drips', 'medium',
                'corrective', 'open', 'manual', $3)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(building_id)
    .bind(creator)
    .fetch_one(pool)
    .await
    .expect("seed work order")
}

fn mint_token(user_id: Uuid, email: &str) -> String {
    use api_server::services::JwtService;
    let config = TestConfig::default();
    let jwt = JwtService::new(&config.jwt_secret).expect("jwt service");
    jwt.generate_access_token(user_id, email, "VendorPortal User", None, None)
        .expect("mint access token")
}

// ---------------------------------------------------------------------------
// HTTP — fabricated PII / data is never returned (the #828 bug)
// ---------------------------------------------------------------------------

/// Before the fix, `get_job_details` returned a `200` with a fabricated
/// `VendorJob` (fake `contact_phone`, `building_address`). After the fix the
/// endpoint must NEVER serve a `200` with such data — an authenticated caller
/// gets a 4xx (no resolved tenant under TestApp) and an unauthenticated caller
/// gets `401`.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn job_details_never_returns_fabricated_pii(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "pii-a").await;
    let user_a = seed_user(&pool, "pii-a@vendor-portal.test").await;
    seed_membership(&pool, org_a, user_a).await;

    let job_id = Uuid::new_v4();
    let uri = format!("/api/v1/vendor-portal/jobs/{job_id}");

    // Authenticated staff member: must be rejected (no fabricated 200).
    let token = mint_token(user_a, "pii-a@vendor-portal.test");
    let resp = app.execute(app.get(&uri).bearer(&token).build()).await;
    assert_ne!(
        resp.status,
        StatusCode::OK,
        "vendor-portal job details must not return a 200 with fabricated PII: {}",
        resp.text()
    );
    let body = resp.text();
    assert!(
        !body.contains("555-123-4567") && !body.contains("Sunset Blvd"),
        "response must not leak the old hard-coded sample PII, got: {body}"
    );

    // Unauthenticated: rejected. Since PAP-33 unmounted the vendor-portal
    // roadmap stub from lib.rs, every request 404s before reaching auth —
    // assert no success (and no PII, above) rather than a specific auth status.
    let resp = app.execute(app.get(&uri).build()).await;
    assert_ne!(
        resp.status,
        StatusCode::OK,
        "unauthenticated vendor-portal access must be rejected: {}",
        resp.text()
    );
}

/// `list_jobs` and `list_invoices` previously returned `200` with a fabricated
/// array. Neither may return a `200` body anymore.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_endpoints_never_return_fabricated_arrays(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "lst-a").await;
    let user_a = seed_user(&pool, "lst-a@vendor-portal.test").await;
    seed_membership(&pool, org_a, user_a).await;
    let token = mint_token(user_a, "lst-a@vendor-portal.test");

    for uri in [
        "/api/v1/vendor-portal/jobs",
        "/api/v1/vendor-portal/invoices",
        "/api/v1/vendor-portal/dashboard/stats",
        "/api/v1/vendor-portal/profile",
    ] {
        let resp = app.execute(app.get(uri).bearer(&token).build()).await;
        assert_ne!(
            resp.status,
            StatusCode::OK,
            "{uri} must not return a fabricated 200 body: {}",
            resp.text()
        );
        let body = resp.text();
        assert!(
            !body.contains("Sunset Apartments") && !body.contains("INV-2024-001"),
            "{uri} must not leak old hard-coded sample data, got: {body}"
        );
    }
}

// ---------------------------------------------------------------------------
// Repository — org scoping is the primitive a vendor-scoped portal builds on
// ---------------------------------------------------------------------------

/// A member of Org B must not see Org A's jobs (work orders), and Org A sees
/// its own. This pins the org-scoping contract `WorkOrderRepository::list`
/// enforces — the foundation a future vendor-identity-scoped portal listing
/// will sit on. Without it, the portal would have no correct way to keep one
/// party's jobs from another's.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn work_orders_are_org_scoped(pool: PgPool) {
    let org_a = seed_org(&pool, "scope-a").await;
    let org_b = seed_org(&pool, "scope-b").await;
    let user_a = seed_user(&pool, "scope-a@vendor-portal.test").await;
    let building_a = seed_building(&pool, org_a).await;
    let job_a = seed_work_order(&pool, org_a, building_a, user_a).await;

    let repo = WorkOrderRepository::new(pool.clone());

    // Org B sees NOTHING of Org A's jobs.
    //
    // The repo is now stateless (PAP-179): every method takes an
    // RLS-context-bearing executor. This test runs on the `#[sqlx::test]`
    // superuser pool (RLS bypassed), so the `WHERE organization_id = $1`
    // predicate is what enforces org-scoping here; we pass `&pool` directly.
    let b_jobs = repo
        .list(&pool, org_b, WorkOrderQuery::default())
        .await
        .expect("list org_b work orders");
    assert!(
        b_jobs.iter().all(|w| w.id != job_a),
        "Org B must not see Org A's job {job_a}"
    );
    assert!(
        b_jobs.is_empty(),
        "Org B has no jobs of its own; expected an empty list, got {} rows",
        b_jobs.len()
    );

    // Org A sees its own job (the legitimate success case).
    let a_jobs = repo
        .list(&pool, org_a, WorkOrderQuery::default())
        .await
        .expect("list org_a work orders");
    assert!(
        a_jobs.iter().any(|w| w.id == job_a),
        "Org A must see its own job {job_a}"
    );
}
