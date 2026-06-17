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
//! Coverage 8a-3 (idempotent apply) — additive no-Redis cases that lock down the
//! *idempotency* half of the realtime-sync contract (S1–S4 covered the publish
//! leg). A realtime client may replay the same PATCH on reconnect, or two tabs
//! may push the same desired state; the handler MUST converge on the same
//! persisted state and the same HTTP outcome regardless of how many times the
//! identical update is applied:
//!
//! | Case | Requires Redis | Asserts |
//! |------|----------------|---------|
//! | S5 | no  | Re-applying `enabled:false` to the same channel twice both return 200 and the channel stays disabled — `update_channel_rls` is an idempotent `UPDATE ... SET enabled=$3`, so a replayed PATCH is a safe no-op, not an error. |
//! | S6 | no  | Re-applying `enabled:true` to an already-enabled channel both return 200 and never trips the "would disable all" 409 (the guard only runs on `!enabled`) — a redundant enable is always idempotent. |
//! | S7 | no  | Disabling an **already-disabled** channel does not 409 even when another channel is the only enabled one: the guard keys off the *current* state of the channel being patched (`current.enabled`), so a replayed disable of a channel that is already off can never be the "last enabled" channel — proving repeated applies don't spuriously block. |
//!
//! Coverage 8a-3 (create/update edges) — additive no-Redis cases that lock down
//! the *create* (default provisioning) and *update* (channel-parse + restore +
//! confirmed disable-all warning) edges the realtime client depends on. These
//! complement S1–S7: S1–S4 prove the publish leg, S5–S7 the idempotency leg,
//! and S8–S11 the create/update contract the publish leg is built on:
//!
//! | Case | Requires Redis | Asserts |
//! |------|----------------|---------|
//! | S8  | no | A freshly-provisioned user's GET returns all three channels (push/email/in_app) **enabled** — the create leg (DB trigger seeds defaults) is what the first realtime sync snapshots. |
//! | S9  | no | An unknown channel segment is rejected with **400 INVALID_CHANNEL** before any update/publish — a malformed realtime replay can never mutate state or emit a spurious event. |
//! | S10 | no | A disable→enable round-trip on the same channel converges back to enabled (the *update transition* restoring state, distinct from S6's redundant enable of an already-on channel). |
//! | S11 | no | Disabling the last enabled channel **with** `confirmDisableAll:true` returns 200 and surfaces the `allDisabledWarning` — the confirmed disable-all path the 409 in S3 guards, proving the update succeeds and the warning rides the response. |
//!
//! Coverage 8a-3 (publish contract, CI-executable) — the publish/delivery leg
//! is the actual point of the #480–#487 / 8a-3 cluster, but the only test that
//! asserted an event was published (S4) is `#[ignore]`d and Redis-gated, so it
//! never ran in CI: a regression that silently dropped the `if let Some(pubsub)`
//! publish (or changed the channel name / `{channel, enabled}` payload shape)
//! would pass every CI-executed test (issue #1376). S12–S13 close that gap by
//! installing a `PreferenceEventRecorder` (via `TestApp::with_recording_pubsub`)
//! that captures what the handler would publish — no live Redis required:
//!
//! | Case | Requires Redis | Asserts |
//! |------|----------------|---------|
//! | S12 | no | A successful PATCH records **exactly one** `preference.updated` event on `notifications:{user_id}` with payload `{channel, enabled}` echoing the patched channel and new state — the publish contract S4 proves over real Redis, now assertable in CI. |
//! | S13 | no | A 409-rejected unconfirmed disable-all records **nothing** — the publish point sits after the disable-all guard, so a rejected change never emits a spurious `preference.updated` (the CI-executable mirror of S3's intent at the publish layer). |
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
use common::{create_authenticated_user_with_org, TestApp, TestUser};
use serde_json::json;
use sqlx::PgPool;

