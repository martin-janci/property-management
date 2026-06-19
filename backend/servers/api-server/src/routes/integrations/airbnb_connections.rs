//! Airbnb linked-listing read surface (Coverage 83-1).
//!
//! | Method | Path | Purpose |
//! |--------|------|---------|
//! | GET | `/organizations/{org_id}/airbnb/connections` | DB-backed list of the org's stored Airbnb connections (linked listings) |
//!
//! # Why this exists alongside `/airbnb/listings`
//!
//! The pre-existing `GET /airbnb/listings` route (in [`super::oauth`]) proxies
//! the Airbnb Partner API through the org's stored OAuth token, auto-refreshing
//! it as needed. That route requires a valid token plus outbound network access
//! and answers `404` / `502` when the org is not connected or Airbnb is down.
//!
//! This route is the complementary *local* read: it returns the connection rows
//! the system already stores for the org — the "linked listings" — without any
//! external round-trip. It is the surface a management dashboard uses to render
//! the list of linked listings and their last-sync / error state, even when the
//! token is expired or Airbnb is unreachable.
//!
//! # Tenant scoping
//!
//! Every handler runs [`verify_org_access`] (membership IDOR guard, issue #765)
//! before touching the repository, and the repository query is itself scoped to
//! `organization_id = $1`. Secrets are never serialised: the response uses
//! [`PlatformConnectionDetail`], the token-free DTO (issue #887 / #804).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use db::models::rental::PlatformConnectionDetail;
use serde::Serialize;
use utoipa::ToSchema;

use super::sync::{verify_org_access, OrgIdPath};
use crate::state::AppState;
use common::errors::ErrorResponse;

// ==================== Types ====================

/// Response body for the Airbnb linked-listing list.
#[derive(Debug, Serialize, ToSchema)]
pub struct AirbnbConnectionsResponse {
    /// The org's stored Airbnb connections, newest-first. Token-free.
    pub connections: Vec<PlatformConnectionDetail>,
    /// Number of connections returned.
    pub count: usize,
}

// ==================== Router ====================

/// Airbnb linked-listing read sub-router.
pub fn router() -> Router<AppState> {
    Router::new().route(
        "/organizations/{org_id}/airbnb/connections",
        get(list_airbnb_connections),
    )
}

// ==================== Handlers ====================

/// List the organisation's stored Airbnb connections (linked listings).
///
/// Returns the DB-backed connection rows for `platform = 'airbnb'`, scoped to
/// the path organisation. Unlike `/airbnb/listings`, this never calls the
/// Airbnb API — so it succeeds (with an empty list) even when the org has no
/// connection, the token is expired, or Airbnb is unreachable.
#[utoipa::path(
    get,
    path = "/api/v1/integrations/organizations/{org_id}/airbnb/connections",
    params(OrgIdPath),
    responses(
        (status = 200, description = "Stored Airbnb connections for the org", body = AirbnbConnectionsResponse),
        (status = 403, description = "Caller is not a member of the organisation"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations - Airbnb"
)]
pub async fn list_airbnb_connections(
    State(state): State<AppState>,
    auth: api_core::AuthUser,
    Path(path): Path<OrgIdPath>,
) -> Result<Json<AirbnbConnectionsResponse>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        user_id = %auth.user_id,
        org_id = %path.org_id,
        "Listing stored Airbnb connections"
    );

    // IDOR guard (issue #765): caller must belong to this org.
    verify_org_access(&state, auth.user_id, path.org_id).await?;

    let connections = state
        .rental_repo
        .list_airbnb_connections(path.org_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list Airbnb connections");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to list Airbnb connections",
                )),
            )
        })?;

    // Map to the token-free DTO so access/refresh tokens are never serialised.
    let connections: Vec<PlatformConnectionDetail> = connections
        .into_iter()
        .map(PlatformConnectionDetail::from)
        .collect();
    let count = connections.len();

    Ok(Json(AirbnbConnectionsResponse { connections, count }))
}

// ==================== Tests ====================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use db::models::rental::RentalPlatformConnection;
    use uuid::Uuid;

    fn sample_connection(with_token: bool) -> RentalPlatformConnection {
        RentalPlatformConnection {
            id: Uuid::new_v4(),
            organization_id: Uuid::new_v4(),
            unit_id: Uuid::nil(),
            platform: "airbnb".to_string(),
            access_token: if with_token {
                Some("enc:secret".to_string())
            } else {
                None
            },
            refresh_token: if with_token {
                Some("enc:refresh".to_string())
            } else {
                None
            },
            token_expires_at: None,
            encrypted_token: None,
            encrypted_refresh_token: None,
            external_property_id: Some("LISTING-123".to_string()),
            external_listing_url: Some("https://airbnb.com/rooms/123".to_string()),
            is_active: true,
            last_sync_at: Some(Utc::now()),
            sync_error: None,
            sync_calendar: true,
            sync_interval_minutes: 15,
            block_other_platforms: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn response_serializes_with_count_and_no_tokens() {
        let detail = PlatformConnectionDetail::from(sample_connection(true));
        let resp = AirbnbConnectionsResponse {
            connections: vec![detail],
            count: 1,
        };

        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["count"], 1);
        assert_eq!(json["connections"][0]["platform"], "airbnb");
        assert_eq!(
            json["connections"][0]["external_property_id"],
            "LISTING-123"
        );
        // is_connected is derived from token presence; the token itself must
        // never appear in the serialised payload.
        assert_eq!(json["connections"][0]["is_connected"], true);
        assert!(json["connections"][0].get("access_token").is_none());
        assert!(json["connections"][0].get("refresh_token").is_none());
    }

    #[test]
    fn detail_marks_tokenless_connection_disconnected() {
        let detail = PlatformConnectionDetail::from(sample_connection(false));
        let json = serde_json::to_value(&detail).unwrap();
        assert_eq!(json["is_connected"], false);
        assert!(json.get("access_token").is_none());
    }

    #[test]
    fn empty_response_serializes() {
        let resp = AirbnbConnectionsResponse {
            connections: vec![],
            count: 0,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["count"], 0);
        assert!(json["connections"].as_array().unwrap().is_empty());
    }
}
