//! Integration tests for the per-tenant `building_disabled` kill switch
//! (Phase 3, defends operational leak #22).
//!
//! Tested behavior:
//! - Seeding `tenant_feature_flags(building_disabled = true)` for a tenant
//!   makes `/tenant-config` return that flag enabled.
//! - The frontend logic (mocked here, since this is a backend test) trusts
//!   the flag value verbatim. The reality-web layout's behavior is covered
//!   by component-level tests; here we confirm the backend honors the
//!   write.

#![allow(dead_code)]

use api_core::middleware::host_tenant::{ResolvedTenant, TenantSource};
use axum::body::to_bytes;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

async fn seed_org(pool: &PgPool) -> Uuid {
    let slug = format!("ks-{}", Uuid::new_v4().simple());
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active') RETURNING id
        "#,
    )
    .bind("Kill Switch Test")
    .bind(&slug)
    .bind(format!("{slug}@example.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn set_flag(pool: &PgPool, org_id: Uuid, key: &str, enabled: bool) {
    sqlx::query(
        r#"
        INSERT INTO tenant_feature_flags (organization_id, flag_key, enabled)
        VALUES ($1, $2, $3)
        ON CONFLICT (organization_id, flag_key) DO UPDATE SET
            enabled = EXCLUDED.enabled, updated_at = NOW()
        "#,
    )
    .bind(org_id)
    .bind(key)
    .bind(enabled)
    .execute(pool)
    .await
    .expect("upsert flag");
}

async fn build_state(pool: PgPool) -> api_server::state::AppState {
    use api_core::middleware::{TenantRateLimiterSet, TenantResolutionCache};
    use api_server::services::{EmailService, JwtService};
    use api_server::state::AppState;
    use std::sync::Arc;

    let jwt =
        JwtService::new("tests-only-jwt-secret-at-least-thirty-two-chars-please").expect("jwt");
    let email = EmailService::new("http://test".into(), false);
    let cache = Arc::new(TenantResolutionCache::new(60, 30, 100));
    // Phase 5.5: feature-flag tests don't exercise the rate limiter, so the
    // 600-rpm default is plenty.
    let rate_limiters = Arc::new(TenantRateLimiterSet::new());
    AppState::new(pool, email, jwt, cache, rate_limiters)
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn building_disabled_flag_propagates_to_tenant_config(pool: PgPool) {
    let org = seed_org(&pool).await;
    set_flag(&pool, org, "building_disabled", true).await;

    let state = build_state(pool).await;
    let resp = api_server::routes::tenant_config::tenant_config_inner(
        state,
        Some(ResolvedTenant {
            organization_id: org,
            source: TenantSource::Subdomain,
        }),
    )
    .await;
    let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["feature_flags"]["building_disabled"]["enabled"], true);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn building_disabled_default_false_for_other_orgs(pool: PgPool) {
    // Seed org A with the flag ON; org B should be unaffected.
    let org_a = seed_org(&pool).await;
    set_flag(&pool, org_a, "building_disabled", true).await;

    let org_b = seed_org(&pool).await;
    // No flag row for B — frontend should treat as disabled=false.

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

    let bd = json["feature_flags"].get("building_disabled");
    // Migration 00134 seeds a row per org with enabled=false, so the
    // flag is present-but-false. Both Some(v) and None are acceptable;
    // when present we sanity-check enabled=false. No row = no override
    // = default off.
    if let Some(v) = bd {
        assert_eq!(v["enabled"], false);
    }
}

/// Mocked frontend kill-switch logic. The real check is in
/// `frontend/apps/reality-web/src/lib/feature-flags.ts` (`isKillSwitchOn`).
/// Asserting it here keeps the backend test self-contained: a flag-enabled
/// response from `/tenant-config` MUST be enough for the layout to render
/// 503 (no additional backend signal is required).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn flag_value_in_response_is_sufficient_for_frontend_503(pool: PgPool) {
    let org = seed_org(&pool).await;
    set_flag(&pool, org, "building_disabled", true).await;

    let state = build_state(pool).await;
    let resp = api_server::routes::tenant_config::tenant_config_inner(
        state,
        Some(ResolvedTenant {
            organization_id: org,
            source: TenantSource::Subdomain,
        }),
    )
    .await;
    let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Mocked frontend check — mirrors `isKillSwitchOn` in feature-flags.ts.
    let frontend_should_render_503 = json["feature_flags"]
        .get("building_disabled")
        .and_then(|f| f.get("enabled"))
        .and_then(|b| b.as_bool())
        .unwrap_or(false);

    assert!(
        frontend_should_render_503,
        "frontend layout would NOT render 503 — kill switch broken"
    );
}