// ----------------------------------------------------------------------------
// Tenant provisioning.
//
// The notification-preference routes use `RlsConnection`, which resolves tenant
// context via `ValidatedTenantExtractor`. Under the `TestApp` harness (no
// `host_tenant_middleware`), that extractor requires BOTH an `X-Tenant-ID`
// header AND a matching active `organization_members` row — otherwise the
// request is rejected with 400 (missing header) or 403 (not a member) before
// the handler ever runs. A freshly-registered user from
// `create_authenticated_user` has no org membership.
//
// This file uses the shared `common::create_authenticated_user_with_org`
// fixture (promoted from this suite's former local helpers — see issue #1090):
// it registers+verifies+logs in the user, seeds a fresh org, grants membership,
// and returns `(access_token, org_id)`. Pass `org_id.to_string()` as the
// `X-Tenant-ID` header.
// ----------------------------------------------------------------------------

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
    let (access_token, org) = create_authenticated_user_with_org(&app, &user, "s1").await;

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
    let (access_token, org) = create_authenticated_user_with_org(&app, &user, "s2").await;

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
    let (access_token, org) = create_authenticated_user_with_org(&app, &user, "s3").await;

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
// S5 — idempotent disable: re-applying enabled:false is a safe 200 no-op
// ============================================================================

/// A realtime client may replay the same PATCH (reconnect, duplicate tab, retry
/// after a dropped response). Disabling the email channel twice in a row must
/// return 200 both times and leave email disabled — `update_channel_rls` is a
/// plain `UPDATE ... SET enabled=$3`, so a redundant disable converges on the
/// same persisted state rather than erroring. push + in_app stay enabled, so the
/// disable-all guard never enters the picture for either apply.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn repeated_disable_is_idempotent(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (access_token, org) = create_authenticated_user_with_org(&app, &user, "s5").await;

    // Apply the identical "disable email" PATCH twice; both must succeed.
    for attempt in 0..2 {
        let req = app
            .patch("/api/v1/users/me/notification-preferences/email")
            .bearer(&access_token)
            .header("X-Tenant-ID", &org.to_string())
            .json(json!({ "enabled": false, "confirmDisableAll": false }))
            .build();
        let resp = app.execute(req).await;
        resp.assert_status(StatusCode::OK);
        assert_eq!(
            resp.json_value()["preference"]["enabled"],
            json!(false),
            "S5: apply #{attempt} must report email disabled"
        );
    }

    // Final persisted state: email disabled, exactly once (idempotent — not
    // toggled, not duplicated).
    let get = app
        .get("/api/v1/users/me/notification-preferences")
        .bearer(&access_token)
        .header("X-Tenant-ID", &org.to_string())
        .build();
    let body = app.execute(get).await.json_value();
    let prefs = body["preferences"].as_array().expect("preferences array");
    let email: Vec<_> = prefs
        .iter()
        .filter(|p| p["channel"] == json!("email"))
        .collect();
    assert_eq!(
        email.len(),
        1,
        "S5: email must appear exactly once after a repeated disable (no duplicate rows)"
    );
    assert_eq!(
        email[0]["enabled"],
        json!(false),
        "S5: email must remain disabled after the second identical PATCH"
    );
}

// ============================================================================
// S6 — idempotent enable: re-applying enabled:true never trips the 409 guard
// ============================================================================

/// The "would disable all" guard runs only on `!enabled`, so a redundant enable
/// can never produce a 409. Re-enabling an already-enabled channel twice must
/// return 200 both times and leave the channel enabled — the enable path has no
/// state-dependent rejection, making it unconditionally idempotent.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn repeated_enable_is_idempotent_and_never_conflicts(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (access_token, org) = create_authenticated_user_with_org(&app, &user, "s6").await;

    // in_app starts enabled by default; re-enable it twice.
    for attempt in 0..2 {
        let req = app
            .patch("/api/v1/users/me/notification-preferences/in_app")
            .bearer(&access_token)
            .header("X-Tenant-ID", &org.to_string())
            .json(json!({ "enabled": true, "confirmDisableAll": false }))
            .build();
        let resp = app.execute(req).await;
        // A redundant enable must never be a 409 (the disable-all guard is gated
        // on `!enabled`) — it is an unconditional 200.
        resp.assert_status(StatusCode::OK);
        assert_eq!(
            resp.json_value()["preference"]["enabled"],
            json!(true),
            "S6: apply #{attempt} must report in_app enabled"
        );
    }

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
        "S6: in_app must remain enabled after repeated identical enables"
    );
}

