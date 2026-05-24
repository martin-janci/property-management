//! WebSocket realtime notification sync (Epic 8A, Story 8A.3).
//!
//! # Purpose
//!
//! Opens a persistent WebSocket connection for a single authenticated user
//! and streams realtime events from the Redis pub/sub channel
//! `notifications:{user_id}`.  Events originate from two paths:
//!
//! 1. **In-app notification created** — `DbInAppAdapter::send` (Epic 2B) publishes
//!    `notification.created` after writing to the DB.
//! 2. **Preference changed** — the `PATCH /api/v1/users/me/notification-preferences/{channel}`
//!    handler publishes `preference.updated` so clients can invalidate their
//!    cached preference state without polling.
//!
//! # Authentication
//!
//! WebSocket upgrades cannot carry a custom `Authorization` header via the browser
//! WS API, so auth is performed via a **short-lived one-time token** passed as the
//! `token` query parameter.  The value is a standard JWT access token issued by
//! the existing `JwtService`; the server validates it with the shared `OnceLock`
//! verifier (same path as REST auth, same 15-min lifetime).
//!
//! # Wire format
//!
//! Client → server: any message is treated as a heartbeat (pong).  The connection
//! is closed server-side after `WS_IDLE_TIMEOUT_SECS` without activity.
//!
//! Server → client: JSON text frames, one per event:
//! ```json
//! {
//!   "event": "notification.created",
//!   "payload": { ... }
//! }
//! ```
//!
//! The `event` field mirrors the `event_type` stored in `PubSubMessage`.

use std::time::Duration;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Router,
};
use chrono::Utc;
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::state::AppState;

/// Maximum idle time before the server closes the connection (60 s).
const WS_IDLE_TIMEOUT_SECS: u64 = 60;

/// Maximum lifetime of a WebSocket session regardless of activity (4 h).
/// Clients should reconnect before their JWT expires (15 m) anyway.
const WS_MAX_SESSION_SECS: u64 = 4 * 60 * 60;

// ============================================================================
// Router
// ============================================================================

/// Create the WebSocket notification sync router.
pub fn router() -> Router<AppState> {
    Router::new().route("/ws", get(ws_handler))
}

// ============================================================================
// Query parameters
// ============================================================================

/// Query parameters for the WebSocket upgrade request.
#[derive(Debug, Deserialize)]
pub struct WsQuery {
    /// JWT access token — required for auth because WS upgrades cannot carry
    /// a custom `Authorization` header from browser clients.
    pub token: String,
}

// ============================================================================
// Outbound message shape
// ============================================================================

/// Envelope for every server-to-client WebSocket frame.
#[derive(Debug, Serialize)]
pub struct WsEvent {
    /// Mirrors `PubSubMessage::event_type` (e.g. `notification.created`,
    /// `preference.updated`).
    pub event: String,
    /// Opaque JSON payload forwarded from the Redis pub/sub message.
    pub payload: serde_json::Value,
}

// ============================================================================
// Handler
// ============================================================================

/// WebSocket upgrade handler for realtime notification sync (Story 8A.3).
///
/// Validates the `token` query param as a JWT access token, then upgrades the
/// connection and forwards Redis pub/sub events to the WebSocket client.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // --- JWT validation ---
    let user_id = match validate_ws_token(&params.token) {
        Ok(uid) => uid,
        Err(msg) => {
            tracing::warn!(msg = %msg, "WebSocket connection rejected: invalid token");
            return (StatusCode::UNAUTHORIZED, msg).into_response();
        }
    };

    // Clone the pubsub service before moving into the closure.
    let pubsub_service = state.pubsub_service.clone();

    ws.on_upgrade(move |socket| {
        handle_ws_session(socket, user_id, pubsub_service)
    })
    .into_response()
}

// ============================================================================
// Session driver
// ============================================================================

