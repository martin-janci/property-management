//! Realtime sensor channel (Story 14.3 — "values update in real-time").
//!
//! Story 14.3 AC requires sensor values to update in real time over WebSocket.
//! The REST router exposes only REST, so this module adds a push channel that
//! mirrors the `ws_notifications.rs` pattern (Epic 8A.3): an authenticated WS
//! upgrade subscribes to a Redis pub/sub channel and forwards events as JSON
//! text frames. Here the channel is **org-scoped** (`sensors:{org_id}`) and the
//! ingest handlers (`add_reading` / `add_batch_readings`) publish onto it.
//!
//! # Authentication & tenant scoping
//!
//! WS upgrades cannot carry an `Authorization` header from browsers, so auth is
//! a short-lived JWT in the `token` query param (same verifier as REST). The
//! caller also passes the `organization_id` they want to subscribe to, and we
//! validate active membership against the DB exactly like
//! `ValidatedTenantExtractor` does — a non-member is rejected with `403` before
//! the upgrade, so the channel can never leak another tenant's readings.

use std::time::Duration;

use crate::state::AppState;
use api_core::extractors::validate_access_token_with_exp;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    http::StatusCode,
    response::IntoResponse,
};
use db::repositories::OrganizationMemberRepository;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Wire-format event name for a single newly-ingested reading. Published by
/// `add_reading` and forwarded verbatim to subscribers as the envelope `event`
/// field. The frontend (`useIotWebSocket`) matches on this exact string, so a
/// rename here is a breaking wire-format change — `wire_contract_*` tests pin it.
pub(super) const EVENT_READING_CREATED: &str = "sensor.reading.created";

/// Wire-format event name for a batch ingest. Published by `add_batch_readings`.
pub(super) const EVENT_READINGS_BATCH: &str = "sensor.readings.batch";

/// Maximum idle time before the server closes the connection (60 s).
const WS_IDLE_TIMEOUT_SECS: u64 = 60;

/// Maximum lifetime of a WebSocket session regardless of activity (4 h).
/// Clients reconnect with a fresh token before their JWT (15 m) expires.
const WS_MAX_SESSION_SECS: u64 = 4 * 60 * 60;

/// Query parameters for the sensor WebSocket upgrade request.
#[derive(Debug, Deserialize)]
pub struct SensorWsQuery {
    /// JWT access token — required because WS upgrades cannot carry a custom
    /// `Authorization` header from browser clients.
    pub token: String,
    /// Organization (tenant) whose sensor stream to subscribe to.
    ///
    /// This is **deliberately client-supplied**: a user who belongs to several
    /// orgs picks which org's stream to open. It is NOT trusted for scoping —
    /// authorization is enforced solely by the `is_member(org_id, user_id)` DB
    /// check in [`sensor_ws_handler`] (active membership required), which returns
    /// `403` before the upgrade for a non-member. The channel name is derived
    /// from this validated value, so a foreign org id can never leak another
    /// tenant's readings. Covered by `iot_sensor_ws_authz_tests`.
    pub organization_id: Uuid,
}

/// Envelope for every server-to-client sensor WebSocket frame.
#[derive(Debug, Serialize)]
pub struct SensorWsEvent {
    /// Mirrors `PubSubMessage::event_type` (`sensor.reading.created`,
    /// `sensor.readings.batch`).
    pub event: String,
    /// Opaque JSON payload forwarded from the Redis pub/sub message.
    pub payload: serde_json::Value,
}

/// Build the org-scoped Redis pub/sub channel name for sensor readings.
fn sensor_channel(org_id: Uuid) -> String {
    format!("sensors:{org_id}")
}

/// Publish a sensor realtime event to the org-scoped channel.
///
/// Best-effort: the reading is already persisted, so a pub/sub failure (or no
/// Redis configured in local dev) is logged and swallowed rather than failing
/// the ingest request — subscribers still see the value on their next fetch.
pub(super) async fn publish_sensor_event(
    pubsub: Option<&integrations::PubSubService>,
    org_id: Uuid,
    event_type: &str,
    payload: serde_json::Value,
) {
    let Some(pubsub) = pubsub else {
        return;
    };
    let channel = sensor_channel(org_id);
    let msg = integrations::PubSubMessage::new(&channel, event_type, payload);
    if let Err(e) = pubsub.publish(&channel, msg).await {
        tracing::warn!(
            org_id = %org_id,
            channel = %channel,
            event = %event_type,
            error = %e,
            "[iot] Failed to publish sensor event to WebSocket channel (non-fatal)"
        );
    }
}