// ============================================================================
// S7 — disabling an already-disabled channel never 409s, even as "last write"
// ============================================================================

/// The disable-all guard keys off the *current* state of the channel being
/// patched: `would_disable_all = enabled_count == 1 && current.enabled`. A
/// channel that is already disabled has `current.enabled == false`, so re-
/// disabling it can never be the last-enabled channel — the guard cannot fire.
///
/// Setup: disable email (push + in_app still on), then disable push (in_app is
/// now the only enabled channel). Re-disabling the already-off email channel
/// must still return 200 — a replayed disable of an off channel is a pure no-op
/// and must not be mistaken for "disabling the last channel". in_app stays on.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn redisable_of_off_channel_never_conflicts(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (access_token, org) = create_authenticated_user_with_org(&app, &user, "s7").await;

    // Disable email then push — in_app becomes the sole enabled channel.
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

    // Re-disable the already-disabled email channel without confirmation. Even
    // though in_app is the only enabled channel, this must NOT 409: email is
    // already off, so it is not the "last enabled" channel.
    let req = app
        .patch("/api/v1/users/me/notification-preferences/email")
        .bearer(&access_token)
        .header("X-Tenant-ID", &org.to_string())
        .json(json!({ "enabled": false, "confirmDisableAll": false }))
        .build();
    app.execute(req).await.assert_status(StatusCode::OK);

    // in_app must still be enabled — the no-op redisable changed nothing.
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
        "S7: in_app must stay enabled — re-disabling an already-off channel is a no-op, not a last-channel disable"
    );
}

// ============================================================================
// S8 — create leg: a fresh user's GET returns all channels enabled by default
// ============================================================================

/// The realtime-sync contract is built on top of the *create* leg: when a user
/// is provisioned, the DB trigger seeds one preference row per channel, all
/// enabled. The first GET a connected client issues snapshots that default
/// state, and every subsequent `preference.updated` is a delta against it. This
/// test pins the baseline — GET on an untouched user returns exactly the three
/// channels (push, email, in_app), each enabled, with no all-disabled warning.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn fresh_user_has_all_channels_enabled_by_default(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (access_token, org) = create_authenticated_user_with_org(&app, &user, "s8").await;

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

    // Exactly the three known channels, each enabled.
    let mut channels: Vec<&str> = prefs
        .iter()
        .map(|p| p["channel"].as_str().expect("channel is a string"))
        .collect();
    channels.sort_unstable();
    assert_eq!(
        channels,
        vec!["email", "in_app", "push"],
        "S8: a fresh user must have exactly the three default channels"
    );
    for p in prefs {
        assert_eq!(
            p["enabled"],
            json!(true),
            "S8: every default channel must be enabled (channel={})",
            p["channel"]
        );
    }
    // All channels enabled ⇒ no all-disabled warning on the create-leg snapshot.
    // The key MUST be present (emitted as explicit JSON null) — `is_some()`
    // distinguishes an emitted-but-null key from an absent one, which a bare
    // `== json!(null)` cannot (serde_json missing-index also yields Null).
    // (issue #1376, secondary finding.)
    let warning = body
        .get("allDisabledWarning")
        .expect("S8: allDisabledWarning key must be present (emitted as null), not absent");
    assert_eq!(
        warning,
        &json!(null),
        "S8: a fresh user with all channels enabled must carry a null all-disabled warning"
    );
}

// ============================================================================
// S9 — update guard: an unknown channel is rejected (400) before any mutation
// ============================================================================

