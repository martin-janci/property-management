// backend/servers/deploy-server/src/api/health.rs
use axum::Json;
use serde_json::json;

pub async fn handler() -> Json<serde_json::Value> {
    Json(json!({"status": "ok", "version": env!("CARGO_PKG_VERSION")}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn health_returns_ok() {
        let Json(body) = handler().await;
        assert_eq!(body["status"], "ok");
    }
}
