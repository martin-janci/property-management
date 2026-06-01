// backend/servers/deploy-server/src/infra/audit.rs
use crate::auth::{ApiKeyValidator, OidcValidator};
use crate::infra::Store;
use axum::{
    body::Body,
    extract::{Request, State},
    http::{header, HeaderValue, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone)]
pub struct AuthState {
    pub api_keys: Arc<ApiKeyValidator>,
    pub oidc: Arc<OidcValidator>,
    pub store: Arc<Store>,
}

#[derive(Clone, Debug)]
pub struct CallerIdentity {
    pub kind: String,
    pub id: String,
    pub scopes: Vec<String>,
}

impl CallerIdentity {
    /// True if the caller has the requested scope, or holds the `*` wildcard.
    pub fn has_scope(&self, required: &str) -> bool {
        self.scopes.iter().any(|s| s == required || s == "*")
    }

    /// Returns Forbidden if the caller does not hold the required scope.
    /// Used to fail-closed at the start of every state-mutating handler.
    pub fn require_scope(&self, scope: &str) -> crate::Result<()> {
        if self.has_scope(scope) {
            Ok(())
        } else {
            Err(crate::DeployError::Forbidden(format!(
                // P1-04: join() instead of {:?} to avoid Debug format in error strings.
                "missing required scope: {scope}; caller {}:{} has scopes [{}]",
                self.kind,
                self.id,
                self.scopes.join(", ")
            )))
        }
    }
}

pub async fn auth_and_audit(
    State(state): State<AuthState>,
    mut req: Request,
    next: Next,
) -> Response {
    let started = Instant::now();
    let endpoint = format!("{} {}", req.method(), req.uri().path());

    // Skip auth for /health
    if req.uri().path() == "/health" {
        return next.run(req).await;
    }

    let token = match extract_bearer(&req) {
        Some(t) => t,
        None => {
            let _ = state
                .store
                .record_audit(
                    "unauth",
                    "-",
                    &endpoint,
                    None,
                    "error:missing_bearer",
                    started.elapsed().as_millis() as i64,
                )
                .await;
            return error_resp(StatusCode::UNAUTHORIZED, "missing bearer");
        }
    };

    // Try API key first (fast), then OIDC (slower).
    let identity = if let Some((name, scopes)) = state.api_keys.validate(&token) {
        CallerIdentity {
            kind: "api_key".into(),
            id: name.into(),
            scopes: scopes.to_vec(),
        }
    } else {
        match state.oidc.validate(&token).await {
            Ok(claims) => {
                let scopes = derive_oidc_scopes(&claims.git_ref);
                CallerIdentity {
                    kind: "oidc".into(),
                    id: format!("{}@{}", claims.repository, claims.git_ref),
                    scopes,
                }
            }
            Err(e) => {
                let _ = state
                    .store
                    .record_audit(
                        "unauth",
                        "-",
                        &endpoint,
                        None,
                        &format!("error:{e}"),
                        started.elapsed().as_millis() as i64,
                    )
                    .await;
                return error_resp(StatusCode::UNAUTHORIZED, &format!("auth failed: {e}"));
            }
        }
    };

    req.extensions_mut().insert(identity.clone());

    let resp = next.run(req).await;
    let status = resp.status();
    let result = if status.is_success() {
        "ok".to_string()
    } else {
        format!("error:{}", status.as_u16())
    };
    let _ = state
        .store
        .record_audit(
            &identity.kind,
            &identity.id,
            &endpoint,
            None,
            &result,
            started.elapsed().as_millis() as i64,
        )
        .await;
    resp
}

fn extract_bearer(req: &Request) -> Option<String> {
    let val: &HeaderValue = req.headers().get(header::AUTHORIZATION)?;
    let s = val.to_str().ok()?;
    s.strip_prefix("Bearer ").map(str::to_string)
}

fn error_resp(status: StatusCode, msg: &str) -> Response {
    let body = serde_json::to_vec(&serde_json::json!({"error": msg})).unwrap();
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap()
}

/// Derive the scope set granted to a GitHub Actions OIDC caller from its
/// `git_ref` claim. Intentionally simple — each CI workflow's ref pattern maps
/// to the minimum scopes that workflow needs (see docker-build.yml,
/// docker-frontend.yml, release.yml, worktree open/close workflows).
///
/// `refs/heads/dev` is granted `release:deploy` so the dev→staging auto-deploy
/// (`trigger-deploy` in docker-build.yml / docker-frontend.yml) is authorized,
/// mirroring `refs/heads/main`→prod. Without this, dev pushes authenticated
/// (dev is in `allowed_refs`) but received an empty scope set, so
/// `require_scope("release:deploy")` rejected every dev auto-deploy with 403.
///
/// NOTE: `release:deploy` is target-agnostic, so a `dev` token could request
/// `target: prod` (same latent property `main` already has). Tightening to
/// target-scoped permissions is tracked as a follow-up.
fn derive_oidc_scopes(git_ref: &str) -> Vec<String> {
    if git_ref == "refs/heads/main" || git_ref == "refs/heads/dev" {
        vec!["release:deploy".to_string()]
    } else if git_ref.starts_with("refs/tags/v") {
        vec!["release:register".to_string()]
    } else if git_ref.starts_with("refs/heads/feature/") {
        vec!["worktree:open".to_string(), "worktree:close".to_string()]
    } else {
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::derive_oidc_scopes;

    #[test]
    fn main_and_dev_get_release_deploy() {
        assert_eq!(
            derive_oidc_scopes("refs/heads/main"),
            vec!["release:deploy".to_string()]
        );
        // Regression (deploy-403): dev previously fell through to an empty
        // scope set, so docker-build.yml/docker-frontend.yml trigger-deploy
        // always 403'd on the dev→staging auto-deploy.
        assert_eq!(
            derive_oidc_scopes("refs/heads/dev"),
            vec!["release:deploy".to_string()]
        );
    }

    #[test]
    fn version_tags_get_release_register() {
        assert_eq!(
            derive_oidc_scopes("refs/tags/v0.3.0"),
            vec!["release:register".to_string()]
        );
        assert_eq!(
            derive_oidc_scopes("refs/tags/v1.2.3"),
            vec!["release:register".to_string()]
        );
    }

    #[test]
    fn feature_branches_get_worktree_scopes() {
        assert_eq!(
            derive_oidc_scopes("refs/heads/feature/epic-1-auth"),
            vec!["worktree:open".to_string(), "worktree:close".to_string()]
        );
    }

    #[test]
    fn unknown_refs_get_no_scope() {
        assert!(derive_oidc_scopes("refs/heads/random").is_empty());
        assert!(derive_oidc_scopes("refs/heads/bugfix/x").is_empty());
        assert!(derive_oidc_scopes("refs/tags/not-a-version").is_empty());
    }
}
