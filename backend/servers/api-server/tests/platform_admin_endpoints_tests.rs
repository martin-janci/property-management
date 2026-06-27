//! Integration tests for platform admin endpoints (Wave 2).
//! Covers tenants (list, get, suspend, reactivate, stats),
//! feature flags (create, list, get, update, toggle, overrides),
//! health (dashboard, alerts, thresholds),
//! announcements (create, list, get, update, delete),
//! and user features/resolved flags.

#[allow(dead_code)]
mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use api_server::services::JwtService;
use common::{seed_org, TestApp};

const JWT_SECRET: &str = "test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes";

/// Seed a platform-principal user. Returns the new user id.
async fn seed_platform_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'h', 'Platform Admin Test', 'active', NOW(), 'platform')
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed platform user")
}

/// Mark the user as MFA-enrolled (the capability gate's step 2.5 wall).
async fn enroll_mfa(pool: &PgPool, user_id: Uuid) {
    sqlx::query(
        r#"
        INSERT INTO user_2fa (user_id, secret, enabled, enabled_at, backup_codes, backup_codes_remaining)
        VALUES ($1, 'unused-secret', true, NOW(), '[]'::jsonb, 0)
        ON CONFLICT (user_id) DO UPDATE SET enabled = true, enabled_at = NOW()
        "#,
    )
    .bind(user_id)
    .execute(pool)
    .await
    .expect("enroll mfa");
}

