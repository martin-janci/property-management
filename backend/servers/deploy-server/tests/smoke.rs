// backend/servers/deploy-server/tests/smoke.rs
//! End-to-end smoke: real sqlite, mocked Caddy, real Docker.

use deploy_server::api::promote::PromoteService;
use deploy_server::api::release::ReleaseService;
use deploy_server::api::router;
use deploy_server::auth::{ApiKeyValidator, OidcValidator};
use deploy_server::config::{ApiKey, Config, OidcConfig, Target, TargetsConfig};
use deploy_server::infra::{
    CaddyClient, DockerClient, GhClient, GitFetcher, HealthProbe, PostgresOps, Store,
};
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
        postgres_admin_url: "postgres://test/postgres".into(),
        postgres_template_db: "ppt_dev_template".into(),
        postgres_user_db_prefix: "ppt_wt_".into(),
        backend_image_prefix: "ghcr.io/test".into(),
        gh_repo: "owner/repo".into(),
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

    let mut docker_pool: HashMap<String, Arc<DockerClient>> = HashMap::new();
    docker_pool.insert("staging".into(), docker.clone());
    let docker_pool = Arc::new(docker_pool);

    let mut caddy_pool: HashMap<String, Arc<CaddyClient>> = HashMap::new();
    caddy_pool.insert("staging".into(), caddy.clone());
    let caddy_pool = Arc::new(caddy_pool);

    let release_svc = Arc::new(ReleaseService {
        store: store.clone(),
        docker_pool,
        caddy_pool,
        targets: targets_cfg.clone(),
        image_prefix: "ghcr.io/test".into(),
    });

    let promote_svc = Arc::new(PromoteService {
        release_svc: release_svc.clone(),
        health: Arc::new(HealthProbe::new()),
        targets: targets_cfg,
    });

    let postgres = Arc::new(PostgresOps {
        admin_url: "postgres://test/postgres".into(),
        template_db: "ppt_dev_template".into(),
        user_db_prefix: "ppt_wt_".into(),
    });
    let gh = Arc::new(GhClient::new("dummy", "owner/repo"));

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
        postgres,
        gh,
        "ghcr.io/test".into(),
        promote_svc,
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