/// A malformed realtime replay (typo'd channel, stale enum value from an older
/// client) must be rejected at the channel-parse step with 400 INVALID_CHANNEL
/// — *before* any DB write or `preference.updated` publish. This guarantees the
/// publish leg can never emit an event for a channel the system doesn't model,
/// and leaves the user's real preferences untouched.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn unknown_channel_is_rejected_without_mutation(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (access_token, org) = create_authenticated_user_with_org(&app, &user, "s9").await;

    let req = app
        .patch("/api/v1/users/me/notification-preferences/sms")
        .bearer(&access_token)
        .header("X-Tenant-ID", &org.to_string())
        .json(json!({ "enabled": false, "confirmDisableAll": false }))
        .build();
    let resp = app.execute(req).await;
    resp.assert_status(StatusCode::BAD_REQUEST);

    // The known channels must be untouched — all still enabled (the bad PATCH
    // could not have mutated state or published a spurious event).
    let get = app
        .get("/api/v1/users/me/notification-preferences")
        .bearer(&access_token)
        .header("X-Tenant-ID", &org.to_string())
        .build();
    let body = app.execute(get).await.json_value();
    let all_enabled = body["preferences"]
        .as_array()
        .expect("preferences array")
        .iter()
        .all(|p| p["enabled"] == json!(true));
    assert!(
        all_enabled,
        "S9: a rejected unknown-channel PATCH must leave every real channel untouched (still enabled)"
    );
}

// ============================================================================
// S10 — update transition: a disable→enable round-trip restores enabled state
// ============================================================================

/// Distinct from S6 (redundant enable of an already-on channel), this exercises
/// the *transition* a realtime client most often drives: a user turns a channel
/// off, then back on. The enable PATCH must flip `current.enabled` false→true
/// and the persisted state must converge to enabled — proving the update leg
/// the publish leg rides is a genuine state machine, not a one-way latch.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn disable_then_enable_restores_channel(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (access_token, org) = create_authenticated_user_with_org(&app, &user, "s10").await;

    // Disable push.
    let disable = app
        .patch("/api/v1/users/me/notification-preferences/push")
        .bearer(&access_token)
        .header("X-Tenant-ID", &org.to_string())
        .json(json!({ "enabled": false, "confirmDisableAll": false }))
        .build();
    let resp = app.execute(disable).await;
    resp.assert_status(StatusCode::OK);
    assert_eq!(
        resp.json_value()["preference"]["enabled"],
        json!(false),
        "S10: the disable PATCH must report push disabled"
    );

    // Re-enable push.
    let enable = app
        .patch("/api/v1/users/me/notification-preferences/push")
        .bearer(&access_token)
        .header("X-Tenant-ID", &org.to_string())
        .json(json!({ "enabled": true, "confirmDisableAll": false }))
        .build();
    let resp = app.execute(enable).await;
    resp.assert_status(StatusCode::OK);
    assert_eq!(
        resp.json_value()["preference"]["enabled"],
        json!(true),
        "S10: the enable PATCH must report push re-enabled"
    );

    // Persisted state must converge to enabled.
    let get = app
        .get("/api/v1/users/me/notification-preferences")
        .bearer(&access_token)
        .header("X-Tenant-ID", &org.to_string())
        .build();
    let body = app.execute(get).await.json_value();
    let push = body["preferences"]
        .as_array()
        .expect("preferences array")
        .iter()
        .find(|p| p["channel"] == json!("push"))
        .expect("push channel present");
    assert_eq!(
        push["enabled"],
        json!(true),
        "S10: push must persist as enabled after the disable→enable round-trip"
    );
}

// ============================================================================
// S11 — confirmed disable-all: 200 + allDisabledWarning rides the response
// ============================================================================

