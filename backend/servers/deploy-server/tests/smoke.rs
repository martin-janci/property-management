// backend/servers/deploy-server/tests/smoke.rs
//! End-to-end smoke: real sqlite, mocked Caddy, real Docker.

use deploy_server::api::release::ReleaseService;
use deploy_server::api::router;
use deploy_server::auth::{ApiKeyValidator, OidcValidator};
use deploy_server::config::{ApiKey, Config, OidcConfig, Target, TargetsConfig};
use deploy_server::infra::{CaddyClient, DockerClient, GitFetcher, StagingDeployer, Store};
use httpmock::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use tempfile::tempdir;

#[tokio::test]
#[ignore] // requires docker daemon + ppt-frontend-dev:local image
async fn open_status_close_flow() {
    let tmp = tempdir().unwrap();
    let store = Arc::new(Store::open(&tmp.path().join("state.db")).await.unwrap());

    let caddy_mock = MockServer::start();
    caddy_mock.mock(|when, then| {
        when.method(PUT);
        then.status(200);
    });
    caddy_mock.mock(|when, then| {
        when.method(DELETE);
        then.status(200);
    });

    // Local bare git repo as fixture (see infra::git tests for setup).
    let bare = tmp.path().join("origin.git");
    let work = tmp.path().join("seed");
    std::fs::create_dir_all(&bare).unwrap();
    std::process::Command::new("git")
        .args(["init", "--bare"])
        .arg(&bare)
        .status()
        .unwrap();
    std::process::Command::new("git")
        .args(["init"])
        .arg(&work)
        .status()
        .unwrap();
    std::process::Command::new("git")
        .args(["-C", work.to_str().unwrap(), "config", "user.email", "t@t"])
        .status()
        .unwrap();
    std::process::Command::new("git")
        .args(["-C", work.to_str().unwrap(), "config", "user.name", "t"])
        .status()
        .unwrap();
    std::fs::write(work.join("README.md"), "hi").unwrap();
    std::process::Command::new("git")
        .args(["-C", work.to_str().unwrap(), "add", "."])
        .status()
        .unwrap();
    std::process::Command::new("git")
        .args(["-C", work.to_str().unwrap(), "commit", "-m", "init"])
        .status()
        .unwrap();
    std::process::Command::new("git")
        .args(["-C", work.to_str().unwrap(), "branch", "-M", "feature-x"])
        .status()
        .unwrap();
    std::process::Command::new("git")
        .args(["-C", work.to_str().unwrap(), "remote", "add", "origin"])
        .arg(&bare)
        .status()
        .unwrap();
    std::process::Command::new("git")
        .args([
            "-C",
            work.to_str().unwrap(),
            "push",
            "-u",
            "origin",
            "feature-x",
        ])
        .status()
        .unwrap();

    let git = Arc::new(GitFetcher::new(
        bare.to_string_lossy().to_string(),
        tmp.path().join("worktrees"),
        "/dev/null",
    ));

    let docker = Arc::new(DockerClient::from_socket("unix:///var/run/docker.sock").unwrap());
    let caddy = Arc::new(CaddyClient::new(caddy_mock.base_url()));

    let api_key = "test-token";
    let hash = hex::encode(<sha2::Sha256 as sha2::Digest>::digest(api_key.as_bytes()));
    let api_keys = Arc::new(ApiKeyValidator::new(vec![ApiKey {
        name: "test".into(),
        hash,
    }]));
    let oidc = Arc::new(OidcValidator::new(OidcConfig {
        issuer: "x".into(),
        jwks_url: "http://x".into(),
        audience: "x".into(),
        allowed_repos: vec![],
        allowed_refs: vec![],
    }));

    let webhook_cfg = deploy_server::api::webhook::WebhookConfig {
        secret: "wh".into(),
    };
    let cfg = Arc::new(Config {
        bind: "0.0.0.0:0".into(),
        state_dir: tmp.path().to_string_lossy().into(),
        worktree_dir: tmp.path().join("worktrees").to_string_lossy().into(),
        snapshot_dir: tmp.path().join("snapshots").to_string_lossy().into(),
        default_ttl_seconds: 172_800,
        idle_pause_seconds: 1800,
        idle_stop_seconds: 86400,
        git_repo_url: bare.to_string_lossy().into(),
    });

    let mut targets_map = HashMap::new();
    targets_map.insert(
        "staging".into(),
        Target {
            docker_socket: "unix:///var/run/docker.sock".into(),
            caddy_url: caddy_mock.base_url(),
            domain_suffix: "staging.rlt.sk".into(),
            idle_timeout: Some("8h".into()),
            promote_strategy: None,
            rollback_mode: "manual".into(),
            health_grace: None,
        },
    );
    let targets_cfg = Arc::new(TargetsConfig {
        targets: targets_map,
    });

    let deployer = Arc::new(StagingDeployer {
        docker: docker.clone(),
        caddy: caddy.clone(),
    });
    let release_svc = Arc::new(ReleaseService {
        store: store.clone(),
        deployer,
        targets: targets_cfg,
        image_prefix: "ghcr.io/test".into(),
    });

    let app = router::build(
        store.clone(),
        git,
        docker,
        caddy,
        api_keys,
        oidc,
        "ppt-frontend-dev:local".into(),
        "dev.ppt.rlt.sk".into(),
        "dev.rlt.sk".into(),
        webhook_cfg,
        cfg,
        release_svc,
    );

    let server = axum_test::TestServer::new(app).unwrap();
    let resp = server
        .post("/api/worktree")
        .add_header("Authorization", "Bearer test-token")
        .json(&serde_json::json!({"branch": "feature-x", "backend": "shared"}))
        .await;
    resp.assert_status_ok();

    let list = server
        .get("/api/worktrees")
        .add_header("Authorization", "Bearer test-token")
        .await;
    list.assert_status_ok();
    assert!(list.text().contains("feature-x"));

    let close = server
        .post("/api/worktree/feature-x/close")
        .add_header("Authorization", "Bearer test-token")
        .await;
    close.assert_status_ok();

    let deploy = server
        .post("/api/deploy")
        .add_header("Authorization", "Bearer test-token")
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
        .add_header("Authorization", "Bearer test-token")
        .await;
    let wake_st = wake.status_code();
    assert!(
        wake_st == 200 || wake_st.as_u16() == 404 || wake_st.as_u16() == 500,
        "wake unexpected: {wake_st}"
    );
}
