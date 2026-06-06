// backend/servers/deploy-server/tests/common/mod.rs
//! Shared test fixtures for smoke tests.
//!
//! `setup_app(targets)` returns a fully wired axum app + an authorized bearer token,
//! ready for `axum_test::TestServer`. The targets argument lets each test customize
//! which targets exist in the config (e.g. only "staging" vs both "staging" + "prod").

#![allow(dead_code)] // helpers may not be used by every test binary

use deploy_server::api::promote::PromoteService;
use deploy_server::api::release::ReleaseService;
use deploy_server::api::router;
use deploy_server::auth::{ApiKeyValidator, OidcValidator};
use deploy_server::config::{ApiKey, Config, OidcConfig, Target, TargetsConfig};
use deploy_server::infra::{
    CaddyClient, DockerClient, GhClient, GitFetcher, HealthProbe, PostgresOps, Store,
    WorktreeLockRegistry,
};
use httpmock::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use tempfile::TempDir;

pub struct TestApp {
    pub app: axum::Router,
    pub token: String,
    pub _tmp: TempDir,
    pub _caddy: MockServer,
}

/// Build a fully-wired router for smoke tests.
///
/// `target_names` controls which targets exist in `TargetsConfig` (and the
/// docker/caddy pools). A local bare git repo with a `feature-x` branch is
/// always set up so worktree tests can succeed.
pub async fn setup_app(target_names: &[&str]) -> TestApp {
    let tmp = tempfile::tempdir().unwrap();
    let store = Arc::new(Store::open(&tmp.path().join("state.db")).await.unwrap());

    // Mock Caddy admin: accept all PUT/DELETE.
    let caddy_mock = MockServer::start();
    caddy_mock.mock(|when, then| {
        when.method(PUT);
        then.status(200);
    });
    caddy_mock.mock(|when, then| {
        when.method(DELETE);
        then.status(200);
    });

    // Local bare git repo as fixture, with a `feature-x` branch pushed.
    //
    // Each git invocation goes through `run_git_assert` which captures stderr
    // and panics with a helpful message if the command fails. Without that, a
    // missing git binary, permission issue, or stale fixture state would let
    // setup continue silently and trip downstream assertions later in the
    // test with no breadcrumbs to the actual cause.
    let bare = tmp.path().join("origin.git");
    let work = tmp.path().join("seed");
    std::fs::create_dir_all(&bare).unwrap();
    let work_str = work.to_str().unwrap();
    let bare_str = bare.to_str().unwrap();
    run_git_assert(&["init", "--bare", bare_str]);
    run_git_assert(&["init", work_str]);
    run_git_assert(&["-C", work_str, "config", "user.email", "t@t"]);
    run_git_assert(&["-C", work_str, "config", "user.name", "t"]);
    // Disable commit signing for the fixture repo. A developer (or CI) machine
    // with `commit.gpgsign=true` in global config would otherwise make the
    // fixture `git commit` invoke a signing backend and fail with a non-zero
    // exit. Mirrors the per-test fixture in `infra/git.rs`.
    run_git_assert(&["-C", work_str, "config", "commit.gpgsign", "false"]);
    std::fs::write(work.join("README.md"), "hi").unwrap();
    run_git_assert(&["-C", work_str, "add", "."]);
    run_git_assert(&["-C", work_str, "commit", "-m", "init"]);
    run_git_assert(&["-C", work_str, "branch", "-M", "feature-x"]);
    run_git_assert(&["-C", work_str, "remote", "add", "origin", bare_str]);
    run_git_assert(&["-C", work_str, "push", "-u", "origin", "feature-x"]);

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
        scopes: vec!["*".into()],
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

    // Build targets map with the requested names.
    let mut targets_map = HashMap::new();
    for name in target_names {
        let target = Target {
            docker_socket: "unix:///var/run/docker.sock".into(),
            caddy_url: caddy_mock.base_url(),
            // Test apexes mirror the conventions used on onyx:
            //   staging → reality_apex="staging.rlt.sk", ppt_apex="staging.ppt.rlt.sk"
            //   prod    → reality_apex="prod.rlt.sk",    ppt_apex="prod.ppt.rlt.sk"
            // (prod uses a "prod." prefix in tests to keep target names
            // distinguishable; real onyx prod target uses bare "rlt.sk" /
            // "ppt.rlt.sk".)
            reality_apex: format!("{name}.rlt.sk"),
            ppt_apex: format!("{name}.ppt.rlt.sk"),
            idle_timeout: Some("8h".into()),
            promote_strategy: Some("blue-green".into()),
            rollback_mode: "manual".into(),
            health_grace: None,
        };
        targets_map.insert(name.to_string(), target);
    }
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

    // Per-target docker + caddy pools.
    let mut docker_pool: HashMap<String, Arc<DockerClient>> = HashMap::new();
    let mut caddy_pool: HashMap<String, Arc<CaddyClient>> = HashMap::new();
    for name in target_names {
        docker_pool.insert(name.to_string(), docker.clone());
        caddy_pool.insert(name.to_string(), caddy.clone());
    }
    let docker_pool = Arc::new(docker_pool);
    let caddy_pool = Arc::new(caddy_pool);

    let release_svc = Arc::new(ReleaseService {
        store: store.clone(),
        docker_pool: docker_pool.clone(),
        caddy_pool: caddy_pool.clone(),
        targets: targets_cfg.clone(),
        image_prefix: "ghcr.io/test".into(),
        // Reuse cfg.postgres_admin_url so PostgresOps and the preflight DB
        // check (read via release_svc.postgres_admin_url) agree on which DB
        // they're talking to. A divergence here let prior fixtures pass while
        // any test that exercised /api/deploy hit a different URL than the
        // rest of the surface.
        postgres_admin_url: cfg.postgres_admin_url.clone(),
    });
    let promote_svc = Arc::new(PromoteService {
        release_svc: release_svc.clone(),
        health: Arc::new(HealthProbe::new()),
        targets: targets_cfg.clone(),
    });
    let worktree_locks = Arc::new(WorktreeLockRegistry::new());
    let bootstrap_svc = Arc::new(deploy_server::api::bootstrap::BootstrapService {
        targets: targets_cfg.clone(),
        postgres: postgres.clone(),
        release_svc: release_svc.clone(),
    });

    let app = router::build(
        store,
        git,
        docker,
        caddy,
        api_keys,
        oidc,
        "ppt-frontend-dev:local".into(),
        "dev.ppt.rlt.sk".into(),
        "dev.rlt.sk".into(),
        "api-server:8080".into(),
        "reality-server:8081".into(),
        webhook_cfg,
        cfg,
        release_svc,
        postgres,
        gh,
        "ghcr.io/test".into(),
        promote_svc,
        worktree_locks,
        bootstrap_svc,
    );

    TestApp {
        app,
        token: api_key.into(),
        _tmp: tmp,
        _caddy: caddy_mock,
    }
}

/// Run a `git` subcommand and panic with stdout+stderr if it fails.
///
/// Used by the test fixture's local-repo bootstrap. Previously each call was
/// `Command::status().unwrap()` which only catches *spawn* failures (e.g. git
/// not on PATH) — a non-zero exit (rejected commit, missing config, etc.) was
/// silently ignored and the test continued with a broken fixture, surfacing
/// later as an obscure clone failure deep inside `GitFetcher`.
fn run_git_assert(args: &[&str]) {
    let output = std::process::Command::new("git")
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn git {args:?}: {e}"));
    if !output.status.success() {
        panic!(
            "git {args:?} failed with status {:?}\n--- stdout ---\n{}\n--- stderr ---\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }
}