/// S3 proves the 409 guard blocks an *unconfirmed* last-channel disable. This is
/// the other side: with `confirmDisableAll:true` the update must succeed (200),
/// land the channel as disabled, AND surface `allDisabledWarning` on the
/// response so a connected client can render the "you may miss updates" notice.
/// This is the update-leg warning the publish leg notifies clients about.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn confirmed_disable_all_succeeds_with_warning(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (access_token, org) = create_authenticated_user_with_org(&app, &user, "s11").await;

    // Disable email + push (in_app becomes the only enabled channel).
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

    // Disable the last channel WITH confirmation — must succeed (200), not 409.
    let req = app
        .patch("/api/v1/users/me/notification-preferences/in_app")
        .bearer(&access_token)
        .header("X-Tenant-ID", &org.to_string())
        .json(json!({ "enabled": false, "confirmDisableAll": true }))
        .build();
    let resp = app.execute(req).await;
    resp.assert_status(StatusCode::OK);

    let body = resp.json_value();
    assert_eq!(
        body["preference"]["enabled"],
        json!(false),
        "S11: the confirmed disable must land in_app as disabled"
    );
    assert!(
        body["allDisabledWarning"].is_string(),
        "S11: disabling the last channel must surface an allDisabledWarning on the response"
    );

    // And it must persist — a re-read shows every channel disabled.
    let get = app
        .get("/api/v1/users/me/notification-preferences")
        .bearer(&access_token)
        .header("X-Tenant-ID", &org.to_string())
        .build();
    let body = app.execute(get).await.json_value();
    let all_disabled = body["preferences"]
        .as_array()
        .expect("preferences array")
        .iter()
        .all(|p| p["enabled"] == json!(false));
    assert!(
        all_disabled,
        "S11: after a confirmed disable-all every channel must be persisted as disabled"
    );
}

// ============================================================================
// S12 — publish contract (CI): a PATCH records exactly one preference.updated
// ============================================================================

/// The CI-executable mirror of S4 (issue #1376). Instead of a live Redis, the
/// app is built with a `PreferenceEventRecorder` that captures every event the
/// handler would publish. A successful PATCH must record **exactly one**
/// `preference.updated` event, on `notifications:{user_id}`, with the
/// `{channel, enabled}` payload echoing the patched channel and its new state —
/// proving the publish leg fires with the right channel, event type, and
/// payload shape. A regression that drops the publish or changes the channel
/// name / payload would now fail in CI rather than silently passing.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn patch_publishes_preference_updated_event(pool: PgPool) {
    let (app, recorder) = TestApp::with_recording_pubsub(pool.clone()).await;
    let user = TestUser::new();
    let (access_token, org) = create_authenticated_user_with_org(&app, &user, "s12").await;

    // Resolve the user's id so we can assert the target channel name.
    let user_id: uuid::Uuid = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("lookup user id");

    // Disable email — push + in_app stay enabled, so this is not a disable-all.
    let req = app
        .patch("/api/v1/users/me/notification-preferences/email")
        .bearer(&access_token)
        .header("X-Tenant-ID", &org.to_string())
        .json(json!({ "enabled": false, "confirmDisableAll": false }))
        .build();
    app.execute(req).await.assert_status(StatusCode::OK);

    let events = recorder.drain();
    assert_eq!(
        events.len(),
        1,
        "S12: a successful PATCH must record exactly one preference.updated event (got {})",
        events.len()
    );
    let event = &events[0];
    assert_eq!(
        event.channel,
        format!("notifications:{user_id}"),
        "S12: the event must target the user's notifications channel"
    );
    assert_eq!(
        event.event_type, "preference.updated",
        "S12: the event type must be preference.updated"
    );
    assert_eq!(
        event.payload["channel"],
        json!("email"),
        "S12: payload.channel must echo the patched channel"
    );
    assert_eq!(
        event.payload["enabled"],
        json!(false),
        "S12: payload.enabled must echo the new state"
    );
}

// ============================================================================
// S13 — publish contract (CI): a 409-rejected disable-all records nothing
// ============================================================================

