// backend/servers/deploy-server/src/api/webhook.rs
use crate::api::worktree::WorktreeService;
use crate::infra::git::sanitize;
use crate::infra::CallerIdentity;
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
    let started = std::time::Instant::now();
    if let Err(e) = verify_signature(&headers, &body, &cfg.secret) {
        // Best-effort audit; don't fail the audit attempt.
        let _ = svc
            .store
            .record_audit(
                "webhook",
                "github",
                "POST /api/webhook/github",
                None,
                &format!("error:bad_signature:{e}"),
                started.elapsed().as_millis() as i64,
            )
            .await;
        return Err(e);
    }
    let payload: WebhookPayload = serde_json::from_slice(&body)
        .map_err(|e| DeployError::BadRequest(format!("bad json: {e}")))?;

    let action = payload.action.as_deref().unwrap_or("?");
    let pr_ref = payload
        .pull_request
        .as_ref()
        .map(|pr| pr.head.git_ref.as_str())
        .unwrap_or("");
    let params_json = serde_json::json!({"action": action, "ref": pr_ref}).to_string();

    let mut close_result = "ok".to_string();
    if action == "closed" {
        if let Some(pr) = &payload.pull_request {
            let name = sanitize(&pr.head.git_ref);
            if svc.store.get_worktree(&name).await?.is_some() {
                let path = axum::extract::Path(name.clone());
                // Synthesize a caller for the internal close — webhook auth is HMAC-based
                // and bypasses the bearer auth middleware, so we mint a webhook identity
                // with just the scope it needs.
                let webhook_caller = CallerIdentity {
                    kind: "webhook".into(),
                    id: "github".into(),
                    scopes: vec!["worktree:close".into()],
                };
                match crate::api::worktree::close_handler(
                    State(svc.clone()),
                    axum::Extension(webhook_caller),
                    path,
                )
                .await
                {
                    Ok(_) => {
                        // Record the synthesized close as its own audit entry attributing it to the webhook.
                        let _ = svc
                            .store
                            .record_audit(
                                "webhook",
                                "github",
                                &format!("POST /api/worktree/{name}/close (synthesized)"),
                                Some(&params_json),
                                "ok",
                                started.elapsed().as_millis() as i64,
                            )
                            .await;
                    }
                    Err(e) => {
                        close_result = format!("error:close_failed:{e}");
                        let _ = svc
                            .store
                            .record_audit(
                                "webhook",
                                "github",
                                &format!("POST /api/worktree/{name}/close (synthesized)"),
                                Some(&params_json),
                                &close_result,
                                started.elapsed().as_millis() as i64,
                            )
                            .await;
                    }
                }
            }
        }
    }

    let _ = svc
        .store
        .record_audit(
            "webhook",
            "github",
            "POST /api/webhook/github",
            Some(&params_json),
            &close_result,
            started.elapsed().as_millis() as i64,
        )
        .await;

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
