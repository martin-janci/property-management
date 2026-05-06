// backend/servers/deploy-server/src/api/webhook.rs
use crate::api::worktree::WorktreeService;
use crate::infra::git::sanitize;
use crate::{DeployError, Result};
use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;
use std::sync::Arc;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone)]
pub struct WebhookConfig {
    pub secret: String,
}

#[derive(Debug, Deserialize)]
pub struct WebhookPayload {
    pub action: Option<String>,
    pub pull_request: Option<PullRequest>,
}

#[derive(Debug, Deserialize)]
pub struct PullRequest {
    pub head: PrRef,
}

#[derive(Debug, Deserialize)]
pub struct PrRef {
    #[serde(rename = "ref")]
    pub git_ref: String,
}

pub async fn handler(
    State((svc, cfg)): State<(Arc<WorktreeService>, WebhookConfig)>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<Json<serde_json::Value>> {
    verify_signature(&headers, &body, &cfg.secret)?;
    let payload: WebhookPayload = serde_json::from_slice(&body)
        .map_err(|e| DeployError::BadRequest(format!("bad json: {e}")))?;

    if payload.action.as_deref() == Some("closed") {
        if let Some(pr) = &payload.pull_request {
            let name = sanitize(&pr.head.git_ref);
            // Best-effort close; ignore not-found.
            if svc.store.get_worktree(&name).await?.is_some() {
                let path = axum::extract::Path(name.clone());
                let _ = crate::api::worktree::close_handler(State(svc.clone()), path).await;
            }
        }
    }
    Ok(Json(serde_json::json!({"ok": true})))
}

fn verify_signature(headers: &HeaderMap, body: &[u8], secret: &str) -> Result<()> {
    let sig = headers
        .get("X-Hub-Signature-256")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("sha256="))
        .ok_or_else(|| DeployError::Unauthorized("missing X-Hub-Signature-256".into()))?;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| DeployError::Internal(format!("hmac key: {e}")))?;
    mac.update(body);
    let expected = hex::encode(mac.finalize().into_bytes());
    if !constant_time_eq(sig.as_bytes(), expected.as_bytes()) {
        return Err(DeployError::Unauthorized("bad signature".into()));
    }
    Ok(())
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_round_trip() {
        let secret = "topsecret";
        let body = b"hello";
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let sig = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));

        let mut headers = HeaderMap::new();
        headers.insert("X-Hub-Signature-256", sig.parse().unwrap());

        verify_signature(&headers, body, secret).unwrap();
    }

    #[test]
    fn bad_signature_rejected() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Hub-Signature-256", "sha256=00".parse().unwrap());
        assert!(verify_signature(&headers, b"x", "k").is_err());
    }
}
