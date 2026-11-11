//! Regression tests for the agency-branding partial-update clear semantics
//! (`reality-server/src/routes/agency_branding.rs::update_branding`).
//!
//! The original handler upserted with `COALESCE(EXCLUDED.x, agency_branding.x)`,
//! so a client could NEVER clear a branding field back to NULL — an explicit
//! `null` bound as SQL NULL and `COALESCE` kept the old value. The fix gives
//! each field three-state semantics:
//!   * absent            → leave the column unchanged,
//!   * explicit `null`   → clear the column to NULL,
//!   * explicit `""`     → clear the column to NULL,
//!   * a value           → set the column.
//!
//! These tests drive the real PUT endpoint and read the row straight from the
//! DB. They use only the HTTP + SQL surface (no handler-internal types), so the
//! same test compiles against the pre-fix code — where the clear assertions
//! fail, proving the defect — and the post-fix code, where they pass.
//!
//! Auth model mirrors `agencies_happy_path_tests`: the caller is a `platform`
//! principal (clears the tenant gate on a tenant-less host) and an active
//! `reality_agency_members` row (the handler's membership check).

use crate::common::{make_app_state, mint_token, send_json};
use axum::{http::Method, Router};
use reality_server::routes;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

fn branding_router(pool: PgPool) -> Router {
    let state = make_app_state(pool);
    Router::new()
        .nest(
            "/api/v1/agencies/{id}/branding",
            routes::agency_branding::router(),
        )
        .with_state(state)
}

async fn seed_user(pool: &PgPool, tag: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'hash', $2, 'active', NOW(), 'platform')
        RETURNING id
        "#,
    )
    .bind(format!("{tag}@branding-clear.test"))
    .bind(format!("BrandingClear {tag}"))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_user({tag}): {e}"))
}

async fn seed_agency(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO reality_agencies (name, slug, email)
        VALUES ($1, $2, $3)
        RETURNING id
        "#,
    )
    .bind(format!("Agency {slug}"))
    .bind(format!("agency-{slug}"))
    .bind(format!("{slug}@branding-clear.test"))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_agency({slug}): {e}"))
}

async fn seed_membership(pool: &PgPool, agency_id: Uuid, user_id: Uuid) {
    sqlx::query(
        "INSERT INTO reality_agency_members (agency_id, user_id, role) VALUES ($1, $2, 'owner')",
    )
    .bind(agency_id)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("seed_membership failed");
}

async fn fetch_col(pool: &PgPool, agency_id: Uuid, col: &str) -> Option<String> {
    // Static query per column (no dynamic SQL): keeps the statement text
    // `'static` and free of interpolated input.
    let sql: &'static str = match col {
        "logo_url" => "SELECT logo_url FROM agency_branding WHERE agency_id = $1",
        "banner_url" => "SELECT banner_url FROM agency_branding WHERE agency_id = $1",
        "primary_color" => "SELECT primary_color FROM agency_branding WHERE agency_id = $1",
        "secondary_color" => "SELECT secondary_color FROM agency_branding WHERE agency_id = $1",
        other => panic!("fetch_col: unsupported column {other}"),
    };
    sqlx::query_scalar::<_, Option<String>>(sql)
        .bind(agency_id)
        .fetch_one(pool)
        .await
        .unwrap_or_else(|e| panic!("fetch_col({col}): {e}"))
}

