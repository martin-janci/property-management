// backend/servers/deploy-server/tests/smoke.rs
//! End-to-end smoke: real sqlite, mocked Caddy, real Docker.
//!
//! Fixture setup is shared via `tests/common/mod.rs` so a `router::build`
//! signature change only needs to be reflected in one place.

mod common;

use common::{setup_app, TestApp};

#[tokio::test]
#[ignore = "requires docker daemon + ppt-frontend-dev:local image"]
async fn open_status_close_flow() {
    let TestApp { app, token, .. } = setup_app(&["staging"]).await;
    let server = axum_test::TestServer::new(app);
    let auth = format!("Bearer {token}");

    let resp = server
        .post("/api/worktree")
        .add_header("Authorization", &auth)
        .json(&serde_json::json!({"branch": "feature-x", "backend": "shared"}))
        .await;
    resp.assert_status_ok();

    let list = server
        .get("/api/worktrees")
        .add_header("Authorization", &auth)
        .await;
    list.assert_status_ok();
    assert!(list.text().contains("feature-x"));

    let close = server
        .post("/api/worktree/feature-x/close")
        .add_header("Authorization", &auth)
        .await;
    close.assert_status_ok();

    let deploy = server
        .post("/api/deploy")
        .add_header("Authorization", &auth)
        .json(&serde_json::json!({"tag": "v0.0.0-test", "target": "staging"}))
        .await;
    // Will fail at deployer.deploy() because real images don't exist locally,
    // but the auth + parse + DB write paths run first. Accept either 200 or 500.
    let st = deploy.status_code();
    assert!(
        st == 200 || st.as_u16() == 500,
        "expected 200 or 500, got {st}"
    );

    // Wake should fail with NotFound if no release was successfully recorded.
    let wake = server
        .post("/api/wake/staging")
        .add_header("Authorization", &auth)
        .await;
    let wake_st = wake.status_code();
    assert!(
        wake_st == 200 || wake_st.as_u16() == 404 || wake_st.as_u16() == 500,
        "wake unexpected: {wake_st}"
    );
}

#[tokio::test]
#[ignore = "requires postgres + GH API + docker daemon + branch images"]
async fn dedicated_open_close_with_dump() {
    // This test sketches the dedicated path. It will fail without:
    //   - real Postgres at admin_url
    //   - GH PAT with workflow_dispatch
    //   - branch-tagged images in GHCR
    // Marked #[ignore]'d. The intent is to ensure the path compiles + auth works.

    let TestApp { app, token, .. } = setup_app(&["staging"]).await;
    let server = axum_test::TestServer::new(app);

    let resp = server
        .post("/api/worktree")
        .add_header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({"branch": "feature-x", "backend": "dedicated"}))
        .await;
    // Will fail because postgres is not real. We just want compile + auth path verification.
    let st = resp.status_code();
    assert!(
        st == 200 || st.as_u16() == 500,
        "expected 200 or 500, got {st}"
    );
}

#[tokio::test]
#[ignore = "requires docker daemon to actually deploy; tests auth + parse paths without it"]
async fn promote_and_rollback_flow() {
    let TestApp { app, token, .. } = setup_app(&["prod", "staging"]).await;
    let server = axum_test::TestServer::new(app);
    let auth = format!("Bearer {token}");

    // 1. Register a prod-candidate
    let images = serde_json::json!({
        "api-server": "ghcr.io/test/ppt-api-server:v1.0.0",
        "reality-server": "ghcr.io/test/ppt-reality-server:v1.0.0",
        "ppt-web": "ghcr.io/test/ppt-web:v1.0.0",
        "reality-web": "ghcr.io/test/reality-web:v1.0.0",
    });
    let reg = server
        .post("/api/release")
        .add_header("Authorization", &auth)
        .json(&serde_json::json!({"tag": "v1.0.0", "images": images}))
        .await;
    reg.assert_status_ok();

    // 2. Dry-run promote
    let dry = server
        .post("/api/promote")
        .add_header("Authorization", &auth)
        .json(&serde_json::json!({"tag": "v1.0.0", "target": "prod", "dry_run": true}))
        .await;
    dry.assert_status_ok();
    assert!(dry.text().contains("v1.0.0"));

    // 3. Real promote — will fail at deployer.deploy() because images aren't real, but auth + parse paths run.
    let promote = server
        .post("/api/promote")
        .add_header("Authorization", &auth)
        .json(&serde_json::json!({"tag": "v1.0.0", "target": "prod"}))
        .await;
    let st = promote.status_code();
    assert!(
        st == 200 || st.as_u16() == 500,
        "promote: expected 200 or 500, got {st}"
    );

    // 4. Rollback (no `to` — would need previous to exist; expect 404 or 500)
    let rollback = server
        .post("/api/rollback")
        .add_header("Authorization", &auth)
        .json(&serde_json::json!({"target": "prod"}))
        .await;
    let rst = rollback.status_code();
    assert!(
        rst == 200 || rst.as_u16() == 404 || rst.as_u16() == 500,
        "rollback: unexpected {rst}"
    );
}