/// Grant capability to the user with `mfa_required = false` for testing convenience.
async fn grant_capability(pool: &PgPool, user_id: Uuid, capability: &str, granted_by: Uuid) {
    sqlx::query(
        r#"
        INSERT INTO capability_grants
            (user_id, capability, granted_by, expires_at, mfa_required, note)
        VALUES ($1, $2, $3, NULL, false, 'ci-test-grant')
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(user_id)
    .bind(capability)
    .bind(granted_by)
    .execute(pool)
    .await
    .expect("grant capability");
}

/// Mint a platform-kind bearer JWT validated by `TestApp`'s JWT secret.
fn mint_platform_token(user_id: Uuid, email: &str, org_id: Option<Uuid>) -> String {
    let svc = JwtService::new(JWT_SECRET).expect("jwt service");
    svc.generate_access_token_with_kind(
        user_id,
        email,
        "Platform Admin Test",
        org_id,
        Some(vec!["super_admin".to_string()]),
        Some("platform".to_string()),
    )
    .expect("mint token")
}

fn request(method: Method, uri: &str, token: Option<&str>, body: Option<Value>) -> Request<Body> {
    let mut req = Request::builder().method(method).uri(uri);
    if let Some(t) = token {
        req = req.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    if let Some(b) = body {
        req = req.header(header::CONTENT_TYPE, "application/json");
        req.body(Body::from(b.to_string())).unwrap()
    } else {
        req.body(Body::empty()).unwrap()
    }
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_tenants_and_stats_workflow(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    // 1. Setup platform admin user with required capabilities
    let admin_id = seed_platform_user(&pool, "platform-admin-tenants@test.local").await;
    let granter_id = seed_platform_user(&pool, "platform-granter-tenants@test.local").await;
    enroll_mfa(&pool, admin_id).await;
    
    // Grant capabilities for reading agencies, suspending agencies, and viewing audit stats
    grant_capability(&pool, admin_id, "agencies_read", granter_id).await;
    grant_capability(&pool, admin_id, "agencies_suspend", granter_id).await;
    grant_capability(&pool, admin_id, "audit_read", granter_id).await;

    let token = mint_platform_token(admin_id, "platform-admin-tenants@test.local", None);

    // Seed organizations to query
    let org_a = seed_org(&pool, "org-a").await;
    let org_b = seed_org(&pool, "org-b").await;

    // 2. Test List Organizations
    let list_req = request(Method::GET, "/api/v1/platform-admin/organizations", Some(&token), None);
    let list_resp = app.execute(list_req).await;
    list_resp.assert_status(StatusCode::OK);
    
    let list_json = list_resp.json_value();
    let orgs = list_json["organizations"].as_array().expect("organizations field");
    assert!(orgs.iter().any(|o| o["id"] == org_a.to_string()));
    assert!(orgs.iter().any(|o| o["id"] == org_b.to_string()));

    // 3. Test Get Organization Detail
    let get_req = request(
        Method::GET,
        &format!("/api/v1/platform-admin/organizations/{org_a}"),
        Some(&token),
        None,
    );
    let get_resp = app.execute(get_req).await;
    get_resp.assert_status(StatusCode::OK);
    let get_json = get_resp.json_value();
    assert_eq!(get_json["id"], org_a.to_string());
    assert_eq!(get_json["status"], "active");

    // 4. Test Suspend Organization
    let suspend_req = request(
        Method::POST,
        &format!("/api/v1/platform-admin/organizations/{org_a}/suspend"),
        Some(&token),
        Some(json!({ "reason": "Non-payment of subscription fees" })),
    );
    let suspend_resp = app.execute(suspend_req).await;
    suspend_resp.assert_status(StatusCode::OK);
    let suspend_json = suspend_resp.json_value();
    assert_eq!(suspend_json["organization"]["status"], "suspended");

    // Verify DB side-effect of suspension
    let db_status: String = sqlx::query_scalar("SELECT status FROM organizations WHERE id = $1")
        .bind(org_a)
        .fetch_one(&pool)
        .await
        .expect("query org status");
    assert_eq!(db_status, "suspended");

    // 5. Test Reactivate Organization
    let reactivate_req = request(
        Method::POST,
        &format!("/api/v1/platform-admin/organizations/{org_a}/reactivate"),
        Some(&token),
        Some(json!({ "reason": "Re-activated after subscription renewal" })),
    );
    let reactivate_resp = app.execute(reactivate_req).await;
    reactivate_resp.assert_status(StatusCode::OK);
    let reactivate_json = reactivate_resp.json_value();
    assert_eq!(reactivate_json["organization"]["status"], "active");

    // Verify DB side-effect of reactivation
    let db_status_active: String = sqlx::query_scalar("SELECT status FROM organizations WHERE id = $1")
        .bind(org_a)
        .fetch_one(&pool)
        .await
        .expect("query org status");
    assert_eq!(db_status_active, "active");

    // 6. Test Platform Stats
    let stats_req = request(Method::GET, "/api/v1/platform-admin/stats", Some(&token), None);
    let stats_resp = app.execute(stats_req).await;
    stats_resp.assert_status(StatusCode::OK);
    let stats_json = stats_resp.json_value();
    assert!(stats_json["stats"].get("totalOrganizations").is_some());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_feature_flags_workflow(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    // Setup platform admin user with required capabilities
    let admin_id = seed_platform_user(&pool, "platform-admin-features@test.local").await;
    let granter_id = seed_platform_user(&pool, "platform-granter-features@test.local").await;
    enroll_mfa(&pool, admin_id).await;
    grant_capability(&pool, admin_id, "feature_flags_write", granter_id).await;

    let token = mint_platform_token(admin_id, "platform-admin-features@test.local", None);

    // 1. Create a feature flag
    let create_body = json!({
        "key": "test_flag_wave2",
        "name": "Wave 2 Test Flag",
        "description": "Used in Wave 2 integration tests",
        "is_enabled": false
    });
    let create_req = request(
        Method::POST,
        "/api/v1/platform-admin/feature-flags",
        Some(&token),
        Some(create_body),
    );
    let create_resp = app.execute(create_req).await;
    create_resp.assert_status(StatusCode::CREATED);
    let create_json = create_resp.json_value();
    let flag_id = create_json["flag"]["id"].as_str().expect("flag id field").to_string();
    assert_eq!(create_json["flag"]["key"], "test_flag_wave2");
    assert_eq!(create_json["flag"]["isEnabled"], false);

    // 2. List feature flags
    let list_req = request(Method::GET, "/api/v1/platform-admin/feature-flags", Some(&token), None);
    let list_resp = app.execute(list_req).await;
    list_resp.assert_status(StatusCode::OK);
    let list_json = list_resp.json_value();
    let flags = list_json["flags"].as_array().expect("flags array");
    assert!(flags.iter().any(|f| f["flag"]["key"] == "test_flag_wave2"));

    // 3. Get feature flag detail
    let get_req = request(
        Method::GET,
        &format!("/api/v1/platform-admin/feature-flags/{flag_id}"),
        Some(&token),
        None,
    );
    let get_resp = app.execute(get_req).await;
    get_resp.assert_status(StatusCode::OK);
    let get_json = get_resp.json_value();
    assert_eq!(get_json["flag"]["flag"]["key"], "test_flag_wave2");

    // 4. Update feature flag
    let update_body = json!({
        "name": "Updated Wave 2 Flag Name",
        "description": "Updated description"
    });
    let update_req = request(
        Method::PUT,
        &format!("/api/v1/platform-admin/feature-flags/{flag_id}"),
        Some(&token),
        Some(update_body),
    );
    let update_resp = app.execute(update_req).await;
    update_resp.assert_status(StatusCode::OK);
    let update_json = update_resp.json_value();
    assert_eq!(update_json["flag"]["name"], "Updated Wave 2 Flag Name");

    // 5. Toggle feature flag
    let toggle_req = request(
        Method::POST,
        &format!("/api/v1/platform-admin/feature-flags/{flag_id}/toggle"),
        Some(&token),
        None,
    );
    let toggle_resp = app.execute(toggle_req).await;
    toggle_resp.assert_status(StatusCode::OK);
    let toggle_json = toggle_resp.json_value();
    assert_eq!(toggle_json["flag"]["isEnabled"], true);

    // 6. Create feature flag override for a specific organization
    let org_id = seed_org(&pool, "flag-override-org").await;
    let override_body = json!({
        "scope_type": "organization",
        "scope_value": org_id.to_string(),
        "is_enabled": false
    });
    let override_req = request(
        Method::POST,
        &format!("/api/v1/platform-admin/feature-flags/{flag_id}/overrides"),
        Some(&token),
        Some(override_body),
    );
    let override_resp = app.execute(override_req).await;
    override_resp.assert_status(StatusCode::OK);
    let override_json = override_resp.json_value();
    let override_id = override_json["override"]["id"].as_str().expect("override id").to_string();

    // Verify detail reflects overrides
    let get_req2 = request(
        Method::GET,
        &format!("/api/v1/platform-admin/feature-flags/{flag_id}"),
        Some(&token),
        None,
    );
    let get_resp2 = app.execute(get_req2).await;
    let get_json2 = get_resp2.json_value();
    let overrides = get_json2["flag"]["overrides"].as_array().expect("overrides array");
    assert_eq!(overrides.len(), 1);
    assert_eq!(overrides[0]["id"], override_id);

    // 7. Delete feature flag override
    let del_override_req = request(
        Method::DELETE,
        &format!("/api/v1/platform-admin/feature-flags/{flag_id}/overrides/{override_id}"),
        Some(&token),
        None,
    );
    let del_override_resp = app.execute(del_override_req).await;
    del_override_resp.assert_status(StatusCode::OK);

    // 8. Delete feature flag
    let del_flag_req = request(
        Method::DELETE,
        &format!("/api/v1/platform-admin/feature-flags/{flag_id}"),
        Some(&token),
        None,
    );
    let del_flag_resp = app.execute(del_flag_req).await;
    del_flag_resp.assert_status(StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_health_and_announcements_workflow(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    // Setup platform admin user with required capabilities
    let admin_id = seed_platform_user(&pool, "platform-admin-ops@test.local").await;
    let granter_id = seed_platform_user(&pool, "platform-granter-ops@test.local").await;
    enroll_mfa(&pool, admin_id).await;
    grant_capability(&pool, admin_id, "audit_read", granter_id).await;
    grant_capability(&pool, admin_id, "site_settings_read", granter_id).await;
    grant_capability(&pool, admin_id, "site_settings_write", granter_id).await;

    let token = mint_platform_token(admin_id, "platform-admin-ops@test.local", None);

    // 1. Test Health Dashboard & Thresholds
    let dash_req = request(Method::GET, "/api/v1/platform-admin/health/dashboard", Some(&token), None);
    let dash_resp = app.execute(dash_req).await;
    dash_resp.assert_status(StatusCode::OK);

    let threshold_req = request(Method::GET, "/api/v1/platform-admin/health/thresholds", Some(&token), None);
    let threshold_resp = app.execute(threshold_req).await;
    threshold_resp.assert_status(StatusCode::OK);

    // 2. Create System Announcement
    let announce_body = json!({
        "title": "Wave 2 Maintenance Announcement",
        "content": "The system will undergo scheduled testing on Saturday.",
        "level": "info",
        "starts_at": chrono::Utc::now().to_rfc3339(),
        "ends_at": (chrono::Utc::now() + chrono::Duration::hours(24)).to_rfc3339()
    });
    let create_ann_req = request(
        Method::POST,
        "/api/v1/platform-admin/announcements",
        Some(&token),
        Some(announce_body),
    );
    let create_ann_resp = app.execute(create_ann_req).await;
    create_ann_resp.assert_status(StatusCode::CREATED);
    let create_ann_json = create_ann_resp.json_value();
    let announcement_id = create_ann_json["announcement"]["id"].as_str().expect("announcement id").to_string();

    // 3. List System Announcements
    let list_ann_req = request(Method::GET, "/api/v1/platform-admin/announcements", Some(&token), None);
    let list_ann_resp = app.execute(list_ann_req).await;
    list_ann_resp.assert_status(StatusCode::OK);
    let list_ann_json = list_ann_resp.json_value();
    let announcements = list_ann_json["announcements"].as_array().expect("announcements array");
    assert!(announcements.iter().any(|a| a["id"] == announcement_id));

    // 4. Get System Announcement Detail
    let get_ann_req = request(
        Method::GET,
        &format!("/api/v1/platform-admin/announcements/{announcement_id}"),
        Some(&token),
        None,
    );
    let get_ann_resp = app.execute(get_ann_req).await;
    get_ann_resp.assert_status(StatusCode::OK);
    let get_ann_json = get_ann_resp.json_value();
    assert_eq!(get_ann_json["id"], announcement_id);

    // 5. Update System Announcement
    let update_body = json!({
        "title": "Updated Wave 2 Maintenance Announcement",
        "content": "Updated content detail",
        "level": "warning"
    });
    let update_ann_req = request(
        Method::PUT,
        &format!("/api/v1/platform-admin/announcements/{announcement_id}"),
        Some(&token),
        Some(update_body),
    );
    let update_ann_resp = app.execute(update_ann_req).await;
    update_ann_resp.assert_status(StatusCode::OK);
    let update_ann_json = update_ann_resp.json_value();
    assert_eq!(update_ann_json["announcement"]["title"], "Updated Wave 2 Maintenance Announcement");

    // 6. Delete System Announcement
    let del_ann_req = request(
        Method::DELETE,
        &format!("/api/v1/platform-admin/announcements/{announcement_id}"),
        Some(&token),
        None,
    );
    let del_ann_resp = app.execute(del_ann_req).await;
    del_ann_resp.assert_status(StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn user_feature_resolution_and_analytics_workflow(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_id = seed_org(&pool, "user-feature-org").await;
    let user_id = seed_platform_user(&pool, "regular-user-features@test.local").await;
    
    // Mint a token with organization context
    let token = mint_platform_token(user_id, "regular-user-features@test.local", Some(org_id));

    // 1. Get resolved feature flags (public resolver)
    let res_flags_req = request(Method::GET, "/api/v1/feature-flags/", Some(&token), None);
    let res_flags_resp = app.execute(res_flags_req).await;
    res_flags_resp.assert_status(StatusCode::OK);

    // 2. Get resolved features (Epic 109)
    let res_features_req = request(Method::GET, "/api/v1/features/resolved", Some(&token), None);
    let res_features_resp = app.execute(res_features_req).await;
    res_features_resp.assert_status(StatusCode::OK);

    // 3. Check specific feature (e.g. key: "dummy_feature")
    let check_req = request(Method::GET, "/api/v1/features/dummy_feature/check", Some(&token), None);
    let check_resp = app.execute(check_req).await;
    check_resp.assert_status(StatusCode::OK);

    // 4. Get upgrade options for a feature
    let upgrade_req = request(Method::GET, "/api/v1/features/dummy_feature/upgrade-options", Some(&token), None);
    let upgrade_resp = app.execute(upgrade_req).await;
    upgrade_resp.assert_status(StatusCode::OK);

    // 5. Set user preference for a feature
    let pref_body = json!({
        "is_enabled": true
    });
    let pref_req = request(
        Method::POST,
        "/api/v1/features/dummy_feature/preference",
        Some(&token),
        Some(pref_body),
    );
    let pref_resp = app.execute(pref_req).await;
    pref_resp.assert_status(StatusCode::OK);

    // 6. Log feature usage analytics event
    let event_body = json!({
        "feature_key": "dummy_feature",
        "event_type": "click",
        "metadata": { "button_id": "test_btn" }
    });
    let event_req = request(
        Method::POST,
        "/api/v1/features/analytics/event",
        Some(&token),
        Some(event_body),
    );
    let event_resp = app.execute(event_req).await;
    event_resp.assert_status(StatusCode::OK);

    // 7. Get feature analytics stats
    // We query stats using dummy UUID as feature ID
    let dummy_feature_id = Uuid::new_v4();
    let stats_req = request(
        Method::GET,
        &format!("/api/v1/features/analytics/{dummy_feature_id}/stats"),
        Some(&token),
        None,
    );
    let stats_resp = app.execute(stats_req).await;
    stats_resp.assert_status(StatusCode::OK);
}