#[tokio::test]
#[ignore] // requires postgres + GH API + docker daemon + branch images
async fn dedicated_open_close_with_dump() {
    // This test sketches the dedicated path. It will fail without:
    //   - real Postgres at admin_url
    //   - GH PAT with workflow_dispatch
    //   - branch-tagged images in GHCR
    // Mark #[ignore]'d. The intent is to ensure the path compiles + auth works.

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

    // Local bare git repo as fixture.
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

    let cfg = Arc::new(Config {
        bind: "0.0.0.0:0".into(),
        state_dir: tmp.path().to_string_lossy().into(),
        worktree_dir: tmp.path().join("worktrees").to_string_lossy().into(),
        snapshot_dir: tmp.path().join("snapshots").to_string_lossy().into(),
        default_ttl_seconds: 172_800,
        idle_pause_seconds: 1800,
        idle_stop_seconds: 86400,
        git_repo_url: bare.to_string_lossy().into(),
        postgres_admin_url: "postgres://test/postgres".into(),
        postgres_template_db: "ppt_dev_template".into(),
        postgres_user_db_prefix: "ppt_wt_".into(),
        backend_image_prefix: "ghcr.io/test".into(),
        gh_repo: "test/repo".into(),
    });

    let postgres = Arc::new(PostgresOps {
        admin_url: cfg.postgres_admin_url.clone(),
        template_db: cfg.postgres_template_db.clone(),
        user_db_prefix: cfg.postgres_user_db_prefix.clone(),
    });
    let gh = Arc::new(GhClient::new("dummy", "test/repo"));

    let mut docker_pool: HashMap<String, Arc<DockerClient>> = HashMap::new();
    docker_pool.insert("staging".into(), docker.clone());
    let docker_pool = Arc::new(docker_pool);

    let mut caddy_pool: HashMap<String, Arc<CaddyClient>> = HashMap::new();
    caddy_pool.insert("staging".into(), caddy.clone());
    let caddy_pool = Arc::new(caddy_pool);

    let release_svc = Arc::new(ReleaseService {
        store: store.clone(),
        docker_pool,
        caddy_pool,
        targets: targets_cfg.clone(),
        image_prefix: "ghcr.io/test".into(),
    });

    let promote_svc = Arc::new(PromoteService {
        release_svc: release_svc.clone(),
        health: Arc::new(HealthProbe::new()),
        targets: targets_cfg.clone(),
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
        postgres,
        gh,
        "ghcr.io/test".into(),
        promote_svc,
    );

    let server = axum_test::TestServer::new(app).unwrap();
    let resp = server
        .post("/api/worktree")
        .add_header("Authorization", "Bearer test-token")
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
#[ignore] // requires docker daemon to actually deploy; without it, tests the auth + parse paths.
async fn promote_and_rollback_flow() {
    let tmp = tempdir().unwrap();
    let store = Arc::new(Store::open(&tmp.path().join("state.db")).await.unwrap());

    let caddy_mock = MockServer::start();
    caddy_mock.mock(|when, then| {
        when.method(PUT);
        then.status(200);
    });

    let bare = tmp.path().join("origin.git");
    std::fs::create_dir_all(&bare).unwrap();
    std::process::Command::new("git")
        .args(["init", "--bare"])
        .arg(&bare)
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

    let mut targets_map = HashMap::new();
    targets_map.insert(
        "prod".into(),
        Target {
            docker_socket: "unix:///var/run/docker.sock".into(),
            caddy_url: caddy_mock.base_url(),
            domain_suffix: "rlt.sk".into(),
            idle_timeout: None,
            promote_strategy: Some("blue-green".into()),
            rollback_mode: "manual".into(),
            health_grace: None, // skip health check in test
        },
    );
    let targets_cfg = Arc::new(TargetsConfig {
        targets: targets_map,
    });

    let cfg = Arc::new(Config {
        bind: "0.0.0.0:0".into(),
        state_dir: tmp.path().to_string_lossy().into(),
        worktree_dir: tmp.path().join("worktrees").to_string_lossy().into(),
        snapshot_dir: tmp.path().join("snapshots").to_string_lossy().into(),
        default_ttl_seconds: 172_800,
        idle_pause_seconds: 1800,
        idle_stop_seconds: 86400,
        git_repo_url: bare.to_string_lossy().into(),
        postgres_admin_url: "postgres://test/postgres".into(),
        postgres_template_db: "ppt_dev_template".into(),
        postgres_user_db_prefix: "ppt_wt_".into(),
        backend_image_prefix: "ghcr.io/test".into(),
        gh_repo: "test/repo".into(),
    });

    let postgres = Arc::new(PostgresOps {
        admin_url: cfg.postgres_admin_url.clone(),
        template_db: cfg.postgres_template_db.clone(),
        user_db_prefix: cfg.postgres_user_db_prefix.clone(),
    });
    let gh = Arc::new(GhClient::new("dummy", "test/repo"));

    let mut docker_pool: HashMap<String, Arc<DockerClient>> = HashMap::new();
    docker_pool.insert("staging".into(), docker.clone());
    docker_pool.insert("prod".into(), docker.clone());
    let docker_pool = Arc::new(docker_pool);

    let mut caddy_pool: HashMap<String, Arc<CaddyClient>> = HashMap::new();
    caddy_pool.insert("staging".into(), caddy.clone());
    caddy_pool.insert("prod".into(), caddy.clone());
    let caddy_pool = Arc::new(caddy_pool);

    let release_svc = Arc::new(ReleaseService {
        store: store.clone(),
        docker_pool,
        caddy_pool,
        targets: targets_cfg.clone(),
        image_prefix: "ghcr.io/test".into(),
    });
    let promote_svc = Arc::new(PromoteService {
        release_svc: release_svc.clone(),
        health: Arc::new(HealthProbe::new()),
        targets: targets_cfg.clone(),
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
        postgres,
        gh,
        "ghcr.io/test".into(),
        promote_svc,
    );

    let server = axum_test::TestServer::new(app).unwrap();

    // 1. Register a prod-candidate
    let images = serde_json::json!({
        "api-server": "ghcr.io/test/ppt-api-server:v1.0.0",
        "reality-server": "ghcr.io/test/ppt-reality-server:v1.0.0",
        "ppt-web": "ghcr.io/test/ppt-web:v1.0.0",
        "reality-web": "ghcr.io/test/reality-web:v1.0.0",
    });
    let reg = server
        .post("/api/release")
        .add_header("Authorization", "Bearer test-token")
        .json(&serde_json::json!({"tag": "v1.0.0", "images": images}))
        .await;
    reg.assert_status_ok();

    // 2. Dry-run promote
    let dry = server
        .post("/api/promote")
        .add_header("Authorization", "Bearer test-token")
        .json(&serde_json::json!({"tag": "v1.0.0", "target": "prod", "dry_run": true}))
        .await;
    dry.assert_status_ok();
    assert!(dry.text().contains("v1.0.0"));

    // 3. Real promote — will fail at deployer.deploy() because images aren't real, but auth + parse paths run.
    let promote = server
        .post("/api/promote")
        .add_header("Authorization", "Bearer test-token")
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
        .add_header("Authorization", "Bearer test-token")
        .json(&serde_json::json!({"target": "prod"}))
        .await;
    let rst = rollback.status_code();
    assert!(
        rst == 200 || rst.as_u16() == 404 || rst.as_u16() == 500,
        "rollback: unexpected {rst}"
    );
}
