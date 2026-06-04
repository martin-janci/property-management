//! Realtime notification-preference sync tests (Epic 8A, Story 8A.3).
//!
//! The WebSocket *transport* leg landed in PR #472 and is covered by
//! `ws_integration_tests.rs` (upgrade, auth, fanout isolation). This file
//! closes the **#480–#487 follow-up cluster**: the other half of the realtime
//! sync contract — the `PATCH .../notification-preferences/{channel}` handler
//! publishing a `preference.updated` event to `notifications:{user_id}` so
//! connected clients can invalidate cached preference state without polling.
//!
//! | Case | Requires Redis | Asserts |
//! |------|----------------|---------|
//! | S1 | no  | A preference update succeeds (200) when Redis is **not** configured — the realtime publish is best-effort and MUST NOT couple to the DB write (the `if let Some(pubsub)` guard in the handler). |
//! | S2 | no  | Disabling a channel persists and the response carries the channel state — the DB leg is independent of the sync leg. |
//! | S3 | no  | The 409 "would disable all" guard fires **before** any publish, so a rejected update never emits a spurious `preference.updated`. |
//! | S4 | yes (`#[ignore]`) | A real PATCH publishes `preference.updated` to `notifications:{user_id}` on Redis with `{channel, enabled}` payload; a different user's channel receives nothing (isolation). |
//!
//! The no-Redis cases run in CI (the default `AppState` has `pubsub_service =
//! None`). The Redis case mirrors the `#[ignore]` style of the fanout test in
//! `ws_integration_tests.rs` — run locally with:
//!
//! ```sh
//! REDIS_URL=redis://localhost:6379 \
//!   cargo test -p api-server -- preference_update_publishes_realtime_event --include-ignored
//! ```

#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use common::{create_authenticated_user, TestApp, TestUser};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

// ----------------------------------------------------------------------------
// Tenant provisioning helpers.
//
// The notification-preference routes use `RlsConnection`, which resolves tenant
// context via `ValidatedTenantExtractor`. Under the `TestApp` harness (no
// `host_tenant_middleware`), that extractor requires BOTH an `X-Tenant-ID`
// header AND a matching active `organization_members` row — otherwise the
// request is rejected with 400 (missing header) or 403 (not a member) before
// the handler ever runs. A freshly-registered user from
// `create_authenticated_user` has no org membership, so we provision one here.
//
// Pattern mirrors the passing `building_manager_rbac_tests.rs` /
// `report_schedule_org_scope_jwt_tests.rs` suites.
// ----------------------------------------------------------------------------