/// WebSocket upgrade handler for realtime sensor readings (Story 14.3).
///
/// Validates the `token` query param as a JWT access token, confirms the caller
/// is an active member of `organization_id`, then upgrades the connection and
/// forwards the org's Redis pub/sub sensor events to the client.
pub async fn sensor_ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<SensorWsQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // --- JWT validation (same verifier/lifetime as REST auth) ---
    let (user_id, exp) = match validate_access_token_with_exp(&params.token) {
        Ok(t) => t,
        Err(msg) => {
            // Never log `params.token` — it is a bearer JWT.
            tracing::warn!(msg = %msg, "Sensor WS rejected: invalid token");
            return (StatusCode::UNAUTHORIZED, msg).into_response();
        }
    };

    // --- Tenant membership check (mirrors ValidatedTenantExtractor) ---
    let org_id = params.organization_id;
    let member_repo = OrganizationMemberRepository::new(state.db.clone());
    match member_repo.is_member(org_id, user_id).await {
        Ok(true) => {}
        Ok(false) => {
            tracing::warn!(
                user_id = %user_id,
                org_id = %org_id,
                "Sensor WS rejected: user is not a member of the requested org"
            );
            return (StatusCode::FORBIDDEN, "Not a member of this organization").into_response();
        }
        Err(e) => {
            tracing::error!(
                user_id = %user_id,
                org_id = %org_id,
                error = %e,
                "Sensor WS: failed to verify org membership"
            );
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to verify organization access",
            )
                .into_response();
        }
    }

    let pubsub_service = state.pubsub_service.clone();

    // Echo the `ppt.v1` subprotocol so browsers that offer it complete the
    // handshake (same compat shim as ws_notifications, issue #438).
    ws.protocols(["ppt.v1"])
        .on_upgrade(move |socket| {
            handle_sensor_ws_session(socket, org_id, user_id, exp, pubsub_service)
        })
        .into_response()
}

/// Drive one sensor WebSocket session for `org_id`.
///
/// Subscribes to `sensors:{org_id}` on Redis (when available) and forwards
/// matching pub/sub events. Falls back to a heartbeat-only loop when Redis is
/// not configured. Closes once the JWT expires (issue #480 parity) or the max
/// session lifetime is reached.
async fn handle_sensor_ws_session(
    mut socket: WebSocket,
    org_id: Uuid,
    user_id: Uuid,
    jwt_exp_unix: i64,
    pubsub_service: Option<integrations::PubSubService>,
) {
    tracing::info!(org_id = %org_id, user_id = %user_id, "Sensor WS session opened");

    let channel = sensor_channel(org_id);

    let mut pubsub_rx = match pubsub_service {
        Some(ref svc) => match svc.subscribe(&channel).await {
            Ok(rx) => {
                tracing::debug!(channel = %channel, "Subscribed to Redis sensor channel");
                Some(rx)
            }
            Err(e) => {
                tracing::warn!(
                    channel = %channel,
                    error = %e,
                    "Failed to subscribe to Redis sensor channel; heartbeat-only mode"
                );
                None
            }
        },
        None => {
            tracing::debug!(
                org_id = %org_id,
                "Redis not configured; sensor WS running in heartbeat-only mode"
            );
            None
        }
    };

    let idle_timeout = Duration::from_secs(WS_IDLE_TIMEOUT_SECS);
    let max_session = Duration::from_secs(WS_MAX_SESSION_SECS);
    let session_deadline = tokio::time::Instant::now() + max_session;

    loop {
        if tokio::time::Instant::now() >= session_deadline {
            tracing::info!(org_id = %org_id, "Sensor WS max-lifetime reached; closing");
            break;
        }

        // Close as soon as the JWT expires so a logged-out user does not retain
        // a live channel (issue #480 parity).
        if chrono::Utc::now().timestamp() >= jwt_exp_unix {
            tracing::info!(org_id = %org_id, "Sensor WS JWT expired; closing");
            break;
        }

        tokio::select! {
            // ---- Redis pub/sub event ----
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
                    let envelope = SensorWsEvent {
                        event: msg.event_type.clone(),
                        payload: msg.payload.clone(),
                    };
                    let text = match serde_json::to_string(&envelope) {
                        Ok(t) => t,
                        Err(e) => {
                            tracing::warn!(error = %e, "Failed to serialise SensorWsEvent; skipping");
                            continue;
                        }
                    };
                    if socket.send(Message::Text(text.into())).await.is_err() {
                        tracing::debug!(org_id = %org_id, "Sensor WS send failed; closing");
                        break;
                    }
                }
            }

            // ---- Inbound client frame (heartbeat / close) ----
            inbound = tokio::time::timeout(idle_timeout, socket.recv()) => {
                match inbound {
                    Err(_elapsed) => {
                        if socket.send(Message::Ping(vec![].into())).await.is_err() {
                            tracing::debug!(org_id = %org_id, "Sensor WS ping failed; closing");
                            break;
                        }
                    }
                    Ok(Some(Ok(Message::Close(_)))) | Ok(None) => {
                        tracing::debug!(org_id = %org_id, "Sensor WS closed by client");
                        break;
                    }
                    Ok(Some(Err(e))) => {
                        tracing::debug!(org_id = %org_id, error = %e, "Sensor WS receive error; closing");
                        break;
                    }
                    Ok(Some(Ok(_other))) => {
                        tracing::trace!(org_id = %org_id, "Sensor WS heartbeat received");
                    }
                }
            }
        }
    }

    tracing::info!(org_id = %org_id, user_id = %user_id, "Sensor WS session closed");
}