/// Core regression: an explicit `null` clears a previously-set field, while an
/// omitted field is left untouched. Under the old COALESCE upsert the cleared
/// field would have retained its value — this test fails on the pre-fix code.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn explicit_null_clears_field_absent_leaves_unchanged(pool: PgPool) {
    let user = seed_user(&pool, "null").await;
    let agency_id = seed_agency(&pool, "null").await;
    seed_membership(&pool, agency_id, user).await;
    let token = mint_token(user);
    let app = branding_router(pool.clone());

    // 1. Set logo_url and banner_url.
    let status = send_json(
        &app,
        Method::PUT,
        &format!("/api/v1/agencies/{agency_id}/branding"),
        Some(&token),
        json!({
            "logo_url": "https://cdn.example.com/logo.png",
            "banner_url": "https://cdn.example.com/banner.png"
        }),
    )
    .await;
    assert!(
        status.is_success(),
        "initial set: expected 2xx, got {status}"
    );
    assert_eq!(
        fetch_col(&pool, agency_id, "logo_url").await.as_deref(),
        Some("https://cdn.example.com/logo.png")
    );
    assert_eq!(
        fetch_col(&pool, agency_id, "banner_url").await.as_deref(),
        Some("https://cdn.example.com/banner.png")
    );

    // 2. Clear logo_url with an explicit null; omit banner_url entirely.
    let status = send_json(
        &app,
        Method::PUT,
        &format!("/api/v1/agencies/{agency_id}/branding"),
        Some(&token),
        json!({ "logo_url": null }),
    )
    .await;
    assert!(status.is_success(), "clear: expected 2xx, got {status}");

    // logo_url must now be NULL (cleared) ...
    assert_eq!(
        fetch_col(&pool, agency_id, "logo_url").await,
        None,
        "explicit null must clear logo_url back to NULL (COALESCE-no-clear regression)"
    );
    // ... and banner_url must be untouched (absent field = leave unchanged).
    assert_eq!(
        fetch_col(&pool, agency_id, "banner_url").await.as_deref(),
        Some("https://cdn.example.com/banner.png"),
        "an omitted field must be left unchanged"
    );
}

/// An empty string also clears the field (stored as NULL), so the frontend can
/// clear via `""` as well as `null`.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn empty_string_clears_field(pool: PgPool) {
    let user = seed_user(&pool, "empty").await;
    let agency_id = seed_agency(&pool, "empty").await;
    seed_membership(&pool, agency_id, user).await;
    let token = mint_token(user);
    let app = branding_router(pool.clone());

    let status = send_json(
        &app,
        Method::PUT,
        &format!("/api/v1/agencies/{agency_id}/branding"),
        Some(&token),
        json!({ "primary_color": "#ff0000" }),
    )
    .await;
    assert!(status.is_success(), "set: expected 2xx, got {status}");
    assert_eq!(
        fetch_col(&pool, agency_id, "primary_color")
            .await
            .as_deref(),
        Some("#ff0000")
    );

    let status = send_json(
        &app,
        Method::PUT,
        &format!("/api/v1/agencies/{agency_id}/branding"),
        Some(&token),
        json!({ "primary_color": "" }),
    )
    .await;
    assert!(status.is_success(), "clear: expected 2xx, got {status}");
    assert_eq!(
        fetch_col(&pool, agency_id, "primary_color").await,
        None,
        "empty string must clear primary_color back to NULL"
    );
}

/// Negative control: setting a field to a new value still works (the fix does
/// not break the set path), and clearing one field does not disturb a sibling
/// that is set in the same request.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn set_and_clear_in_same_request(pool: PgPool) {
    let user = seed_user(&pool, "mixed").await;
    let agency_id = seed_agency(&pool, "mixed").await;
    seed_membership(&pool, agency_id, user).await;
    let token = mint_token(user);
    let app = branding_router(pool.clone());

    // Seed both colors.
    let status = send_json(
        &app,
        Method::PUT,
        &format!("/api/v1/agencies/{agency_id}/branding"),
        Some(&token),
        json!({ "primary_color": "#111111", "secondary_color": "#222222" }),
    )
    .await;
    assert!(status.is_success(), "set: expected 2xx, got {status}");

    // Clear primary, re-set secondary, in one request.
    let status = send_json(
        &app,
        Method::PUT,
        &format!("/api/v1/agencies/{agency_id}/branding"),
        Some(&token),
        json!({ "primary_color": null, "secondary_color": "#333333" }),
    )
    .await;
    assert!(status.is_success(), "mixed: expected 2xx, got {status}");

    assert_eq!(
        fetch_col(&pool, agency_id, "primary_color").await,
        None,
        "primary_color must be cleared"
    );
    assert_eq!(
        fetch_col(&pool, agency_id, "secondary_color")
            .await
            .as_deref(),
        Some("#333333"),
        "secondary_color must be updated to the new value"
    );
}