/// The CI-executable mirror of S3's intent at the publish layer (issue #1376).
/// The publish point sits *after* the "would disable all" guard, so an
/// unconfirmed last-channel disable that is rejected with 409 must never reach
/// the recorder: zero `preference.updated` events. This proves a rejected
/// change can never emit a spurious realtime event — assertable in CI without
/// Redis.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn rejected_disable_all_records_no_event(pool: PgPool) {
    let (app, recorder) = TestApp::with_recording_pubsub(pool.clone()).await;
    let user = TestUser::new();
    let (access_token, org) = create_authenticated_user_with_org(&app, &user, "s13").await;

    // Disable email + push (each a legitimate publish) so in_app is the only
    // enabled channel.
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

    // Drain the two legitimate publishes so the next assertion isolates the
    // rejected attempt.
    let pre = recorder.drain();
    assert_eq!(
        pre.len(),
        2,
        "S13: the two successful disables must each record one event (got {})",
        pre.len()
    );

    // Disabling the last enabled channel without confirmation must 409 — and
    // must NOT record any event.
    let req = app
        .patch("/api/v1/users/me/notification-preferences/in_app")
        .bearer(&access_token)
        .header("X-Tenant-ID", &org.to_string())
        .json(json!({ "enabled": false, "confirmDisableAll": false }))
        .build();
    app.execute(req).await.assert_status(StatusCode::CONFLICT);

    let post = recorder.drain();
    assert!(
        post.is_empty(),
        "S13: a 409-rejected disable-all must record no preference.updated event (got {})",
        post.len()
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
    let (access_token, org) = create_authenticated_user_with_org(&app, &user, "s4").await;

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

// ============================================================================
// S12 — (CI Coverage) preference update publishes realtime event via in-memory
// ============================================================================

/// CI-runnable version of S4: exercises the publish leg using an in-memory
/// broker instead of a real Redis. Proves the event is published to the
/// correct channel with the correct payload (Story 1376).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn preference_update_publishes_realtime_event_in_memory(pool: PgPool) {
    use integrations::{PubSubService, InMemoryBroker};
    use std::sync::Arc;
    use tokio::time::{timeout, Duration};

    // Shared in-memory broker for publisher and observer.
    let broker = Arc::new(InMemoryBroker::new());

    // Build a TestApp with the in-memory pubsub service.
    let app = {
        use api_server::services::{EmailService, JwtService};
        use api_server::state::AppState;

        let config = common::TestConfig::default();
        let _ = TestApp::new(pool.clone()).await;

        let email_service = EmailService::new(config.base_url.clone(), config.email_enabled);
        let jwt_service = JwtService::new(&config.jwt_secret).unwrap();
        let tenant_cache = Arc::new(api_core::middleware::TenantResolutionCache::new(
            300, 30, 10_000,
        ));
        let tenant_rate_limiters =
            Arc::new(api_core::middleware::TenantRateLimiterSet::new());
        let state = AppState::new(
            pool.clone(),
            email_service,
            jwt_service,
            tenant_cache,
            tenant_rate_limiters,
        )
        .with_pubsub(PubSubService::with_broker(broker.clone()));

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
    let (access_token, org) = create_authenticated_user_with_org(&app, &user, "s12").await;

    // Resolve user id.
    let user_id: uuid::Uuid = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("lookup user id");

    // Independent observer using the SAME broker but a DIFFERENT instance_id
    // (PubSubService::with_broker generates a new instance_id).
    let observer = PubSubService::with_broker(broker.clone());
    let mut rx = observer
        .subscribe(&format!("notifications:{user_id}"))
        .await
        .expect("subscribe");

    // Disable the email channel.
    let session = app.session(access_token, org);
    let req = session
        .patch("/api/v1/users/me/notification-preferences/email")
        .json(json!({ "enabled": false, "confirmDisableAll": false }))
        .build();
    app.execute(req).await.assert_status(StatusCode::OK);

    // The observer must receive the event.
    let received = timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("preference.updated must arrive")
        .expect("message should be present");

    assert_eq!(received.event_type, "preference.updated");
    assert_eq!(received.payload["channel"], json!("email"));
    assert_eq!(received.payload["enabled"], json!(false));
}