/// Insert a fresh active organization and return its id.
async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO organizations (name, slug, contact_email, status)
           VALUES ($1, $2, $3, 'active') RETURNING id"#,
    )
    .bind(format!("NotifSync {slug}"))
    .bind(format!("notif-sync-{slug}-{}", Uuid::new_v4()))
    .bind(format!("{slug}-{}@notif-sync.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

/// Make `user_id` an active member of `org_id` with the given role.
async fn seed_membership(pool: &PgPool, org_id: Uuid, user_id: Uuid, role: &str) {
    sqlx::query(
        r#"INSERT INTO organization_members
               (id, organization_id, user_id, role_type, status, created_at)
           VALUES ($1, $2, $3, $4, 'active', NOW()) ON CONFLICT DO NOTHING"#,
    )
    .bind(Uuid::new_v4())
    .bind(org_id)
    .bind(user_id)
    .bind(role)
    .execute(pool)
    .await
    .expect("seed membership");
}

/// Resolve a user's id from their email.
async fn user_id_for(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("user id")
}

/// Register + log in a user, then make them an active member of a fresh org so
/// `RlsConnection` resolves tenant context. Returns `(access_token, org_id)`.
async fn authed_member(
    pool: &PgPool,
    app: &TestApp,
    user: &TestUser,
    slug: &str,
) -> (String, Uuid) {
    let org = seed_org(pool, slug).await;
    let (access_token, _refresh) = create_authenticated_user(app, user).await;
    let uid = user_id_for(pool, &user.email).await;
    seed_membership(pool, org, uid, "resident").await;
    (access_token, org)
}

// ============================================================================
// S1 — preference update succeeds with NO Redis configured (best-effort sync)
// ============================================================================

/// The realtime publish is wrapped in `if let Some(ref pubsub) = state.pubsub_service`
/// and the publish error is explicitly non-fatal. With the default `TestApp`
/// (no Redis), the PATCH must still return 200 — proving the DB write is not
/// coupled to the realtime sync leg. This is the core #480–#487 concern: a
/// missing/broken pubsub must never break preference updates.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn preference_update_succeeds_without_redis(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (access_token, org) = authed_member(&pool, &app, &user, "s1").await;

    // Disable the email channel — push + in_app remain enabled so this is not a
    // "disable all" case (that path is exercised in S3).
    let req = app
        .patch("/api/v1/users/me/notification-preferences/email")
        .bearer(&access_token)
        .header("X-Tenant-ID", &org.to_string())
        .json(json!({ "enabled": false, "confirmDisableAll": false }))
        .build();
    let resp = app.execute(req).await;

    resp.assert_status(StatusCode::OK);
    // The handler returns the updated preference; the realtime publish (skipped
    // here because Redis is absent) must not have changed the HTTP outcome.
    resp.assert_json_field("preference");
}

// ============================================================================
// S2 — the DB leg persists independently of the realtime sync leg
// ============================================================================

/// A successful PATCH must persist the new channel state regardless of whether
/// the realtime event went out. We re-read via the public GET endpoint to prove
/// the write landed (the sync leg is fire-and-forget and never gates this).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn preference_update_persists_independently_of_sync(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (access_token, org) = authed_member(&pool, &app, &user, "s2").await;

    // Disable push.
    let patch = app
        .patch("/api/v1/users/me/notification-preferences/push")
        .bearer(&access_token)
        .header("X-Tenant-ID", &org.to_string())
        .json(json!({ "enabled": false, "confirmDisableAll": false }))
        .build();
    app.execute(patch).await.assert_status(StatusCode::OK);

    // Read the preferences back — push must now be disabled.
    let get = app
        .get("/api/v1/users/me/notification-preferences")
        .bearer(&access_token)
        .header("X-Tenant-ID", &org.to_string())
        .build();
    let resp = app.execute(get).await;
    resp.assert_status(StatusCode::OK);

    let body = resp.json_value();
    let prefs = body["preferences"]
        .as_array()
        .expect("preferences array present");
    let push = prefs
        .iter()
        .find(|p| p["channel"] == json!("push"))
        .expect("push channel present in preferences");
    assert_eq!(
        push["enabled"],
        json!(false),
        "S2: push must be persisted as disabled after the PATCH (DB leg independent of sync leg)"
    );
}

// ============================================================================
// S3 — the "would disable all" 409 fires BEFORE any realtime publish
// ============================================================================

/// Disabling the last enabled channel without `confirmDisableAll` must be
/// rejected with 409 *before* the update + publish. This guards against a
/// spurious `preference.updated` being emitted for a change that never landed —
/// a realtime-sync correctness concern from the #480–#487 cluster.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn disable_all_guard_blocks_before_publish(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (access_token, org) = authed_member(&pool, &app, &user, "s3").await;

    // Disable every channel except in_app, without confirmation each time.
    for channel in ["email", "push"] {
        let req = app
            .patch(&format!(
                "/api/v1/users/me/notification-preferences/{channel}"
            ))
            .bearer(&access_token)
            .header("X-Tenant-ID", &org.to_string())
            .json(json!({ "enabled": false, "confirmDisableAll": false }))
            .build();
        app.execute(req).await.assert_status(StatusCode::OK);
    }

    // Now in_app is the only enabled channel; disabling it without confirmation
    // must be rejected with 409 (no update, no publish).
    let req = app
        .patch("/api/v1/users/me/notification-preferences/in_app")
        .bearer(&access_token)
        .header("X-Tenant-ID", &org.to_string())
        .json(json!({ "enabled": false, "confirmDisableAll": false }))
        .build();
    let resp = app.execute(req).await;
    resp.assert_status(StatusCode::CONFLICT);

    // The rejected change must NOT have persisted — in_app stays enabled, which
    // also means no `preference.updated` could have been published for it.
    let get = app
        .get("/api/v1/users/me/notification-preferences")
        .bearer(&access_token)
        .header("X-Tenant-ID", &org.to_string())
        .build();
    let body = app.execute(get).await.json_value();
    let in_app = body["preferences"]
        .as_array()
        .expect("preferences array")
        .iter()
        .find(|p| p["channel"] == json!("in_app"))
        .expect("in_app channel present");
    assert_eq!(
        in_app["enabled"],
        json!(true),
        "S3: a 409-rejected disable-all must leave in_app enabled (no update, no spurious publish)"
    );
}

// ============================================================================
// S4 (ignored — requires live Redis): PATCH publishes preference.updated
// ============================================================================

