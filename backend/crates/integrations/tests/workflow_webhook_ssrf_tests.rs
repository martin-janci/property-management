//! Regression: workflow `call_webhook` actions must not be usable for SSRF
//! (closes the P0 part of #835).
//!
//! `execute_webhook` took the webhook URL straight from (attacker-influenced)
//! workflow config and issued the request without validation, so a workflow
//! could be pointed at `http://169.254.169.254/…` (cloud metadata),
//! `localhost`, or any private/reserved address. The fix runs
//! `common::url_validation::validate_external_url` first and rejects with
//! `InvalidConfig`.
//!
//! These URLs are rejected by the validator *before* any network call, so the
//! test is fast and deterministic.

use integrations::workflow_executor::WorkflowExecutor;
use serde_json::json;

async fn webhook_is_rejected(url: &str) {
    let exec = WorkflowExecutor::new(None);
    let config = json!({ "url": url, "method": "GET" });
    // retry_count = 0 → single attempt, no backoff sleeps.
    let result = exec
        .execute_action("call_webhook", &config, &json!({}), 0, 0)
        .await;

    let err = result.expect_err(&format!("SSRF URL {url} must be rejected"));
    let msg = err.to_string();
    assert!(
        msg.contains("rejected"),
        "expected a URL-rejection error for {url}, got: {msg}"
    );
}

#[tokio::test]
async fn webhook_to_cloud_metadata_is_rejected() {
    // AWS/GCP link-local metadata endpoint — the canonical SSRF target.
    webhook_is_rejected("http://169.254.169.254/latest/meta-data/").await;
}

#[tokio::test]
async fn webhook_to_loopback_is_rejected() {
    webhook_is_rejected("http://127.0.0.1:8080/internal").await;
}

#[tokio::test]
async fn webhook_to_localhost_is_rejected() {
    webhook_is_rejected("http://localhost/admin").await;
}

#[tokio::test]
async fn webhook_to_private_range_is_rejected() {
    webhook_is_rejected("http://10.0.0.5/").await;
}

#[tokio::test]
async fn webhook_non_http_scheme_is_rejected() {
    webhook_is_rejected("file:///etc/passwd").await;
}
