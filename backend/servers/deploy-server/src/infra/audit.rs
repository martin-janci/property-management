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
                "missing required scope: {scope}; caller {}:{} has scopes {:?}",
                self.kind, self.id, self.scopes
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
                // OIDC scope derivation is intentionally simple — we map common GitHub
                // ref patterns to the minimum scopes needed by the matching CI workflow.
                // Refine when CI patterns stabilize.
                let mut scopes = vec![];
                if claims.git_ref == "refs/heads/main" {
                    scopes.push("release:deploy".to_string());
                } else if claims.git_ref.starts_with("refs/tags/v") {
                    scopes.push("release:register".to_string());
                } else if claims.git_ref.starts_with("refs/heads/feature/") {
                    scopes.push("worktree:open".to_string());
                    scopes.push("worktree:close".to_string());
                }
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
