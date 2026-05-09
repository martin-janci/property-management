// backend/servers/deploy-server/src/error.rs
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DeployError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("already exists: {0}")]
    Conflict(String),

    #[error("unauthorized: {0}")]
    Unauthorized(String),

    #[error("forbidden: {0}")]
    Forbidden(String),

    #[error("invalid input: {0}")]
    BadRequest(String),

    #[error("docker error: {0}")]
    Docker(#[from] bollard::errors::Error),

    #[error("sqlite error: {0}")]
    Db(#[from] sqlx::Error),

    #[error("http client error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("config error: {0}")]
    Config(String),

    /// Transient: container is created/started but Docker hasn't finished
    /// assigning a bridge-network IP yet. Callers (notably `worktree::open`)
    /// match this variant structurally to retry, instead of inspecting the
    /// `Internal(String)` text. Surfaces as a generic 500 to clients on
    /// timeout — the retry loop catches the in-process race.
    #[error("bridge IP not ready yet for container {0}")]
    BridgeIpNotReady(String),

    #[error("internal: {0}")]
    Internal(String),
}

impl DeployError {
    fn status(&self) -> StatusCode {
        match self {
            DeployError::NotFound(_) => StatusCode::NOT_FOUND,
            DeployError::Conflict(_) => StatusCode::CONFLICT,
            DeployError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            DeployError::Forbidden(_) => StatusCode::FORBIDDEN,
            DeployError::BadRequest(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for DeployError {
    fn into_response(self) -> Response {
        let status = self.status();
        let body = Json(json!({
            "error": self.to_string(),
        }));
        (status, body).into_response()
    }
}

pub type Result<T> = std::result::Result<T, DeployError>;
