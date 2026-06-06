// backend/servers/deploy-server/tests/worktree_path_validation.rs
//! Regression for issue #769 (Low, finding 6): worktree path params on the
//! `get` / `close` / `logs` endpoints must be run through `validate_alias_strict`
//! at the API boundary — the same strict rules `open_handler` already applies to
//! a freshly-created alias.
//!
//! Before the fix the `{name}` segment flowed straight into the store lookup,
//! the Caddy host strings, and (on the dedicated close path) Postgres DB-name
//! reconstruction without any character allowlist. An out-of-charset name like
//! `a/b` or `a";DROP` fell through to a 404 (NotFound) instead of being rejected
//! with a 400 (BadRequest) up front.
//!
//! These cases reach validation BEFORE any Docker / Postgres call, so they do
//! not require a live docker daemon (unlike the happy-path smoke test).

mod common;

use common::{setup_app, TestApp};

/// Names that must be rejected at the boundary with HTTP 400.
const MALICIOUS_NAMES: &[&str] = &[
    "a%22%3BDROP%20DATABASE%20%22x%22%3B--", // a";DROP DATABASE "x";-- (url-encoded)
    "-foo",                                  // leading dash (option-injection shape)
    "ab%20cd",                               // contains a space
    "a..b",                                  // path-traversal-ish dots
];

#[tokio::test]
async fn get_handler_rejects_malicious_path_param() {
    let TestApp { app, token, .. } = setup_app(&["staging"]).await;
    let server = axum_test::TestServer::new(app);
    let auth = format!("Bearer {token}");

    for name in MALICIOUS_NAMES {
        let resp = server
            .get(&format!("/api/worktree/{name}"))
            .add_header("Authorization", &auth)
            .await;
        assert_eq!(
            resp.status_code(),
            400,
            "GET /api/worktree/{name} should be rejected with 400 (got {}); \
             body: {}",
            resp.status_code(),
            resp.text()
        );
    }
}

#[tokio::test]
async fn close_handler_rejects_malicious_path_param() {
    let TestApp { app, token, .. } = setup_app(&["staging"]).await;
    let server = axum_test::TestServer::new(app);
    let auth = format!("Bearer {token}");

    for name in MALICIOUS_NAMES {
        let resp = server
            .post(&format!("/api/worktree/{name}/close"))
            .add_header("Authorization", &auth)
            .await;
        assert_eq!(
            resp.status_code(),
            400,
            "POST /api/worktree/{name}/close should be rejected with 400 (got {}); \
             body: {}",
            resp.status_code(),
            resp.text()
        );
    }
}

#[tokio::test]
async fn logs_handler_rejects_malicious_path_param() {
    let TestApp { app, token, .. } = setup_app(&["staging"]).await;
    let server = axum_test::TestServer::new(app);
    let auth = format!("Bearer {token}");

    for name in MALICIOUS_NAMES {
        let resp = server
            .get(&format!("/api/logs/{name}"))
            .add_header("Authorization", &auth)
            .await;
        assert_eq!(
            resp.status_code(),
            400,
            "GET /api/logs/{name} should be rejected with 400 (got {}); \
             body: {}",
            resp.status_code(),
            resp.text()
        );
    }
}

/// A well-formed name must NOT be rejected by the validator — it passes the
/// allowlist and proceeds to the normal lookup path (which then resolves to
/// not-found / etc., never a 400). This guards against the validator
/// over-rejecting legitimate aliases.
#[tokio::test]
async fn get_handler_valid_name_is_not_rejected_by_validator() {
    let TestApp { app, token, .. } = setup_app(&["staging"]).await;
    let server = axum_test::TestServer::new(app);
    let auth = format!("Bearer {token}");

    let resp = server
        .get("/api/worktree/does-not-exist")
        .add_header("Authorization", &auth)
        .await;
    assert_ne!(
        resp.status_code(),
        400,
        "a valid alias must not be rejected as a bad request by the path validator"
    );
}