#[cfg(test)]
mod realtime_tests {
    use super::*;

    #[test]
    fn sensor_channel_is_org_scoped() {
        let org = Uuid::parse_str("00000000-0000-0000-0000-0000000000aa").unwrap();
        assert_eq!(
            sensor_channel(org),
            "sensors:00000000-0000-0000-0000-0000000000aa"
        );
    }

    #[test]
    fn different_orgs_get_distinct_channels() {
        let a = Uuid::parse_str("00000000-0000-0000-0000-0000000000a1").unwrap();
        let b = Uuid::parse_str("00000000-0000-0000-0000-0000000000b2").unwrap();
        assert_ne!(sensor_channel(a), sensor_channel(b));
    }

    #[test]
    fn sensor_ws_event_serialises_with_event_and_payload() {
        let evt = SensorWsEvent {
            event: EVENT_READING_CREATED.to_string(),
            payload: serde_json::json!({
                "id": "00000000-0000-0000-0000-000000000001",
                "sensor_id": "00000000-0000-0000-0000-000000000002",
                "value": 21.5,
                "unit": "C"
            }),
        };
        let text = serde_json::to_string(&evt).unwrap();
        assert!(text.contains("\"event\":\"sensor.reading.created\""));
        assert!(text.contains("\"payload\""));
        assert!(text.contains("\"value\":21.5"));
    }

    #[test]
    fn batch_event_carries_sensor_and_count() {
        let evt = SensorWsEvent {
            event: EVENT_READINGS_BATCH.to_string(),
            payload: serde_json::json!({
                "sensor_id": "00000000-0000-0000-0000-000000000002",
                "inserted": 7
            }),
        };
        let text = serde_json::to_string(&evt).unwrap();
        assert!(text.contains("\"event\":\"sensor.readings.batch\""));
        assert!(text.contains("\"inserted\":7"));
    }

    /// Pin the published event names so a future rename of the ingest
    /// publishers (`add_reading` / `add_batch_readings`) fails CI rather than
    /// silently breaking the documented wire format. This is the regression
    /// guard for issue #1668: PR #1644 renamed these event types out from under
    /// PR #1640's subscriber, and no test caught it because each handler only
    /// asserted its own side. The frontend `useIotWebSocket` hook matches on
    /// these exact strings.
    #[test]
    fn publisher_event_names_match_documented_wire_contract() {
        assert_eq!(EVENT_READING_CREATED, "sensor.reading.created");
        assert_eq!(EVENT_READINGS_BATCH, "sensor.readings.batch");
    }

    /// End-to-end wire-contract guard: build the subscriber envelope exactly as
    /// `handle_sensor_ws_session` does — from a `PubSubMessage` carrying the
    /// publisher's `event_type` — and assert the serialised `event` field equals
    /// the publisher constant. This closes the publish→subscribe gap the issue
    /// flagged: it ties the publisher's event name to the frame a subscriber
    /// actually receives, so the two can never diverge again unnoticed.
    #[test]
    fn publish_subscribe_event_names_round_trip() {
        for event in [EVENT_READING_CREATED, EVENT_READINGS_BATCH] {
            // Publisher side: what `publish_sensor_event` puts on the channel.
            let published = integrations::PubSubMessage::new(
                &sensor_channel(Uuid::nil()),
                event,
                serde_json::json!({}),
            );
            // Subscriber side: how `handle_sensor_ws_session` forwards it.
            let envelope = SensorWsEvent {
                event: published.event_type.clone(),
                payload: published.payload.clone(),
            };
            let text = serde_json::to_string(&envelope).unwrap();
            assert!(
                text.contains(&format!("\"event\":\"{event}\"")),
                "subscriber frame must carry the publisher's event name `{event}`",
            );
        }
    }
}
