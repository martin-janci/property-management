//! WebSocket realtime notification sync (Epic 8A, Story 8A.3).
//!
//! gap-8a-3: verified present and clean by auto-impl/gap-8a-3-websocket-realtime-sync.
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
//! WS API, so auth is performed with a standard JWT access token issued by the
//! existing `JwtService`, validated with the shared `OnceLock` verifier (same
//! path as REST auth, same 15-min lifetime).
//!
//! The token is read from the `Sec-WebSocket-Protocol` handshake header as a
//! `bearer.<jwt>` subprotocol (the browser's `new WebSocket(url, ['bearer.<jwt>',
//! 'ppt.v1'])` form). This is preferred over the legacy `?token=<jwt>` query
//! parameter because a query-string token leaks the bearer credential into
//! access logs and proxy logs, whereas the subprotocol header does not. The
//! `token` query parameter is still accepted as a fallback for clients that have
//! not yet migrated.
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

use api_core::extractors::validate_access_token_with_exp;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    http::{header::SEC_WEBSOCKET_PROTOCOL, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::state::AppState;

/// Maximum idle time before the server closes the connection (60 s).
const WS_IDLE_TIMEOUT_SECS: u64 = 60;

/// Prefix that marks the JWT-bearing entry in the `Sec-WebSocket-Protocol`
/// handshake header (`bearer.<jwt>`). Must stay in sync with
/// `WS_BEARER_SUBPROTOCOL_PREFIX` in the ppt-web client (`lib/websocket.ts`).
const WS_BEARER_SUBPROTOCOL_PREFIX: &str = "bearer.";

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
    /// Legacy JWT access token transport. Optional: the preferred transport is
    /// the `bearer.<jwt>` entry in the `Sec-WebSocket-Protocol` header, which
    /// (unlike this query param) does not leak the token into access logs. Kept
    /// as a fallback for clients that have not yet migrated.
    #[serde(default)]
    pub token: Option<String>,
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
    headers: HeaderMap,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // --- Resolve the auth token ---
    // Prefer the `bearer.<jwt>` subprotocol from the `Sec-WebSocket-Protocol`
    // header: unlike the legacy `?token=` query param it never lands in access
    // logs. Fall back to the query param for not-yet-migrated clients.
    let token = match extract_token_from_subprotocols(&headers).or(params.token) {
        Some(t) => t,
        None => {
            tracing::warn!("WebSocket connection rejected: missing auth token");
            return (StatusCode::UNAUTHORIZED, "missing auth token").into_response();
        }
    };

    // --- JWT validation ---
    // Issue #480: also extract the `exp` so the session loop can close
    // the socket once the access token expires; otherwise a logged-out
    // user retains a live channel for up to WS_MAX_SESSION_SECS.
    let (user_id, exp) = match validate_ws_token_with_exp(&token) {
        Ok(t) => t,
        Err(msg) => {
            // Never log the token — it is a bearer JWT and a single
            // log line is enough to give an attacker session takeover.
            tracing::warn!(msg = %msg, "WebSocket connection rejected: invalid token");
            return (StatusCode::UNAUTHORIZED, msg).into_response();
        }
    };

    // Clone the pubsub service before moving into the closure.
    let pubsub_service = state.pubsub_service.clone();

    // Issue #438: echo the `ppt.v1` subprotocol so browsers that send
    // `new WebSocket(url, ['bearer.<jwt>', 'ppt.v1'])` complete the
    // handshake. The spec requires the server to echo at least one of
    // the offered subprotocols or the browser aborts with a silent
    // network error — not a 401.
    ws.protocols(["ppt.v1"])
        .on_upgrade(move |socket| handle_ws_session(socket, user_id, exp, pubsub_service))
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
    jwt_exp_unix: i64,
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

        // Issue #480: close the session as soon as the JWT access token
        // expires so a logged-out user does not retain a live channel.
        // Clients are expected to reconnect with a fresh token.
        if chrono::Utc::now().timestamp() >= jwt_exp_unix {
            tracing::info!(user_id = %user_id, "WebSocket session JWT expired; closing");
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
// Token extraction
// ============================================================================

/// Recover the JWT access token from the `Sec-WebSocket-Protocol` handshake
/// header, where the browser client offers it as a `bearer.<jwt>` subprotocol
/// (alongside `ppt.v1`).
///
/// Returns `None` when the header is absent, non-ASCII, or carries no
/// non-empty `bearer.` entry — callers then fall back to the legacy `?token=`
/// query param. The header is a comma-separated list of subprotocol tokens;
/// we scan for the first `bearer.`-prefixed entry and strip the prefix.
///
/// `pub(crate)` so other WS upgrade handlers in this crate (e.g. the IoT
/// sensor channel in `routes::iot::realtime`) reuse the exact same extraction
/// instead of re-implementing the `bearer.<jwt>` parsing — keeping the client
/// contract (`WS_BEARER_SUBPROTOCOL_PREFIX` in `lib/websocket.ts`) in one place.
pub(crate) fn extract_token_from_subprotocols(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(SEC_WEBSOCKET_PROTOCOL)?.to_str().ok()?;
    value
        .split(',')
        .map(str::trim)
        .find_map(|p| p.strip_prefix(WS_BEARER_SUBPROTOCOL_PREFIX))
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
}

// ============================================================================
// JWT validation (query-param path)
// ============================================================================

/// Validate a JWT access token supplied as a query parameter and return the
/// subject user ID.
///
/// Delegates to `api_core::extractors::validate_access_token` so the shared
/// `JWT_VERIFIER` (with its `leeway`, algorithm, and future `iss`/`aud`
/// hardening) applies uniformly to WebSocket auth and HTTP extractors alike.
#[cfg(test)]
fn validate_ws_token(token: &str) -> Result<Uuid, &'static str> {
    validate_access_token_with_exp(token).map(|(uid, _)| uid)
}

/// Validate a JWT access token supplied as a query parameter and return the
/// subject user ID together with its `exp` Unix timestamp. Issue #480.
fn validate_ws_token_with_exp(token: &str) -> Result<(Uuid, i64), &'static str> {
    validate_access_token_with_exp(token)
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

    fn header_map(protocol: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(SEC_WEBSOCKET_PROTOCOL, protocol.parse().unwrap());
        headers
    }

    #[test]
    fn extract_token_reads_bearer_subprotocol() {
        let headers = header_map("bearer.jwt-abc, ppt.v1");
        assert_eq!(
            extract_token_from_subprotocols(&headers),
            Some("jwt-abc".to_string())
        );
    }

    #[test]
    fn extract_token_handles_bearer_without_whitespace() {
        let headers = header_map("bearer.jwt-xyz,ppt.v1");
        assert_eq!(
            extract_token_from_subprotocols(&headers),
            Some("jwt-xyz".to_string())
        );
    }

    #[test]
    fn extract_token_ignores_non_bearer_protocols() {
        let headers = header_map("ppt.v1");
        assert_eq!(extract_token_from_subprotocols(&headers), None);
    }

    #[test]
    fn extract_token_rejects_empty_bearer() {
        let headers = header_map("bearer., ppt.v1");
        assert_eq!(extract_token_from_subprotocols(&headers), None);
    }

    #[test]
    fn extract_token_none_when_header_absent() {
        assert_eq!(extract_token_from_subprotocols(&HeaderMap::new()), None);
    }
}