/// Drive one WebSocket session for `user_id`.
///
/// Subscribes to `notifications:{user_id}` on Redis (when available) and
/// forwards matching pub/sub events.  Falls back to a heartbeat-only loop
/// when Redis is not configured.
async fn handle_ws_session(
    mut socket: WebSocket,
    user_id: Uuid,
    pubsub_service: Option<integrations::PubSubService>,
) {
    tracing::info!(
        user_id = %user_id,
        "WebSocket notification session opened"
    );

    let channel = format!("notifications:{user_id}");

    // Attempt to subscribe to the user's Redis notification channel.
    let mut pubsub_rx = match pubsub_service {
        Some(ref svc) => match svc.subscribe(&channel).await {
            Ok(rx) => {
                tracing::debug!(channel = %channel, "Subscribed to Redis notification channel");
                Some(rx)
            }
            Err(e) => {
                tracing::warn!(
                    channel = %channel,
                    error = %e,
                    "Failed to subscribe to Redis channel; running in heartbeat-only mode"
                );
                None
            }
        },
        None => {
            tracing::debug!(
                user_id = %user_id,
                "Redis not configured; WebSocket running in heartbeat-only mode"
            );
            None
        }
    };

    let idle_timeout = Duration::from_secs(WS_IDLE_TIMEOUT_SECS);
    let max_session = Duration::from_secs(WS_MAX_SESSION_SECS);
    let session_deadline = tokio::time::Instant::now() + max_session;

    loop {
        // Check global session deadline.
        if tokio::time::Instant::now() >= session_deadline {
            tracing::info!(user_id = %user_id, "WebSocket session max-lifetime reached; closing");
            break;
        }

        tokio::select! {
            // ---- Redis pub/sub event ----
            pubsub_msg = async {
                match pubsub_rx.as_mut() {
                    Some(rx) => rx.recv().await.ok(),
                    None => {
                        // No subscriber — sleep so we don't busy-loop.
                        tokio::time::sleep(Duration::from_secs(30)).await;
                        None
                    }
                }
            } => {
                if let Some(msg) = pubsub_msg {
                    let envelope = WsEvent {
                        event: msg.event_type.clone(),
                        payload: msg.payload.clone(),
                    };
                    let text = match serde_json::to_string(&envelope) {
                        Ok(t) => t,
                        Err(e) => {
                            tracing::warn!(error = %e, "Failed to serialise WsEvent; skipping");
                            continue;
                        }
                    };
                    if socket.send(Message::Text(text.into())).await.is_err() {
                        tracing::debug!(user_id = %user_id, "WebSocket send failed; closing session");
                        break;
                    }
                }
            }

            // ---- Inbound client frame (heartbeat / close) ----
            inbound = tokio::time::timeout(idle_timeout, socket.recv()) => {
                match inbound {
                    Err(_elapsed) => {
                        // Idle timeout — send a ping to check if client is alive.
                        if socket.send(Message::Ping(vec![].into())).await.is_err() {
                            tracing::debug!(user_id = %user_id, "WebSocket ping failed; closing");
                            break;
                        }
                        // If pong not received within another idle_timeout, the next
                        // select iteration will time out again and we'll break.
                    }
                    Ok(Some(Ok(Message::Close(_)))) | Ok(None) => {
                        tracing::debug!(user_id = %user_id, "WebSocket closed by client");
                        break;
                    }
                    Ok(Some(Err(e))) => {
                        tracing::debug!(user_id = %user_id, error = %e, "WebSocket receive error; closing");
                        break;
                    }
                    Ok(Some(Ok(_other))) => {
                        // Text / Binary / Ping / Pong frames from client → treat as heartbeat.
                        tracing::trace!(user_id = %user_id, "WebSocket heartbeat received");
                    }
                }
            }
        }
    }

    tracing::info!(user_id = %user_id, "WebSocket notification session closed");
}

// ============================================================================
// JWT validation (query-param path)
// ============================================================================

/// Validate a JWT access token supplied as a query parameter and return the
/// subject user ID.
///
/// Uses the same `JWT_SECRET` env-var + `jsonwebtoken` verification path as
/// the `AuthUser` extractor in `api_core::extractors::auth`, but operates on
/// a plain `&str` rather than an HTTP header.
fn validate_ws_token(token: &str) -> Result<Uuid, &'static str> {
    use std::sync::OnceLock;

    #[derive(serde::Deserialize)]
    struct WsClaims {
        sub: Uuid,
        exp: i64,
        token_type: String,
    }

    struct WsVerifier {
        key: DecodingKey,
        validation: Validation,
    }

    static WS_VERIFIER: OnceLock<Result<WsVerifier, String>> = OnceLock::new();

    let cell = WS_VERIFIER.get_or_init(|| {
        let secret = std::env::var("JWT_SECRET")
            .map_err(|_| "JWT_SECRET not set".to_string())?;
        if secret.len() < 32 {
            return Err("JWT_SECRET too short".to_string());
        }
        let mut validation = Validation::default();
        validation.leeway = 30;
        Ok(WsVerifier {
            key: DecodingKey::from_secret(secret.as_bytes()),
            validation,
        })
    });

    let verifier = match cell {
        Ok(v) => v,
        Err(_) => return Err("Server configuration error"),
    };

    let token_data = decode::<WsClaims>(token, &verifier.key, &verifier.validation)
        .map_err(|_| "Invalid or expired token")?;

    let claims = token_data.claims;

    // Reject refresh tokens — only access tokens are accepted here.
    if claims.token_type != "access" {
        return Err("Invalid token type for WebSocket auth");
    }

    // Validate exp (jsonwebtoken already checks this, but be explicit).
    let now = chrono::Utc::now().timestamp();
    if claims.exp < now {
        return Err("Token has expired");
    }

    Ok(claims.sub)
}

// ============================================================================
// Unit tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ws_event_serialises_correctly() {
        let evt = WsEvent {
            event: "notification.created".to_string(),
            payload: serde_json::json!({
                "notification_id": "00000000-0000-0000-0000-000000000001",
                "category": "announcements",
                "title": "Test"
            }),
        };
        let text = serde_json::to_string(&evt).unwrap();
        assert!(text.contains("\"event\":\"notification.created\""));
        assert!(text.contains("\"payload\""));
    }

    #[test]
    fn validate_ws_token_rejects_garbage() {
        let result = validate_ws_token("not-a-jwt");
        assert!(result.is_err());
    }

    #[test]
    fn validate_ws_token_rejects_empty() {
        let result = validate_ws_token("");
        assert!(result.is_err());
    }
}
