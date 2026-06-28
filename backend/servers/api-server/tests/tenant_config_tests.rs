//! Integration tests for `/tenant-config` (Phase 3: Hosting & Theming).
//!
//! Tested behavior:
//! - Resolved tenant A → A's branding + flags.
//! - Resolved tenant B → B's branding + flags (no leak between tenants).
//! - No `ResolvedTenant` extension (platform host) → platform defaults,
//!   `tenant_id: null`.
//! - Cache headers: `Vary: Host` and `Cache-Control` with the right TTL.
//!
//! These tests call `tenant_config_inner` directly, which takes the
//! resolved-tenant Option, so we don't need to drive the full Phase 1
//! middleware here. Phase 1 has its own integration tests for that path
//! (`dev_mode_tenant_tests.rs`); this file isolates the Phase 3 surface.

#![allow(dead_code)]

use api_core::middleware::host_tenant::{ResolvedTenant, TenantSource};
use axum::body::to_bytes;
use serde_json::Value;
use sqlx::PgPool;

mod common_phase3 {
    //! Local helpers — kept out of the test module so multiple test files
    //! can `use` them.

    use sqlx::PgPool;
    use uuid::Uuid;

    pub async fn seed_org(pool: &PgPool, name: &str) -> Uuid {
        let slug = format!("tc-{}", Uuid::new_v4().simple());
        sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO organizations (name, slug, contact_email, status)
            VALUES ($1, $2, $3, 'active') RETURNING id
            "#,
        )
        .bind(name)
        .bind(&slug)
        .bind(format!("{slug}@example.test"))
        .fetch_one(pool)
        .await
        .expect("seed org")
    }

    pub async fn seed_branding(pool: &PgPool, org_id: Uuid, primary: &str, font: &str, logo: &str) {
        sqlx::query(
            r#"
            INSERT INTO agency_branding
                (organization_id, primary_color, font_family, logo_url, css_vars)
            VALUES ($1, $2, $3, $4, '{"--ppt-radius-md": "10px"}'::jsonb)
            "#,
        )
        .bind(org_id)
        .bind(primary)
        .bind(font)
        .bind(logo)
        .execute(pool)
        .await
        .expect("seed branding");
    }
}

/// Build a minimal AppState carrying just the DB pool and an empty
/// tenant resolution cache. The full state has many fields not needed
/// here; the route only touches `state.db`.
async fn build_state(pool: PgPool) -> api_server::state::AppState {
    use api_core::middleware::{TenantRateLimiterSet, TenantResolutionCache};
    use api_server::services::{EmailService, JwtService};
    use api_server::state::AppState;
    use std::sync::Arc;

    // Quiet the JWT-service constructor — it only matters if we exercise
    // an auth-gated path, which `/tenant-config` is not.
    let jwt =
        JwtService::new("tests-only-jwt-secret-at-least-thirty-two-chars-please").expect("jwt");
    let email = EmailService::new("http://test".into(), false);

    let cache = Arc::new(TenantResolutionCache::new(60, 30, 100));
    // Phase 5.5: tenant-config tests don't exercise the rate limiter — a
    // fresh default-rpm set is enough to satisfy `AppState::new`.
    let rate_limiters = Arc::new(TenantRateLimiterSet::new());
    AppState::new(pool, email, jwt, cache, rate_limiters)
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_host_returns_defaults(pool: PgPool) {
    let state = build_state(pool).await;
    let resp = api_server::routes::tenant_config::tenant_config_inner(state, None).await;

    assert_eq!(resp.status(), axum::http::StatusCode::OK);

    // Cache headers: Vary: Host + Cache-Control.
    let h = resp.headers();
    assert_eq!(h.get("vary").unwrap(), "Host");
    let cc = h.get("cache-control").unwrap().to_str().unwrap();
    assert!(cc.contains("max-age=60"), "cache-control was: {}", cc);

    let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json.get("tenant_id").unwrap().is_null());
    assert_eq!(json["name"], "Reality Portal");
    assert!(json["feature_flags"].as_object().unwrap().is_empty());
    assert!(json["locales"]
        .as_array()
        .unwrap()
        .contains(&Value::String("sk".into())));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn agency_a_returns_a_branding(pool: PgPool) {
    let org_a = common_phase3::seed_org(&pool, "Agency A").await;
    common_phase3::seed_branding(
        &pool,
        org_a,
        "#FF0000",
        "AgencyAFont, sans-serif",
        "https://cdn.example.test/a/logo.png",
    )
    .await;

    let state = build_state(pool).await;
    let resolved = ResolvedTenant {
        organization_id: org_a,
        source: TenantSource::Subdomain,
    };
    let resp = api_server::routes::tenant_config::tenant_config_inner(state, Some(resolved)).await;
    assert_eq!(resp.status(), axum::http::StatusCode::OK);

    let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["tenant_id"], org_a.to_string());
    assert_eq!(json["name"], "Agency A");
    assert_eq!(json["branding"]["primary_color"], "#FF0000");
    assert_eq!(json["branding"]["font_family"], "AgencyAFont, sans-serif");
    assert_eq!(
        json["branding"]["logo_url"],
        "https://cdn.example.test/a/logo.png"
    );
    // css_vars round-trips JSONB.
    assert_eq!(json["branding"]["css_vars"]["--ppt-radius-md"], "10px");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn agency_b_returns_b_branding_and_does_not_leak_a(pool: PgPool) {
    let org_a = common_phase3::seed_org(&pool, "Agency A").await;
    common_phase3::seed_branding(&pool, org_a, "#FF0000", "FontA", "https://a.example/logo").await;
    let org_b = common_phase3::seed_org(&pool, "Agency B").await;
    common_phase3::seed_branding(&pool, org_b, "#0000FF", "FontB", "https://b.example/logo").await;

    let state = build_state(pool).await;
    let resp = api_server::routes::tenant_config::tenant_config_inner(
        state,
        Some(ResolvedTenant {
            organization_id: org_b,
            source: TenantSource::Subdomain,
        }),
    )
    .await;

    let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["name"], "Agency B");
    assert_eq!(json["branding"]["primary_color"], "#0000FF");
    assert_eq!(json["branding"]["font_family"], "FontB");
    // Critical: no leak from A.
    assert_ne!(json["branding"]["primary_color"], "#FF0000");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn cache_headers_set_for_resolved_response(pool: PgPool) {
    let org = common_phase3::seed_org(&pool, "Agency C").await;
    let state = build_state(pool).await;
    let resp = api_server::routes::tenant_config::tenant_config_inner(
        state,
        Some(ResolvedTenant {
            organization_id: org,
            source: TenantSource::Subdomain,
        }),
    )
    .await;
    let h = resp.headers();
    assert_eq!(h.get("vary").unwrap(), "Host");
    let cc = h.get("cache-control").unwrap().to_str().unwrap();
    assert!(cc.contains("max-age=60"));
    assert!(cc.contains("s-maxage=60"));
}
