//! WebSocket real-time sensor reading channel (Epic 14, Story 14.3).
//!
//! # Purpose
//!
//! Opens a persistent WebSocket connection for an authenticated user and
//! streams sensor reading events from the Redis pub/sub channel
//! `sensors:{org_id}`.  Events are published by `routes/iot.rs` whenever a
//! reading is ingested via `POST /api/v1/iot/sensors/{id}/readings` or the
//! batch variant.
//!
//! # Authentication
//!
//! Same token-in-query-param pattern as `ws_notifications.rs`: the caller
//! supplies a standard JWT as the `token` query parameter.  The resolved
//! `org_id` comes from the token's `organization_id` claim via
//! `api_core::extractors::validate_access_token_with_exp`.
//!
//! # Wire format
//!
//! Server → client JSON text frames:
//! ```json
//! { "event": "sensor.reading", "payload": { <SensorReading fields> } }
//! ```
//!
//! Client → server: any frame is treated as a heartbeat.  The connection is
//! closed server-side after `WS_IDLE_TIMEOUT_SECS` of inactivity.

use std::time::Duration;

use api_core::extractors::validate_access_token_full;
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
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::state::AppState;

const WS_IDLE_TIMEOUT_SECS: u64 = 60;
const WS_MAX_SESSION_SECS: u64 = 4 * 60 * 60;

// ============================================================================
// Router
// ============================================================================

pub fn router() -> Router<AppState> {
    Router::new().route("/ws", get(ws_handler))
}

// ============================================================================
// Query parameters
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct WsQuery {
    pub token: String,
}

// ============================================================================
// Outbound message shape
// ============================================================================

#[derive(Debug, Serialize)]
pub struct WsEvent {
    pub event: String,
    pub payload: serde_json::Value,
}

// ============================================================================
// Handler
// ============================================================================

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // Validate JWT; extract user_id, exp, and tenant_id (org_id) from claims.
    let (user_id, exp, org_id) = match validate_ws_token_with_exp(&params.token) {
        Ok(t) => t,
        Err(msg) => {
            tracing::warn!(msg = %msg, "Sensor WS connection rejected: invalid token");
            return (StatusCode::UNAUTHORIZED, msg).into_response();
        }
    };

    let pubsub_service = state.pubsub_service.clone();

    ws.protocols(["ppt.v1"])
        .on_upgrade(move |socket| handle_ws_session(socket, user_id, org_id, exp, pubsub_service))
        .into_response()
}

// ============================================================================
// Session driver
// ============================================================================

async fn handle_ws_session(
    mut socket: WebSocket,
    user_id: Uuid,
    org_id: Option<Uuid>,
    jwt_exp_unix: i64,
    pubsub_service: Option<integrations::PubSubService>,
) {
    tracing::info!(
        user_id = %user_id,
        org_id = ?org_id,
        "Sensor WS session opened"
    );

    let channel = org_id.map(|oid| format!("sensors:{oid}"));

    let mut pubsub_rx = match (pubsub_service.as_ref(), channel.as_deref()) {
        (Some(svc), Some(ch)) => match svc.subscribe(ch).await {
            Ok(rx) => {
                tracing::debug!(channel = %ch, "Subscribed to Redis sensor channel");
                Some(rx)
            }
            Err(e) => {
                tracing::warn!(
                    channel = %ch,
                    error = %e,
                    "Failed to subscribe to Redis sensor channel; heartbeat-only mode"
                );
                None
            }
        },
        _ => {
            tracing::debug!(
                user_id = %user_id,
                "Redis not configured or org_id unknown; sensor WS in heartbeat-only mode"
            );
            None
        }
    };

    let idle_timeout = Duration::from_secs(WS_IDLE_TIMEOUT_SECS);
    let max_session = Duration::from_secs(WS_MAX_SESSION_SECS);
    let session_deadline = tokio::time::Instant::now() + max_session;

    loop {
        if tokio::time::Instant::now() >= session_deadline {
            tracing::info!(user_id = %user_id, "Sensor WS session max-lifetime reached; closing");
            break;
        }

        if chrono::Utc::now().timestamp() >= jwt_exp_unix {
            tracing::info!(user_id = %user_id, "Sensor WS session JWT expired; closing");
            break;
        }

        tokio::select! {
            pubsub_msg = async {
                match pubsub_rx.as_mut() {
                    Some(rx) => rx.recv().await.ok(),
                    None => {
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
                            tracing::warn!(error = %e, "Failed to serialise sensor WsEvent; skipping");
                            continue;
                        }
                    };
                    if socket.send(Message::Text(text.into())).await.is_err() {
                        tracing::debug!(user_id = %user_id, "Sensor WS send failed; closing");
                        break;
                    }
                }
            }

            inbound = tokio::time::timeout(idle_timeout, socket.recv()) => {
                match inbound {
                    Err(_elapsed) => {
                        if socket.send(Message::Ping(vec![].into())).await.is_err() {
                            tracing::debug!(user_id = %user_id, "Sensor WS ping failed; closing");
                            break;
                        }
                    }
                    Ok(Some(Ok(Message::Close(_)))) | Ok(None) => {
                        tracing::debug!(user_id = %user_id, "Sensor WS closed by client");
                        break;
                    }
                    Ok(Some(Err(e))) => {
                        tracing::debug!(user_id = %user_id, error = %e, "Sensor WS receive error; closing");
                        break;
                    }
                    Ok(Some(Ok(_other))) => {
                        tracing::trace!(user_id = %user_id, "Sensor WS heartbeat received");
                    }
                }
            }
        }
    }

    tracing::info!(user_id = %user_id, "Sensor WS session closed");
}

// ============================================================================
// JWT validation
// ============================================================================

fn validate_ws_token_with_exp(token: &str) -> Result<(Uuid, i64, Option<Uuid>), &'static str> {
    validate_access_token_full(token)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ws_event_serialises_correctly() {
        let evt = WsEvent {
            event: "sensor.reading".to_string(),
            payload: serde_json::json!({
                "sensor_id": "00000000-0000-0000-0000-000000000001",
                "value": 23.5,
                "unit": "°C"
            }),
        };
        let text = serde_json::to_string(&evt).unwrap();
        assert!(text.contains("\"event\":\"sensor.reading\""));
        assert!(text.contains("\"payload\""));
    }
}