/// End-to-end realtime-sync proof: a real PATCH publishes `preference.updated`
/// to `notifications:{user_id}` on Redis, with the `{channel, enabled}` payload
/// the WS handler forwards verbatim. A second user's channel receives nothing.
///
/// Skipped in CI (no Redis). Run locally:
///
/// ```sh
/// REDIS_URL=redis://localhost:6379 \
///   cargo test -p api-server -- preference_update_publishes_realtime_event --include-ignored
/// ```
///
/// We subscribe via an **independent** `PubSubService` (distinct `instance_id`)
/// because `subscribe()` filters out messages originating from the same
/// instance — this mirrors the realistic cross-instance sync path (one server
/// publishes, another server's WS subscriber observes).
#[ignore = "requires live Redis — set REDIS_URL and run with --include-ignored"]
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn preference_update_publishes_realtime_event(pool: PgPool) {
    use integrations::{PubSubService, RedisClient, RedisConfig};
    use tokio::time::{sleep, Duration};

    let redis_url = match std::env::var("REDIS_URL") {
        Ok(url) => url,
        Err(_) => {
            eprintln!("REDIS_URL not set — skipping realtime preference-sync test");
            return;
        }
    };

    // Build a Redis-enabled TestApp so the PATCH handler has a pubsub_service.
    let app = {
        use api_server::services::{EmailService, JwtService};
        use api_server::state::AppState;

        // Match TestConfig::default() so create_authenticated_user's tokens validate.
        let config = common::TestConfig::default();
        // Prime JWT_SECRET / RUST_ENV the same way TestApp::with_config does.
        let _ = TestApp::new(pool.clone()).await;

        let email_service = EmailService::new(config.base_url.clone(), config.email_enabled);
        let jwt_service = JwtService::new(&config.jwt_secret).unwrap();
        let tenant_cache = std::sync::Arc::new(api_core::middleware::TenantResolutionCache::new(
            300, 30, 10_000,
        ));
        let tenant_rate_limiters =
            std::sync::Arc::new(api_core::middleware::TenantRateLimiterSet::new());
        let state = AppState::new(
            pool.clone(),
            email_service,
            jwt_service,
            tenant_cache,
            tenant_rate_limiters,
        )
        .with_redis(
            RedisClient::new(RedisConfig::new(&redis_url))
                .await
                .expect("redis connect"),
        );

        let router =
            api_server::create_router(state).layer(axum::extract::connect_info::MockConnectInfo(
                std::net::SocketAddr::from(([127, 0, 0, 1], 0)),
            ));
        TestApp {
            router,
            pool: pool.clone(),
            config,
        }
    };

    let user = TestUser::new();
    let (access_token, org) = authed_member(&pool, &app, &user, "s4").await;

    // Resolve the user's id from the DB so we can subscribe to their channel.
    let user_id: uuid::Uuid = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("lookup user id");

    // Independent subscriber (distinct instance_id) — observes cross-instance publish.
    let observer = PubSubService::new(
        RedisClient::new(RedisConfig::new(&redis_url))
            .await
            .expect("redis connect (observer)"),
    );
    let mut user_rx = observer
        .subscribe(&format!("notifications:{user_id}"))
        .await
        .expect("subscribe to user channel");
    let mut other_rx = observer
        .subscribe(&format!("notifications:{}", uuid::Uuid::new_v4()))
        .await
        .expect("subscribe to other channel");

    // Allow the subscriptions to establish before publishing.
    sleep(Duration::from_millis(200)).await;

    // Disable the email channel via the real PATCH route.
    let req = app
        .patch("/api/v1/users/me/notification-preferences/email")
        .bearer(&access_token)
        .header("X-Tenant-ID", &org.to_string())
        .json(json!({ "enabled": false, "confirmDisableAll": false }))
        .build();
    app.execute(req).await.assert_status(StatusCode::OK);

    // The user's channel must receive the preference.updated event.
    let received = tokio::time::timeout(Duration::from_secs(2), user_rx.recv())
        .await
        .expect("preference.updated must arrive within 2s")
        .expect("broadcast channel must yield the message");

    assert_eq!(
        received.event_type, "preference.updated",
        "S4: published event_type must be preference.updated"
    );
    assert_eq!(
        received.payload["channel"],
        json!("email"),
        "S4: payload.channel must echo the patched channel"
    );
    assert_eq!(
        received.payload["enabled"],
        json!(false),
        "S4: payload.enabled must echo the new state"
    );

    // The unrelated user's channel must NOT receive anything (isolation).
    let other = tokio::time::timeout(Duration::from_millis(300), other_rx.recv()).await;
    assert!(
        other.is_err(),
        "S4: a different user's channel must not receive this preference.updated event"
    );
}
