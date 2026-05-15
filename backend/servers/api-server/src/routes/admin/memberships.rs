//! Admin: membership invite + accept + revoke (Phase 2 — Identity Unification).
//!
//! Mounted at `/api/v1/admin/memberships`. The endpoints here are the ONE
//! sanctioned path to grant or revoke a `user_memberships` row (defends
//! leak #9). Capability-based authorization proper lands in Phase 5; for now
//! we gate on a stub `require_platform_principal` that requires the caller's
//! `RequestPrincipal` to be of `PrincipalKind::Platform`.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use db::models::membership::GrantMembership;
use db::repositories::{ConsumeInviteOutcome, MembershipRepository, UserInviteRepository};
use rand::Rng;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::state::AppState;
use api_core::extractors::principal::RequestPrincipal;

/// Sub-router. Merged into `super::router()`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/memberships/invite", post(invite))
        .route("/memberships/accept", post(accept))
        .route("/memberships/{user_id}", delete(revoke))
}

/// Phase 2 stub: capability gating proper lands in Phase 5. Until then, only
/// `Platform` principals can mutate memberships from this endpoint surface.
fn require_platform_principal(p: &RequestPrincipal) -> Result<(), (StatusCode, &'static str)> {
    if p.is_platform() {
        Ok(())
    } else {
        Err((
            StatusCode::FORBIDDEN,
            "platform principal required (Phase 5 capability gating not yet wired)",
        ))
    }
}

// ====================================================================
// Invite
// ====================================================================

#[derive(Debug, Deserialize, ToSchema)]
pub struct InviteRequest {
    pub email: String,
    pub organization_id: Uuid,
    pub role: String,
    /// Optional explicit TTL in days. Defaults to 7.
    #[serde(default)]
    pub ttl_days: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct InviteResponse {
    pub id: Uuid,
    pub email: String,
    pub organization_id: Uuid,
    pub role: String,
    pub expires_at: DateTime<Utc>,
    /// Plaintext token — shown ONCE here. Store-and-show server side never again.
    pub token: String,
}

pub async fn invite(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Json(req): Json<InviteRequest>,
) -> Result<(StatusCode, Json<InviteResponse>), (StatusCode, &'static str)> {
    require_platform_principal(&principal)?;

    if req.email.trim().is_empty() || !req.email.contains('@') {
        return Err((StatusCode::BAD_REQUEST, "invalid email"));
    }
    if req.role.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "role required"));
    }

    let token = generate_token();
    let ttl = chrono::Duration::days(req.ttl_days.unwrap_or(7).max(1));

    let repo = UserInviteRepository::new(state.db.clone());
    let invite = repo
        .create(
            &req.email,
            req.organization_id,
            &req.role,
            Some(principal.user_id),
            &token,
            ttl,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to create invite");
            (StatusCode::INTERNAL_SERVER_ERROR, "invite create failed")
        })?;

    Ok((
        StatusCode::CREATED,
        Json(InviteResponse {
            id: invite.id,
            email: invite.email,
            organization_id: invite.organization_id,
            role: invite.role,
            expires_at: invite.expires_at,
            token,
        }),
    ))
}

// ====================================================================
// Accept
// ====================================================================

#[derive(Debug, Deserialize, ToSchema)]
pub struct AcceptRequest {
    pub token: String,
    /// The accepting user's id (the request principal). The handler also
    /// verifies the principal's email matches the invite (single-use, email-bound).
    pub user_id: Uuid,
    pub email: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AcceptResponse {
    pub user_id: Uuid,
    pub organization_id: Uuid,
    pub role: String,
}

pub async fn accept(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Json(req): Json<AcceptRequest>,
) -> Result<Json<AcceptResponse>, (StatusCode, &'static str)> {
    // The accepting principal MUST be the same user as the one named in the
    // body — defense-in-depth, since `consume_by_token` already verifies the
    // email match. We allow Platform principals to accept on behalf of a user
    // (admin-side flow), but not other staff/public principals.
    if !principal.is_platform() && principal.user_id != req.user_id {
        return Err((
            StatusCode::FORBIDDEN,
            "cannot accept invite on behalf of another user",
        ));
    }

    let invite_repo = UserInviteRepository::new(state.db.clone());
    let outcome = invite_repo
        .consume_by_token(&req.token, req.user_id, &req.email)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to consume invite");
            (StatusCode::INTERNAL_SERVER_ERROR, "invite accept failed")
        })?;

    let invite = match outcome {
        ConsumeInviteOutcome::Accepted(i) => i,
        ConsumeInviteOutcome::UnknownToken => {
            return Err((StatusCode::NOT_FOUND, "invite not found"))
        }
        ConsumeInviteOutcome::AlreadyAccepted => {
            return Err((StatusCode::CONFLICT, "invite already accepted"))
        }
        ConsumeInviteOutcome::Expired => {
            return Err((StatusCode::GONE, "invite expired"))
        }
        ConsumeInviteOutcome::EmailMismatch => {
            return Err((StatusCode::FORBIDDEN, "invite email does not match"))
        }
    };

    // Acceptance succeeded — write the membership grant in the same flow.
    let mem_repo = MembershipRepository::new(state.db.clone());
    let grant = mem_repo
        .grant(GrantMembership {
            user_id: req.user_id,
            organization_id: invite.organization_id,
            role: invite.role.clone(),
            granted_by: invite.invited_by,
            expires_at: None,
        })
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to write membership after accept");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "membership grant failed",
            )
        })?;

    Ok(Json(AcceptResponse {
        user_id: grant.user_id,
        organization_id: grant.organization_id,
        role: grant.role,
    }))
}

// ====================================================================
// Revoke
// ====================================================================

#[derive(Debug, Deserialize, ToSchema)]
pub struct RevokeRequest {
    pub organization_id: Uuid,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RevokeResponse {
    pub user_id: Uuid,
    pub organization_id: Uuid,
    pub revoked_roles: Vec<String>,
}

pub async fn revoke(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(user_id): Path<Uuid>,
    Json(req): Json<RevokeRequest>,
) -> Result<Json<RevokeResponse>, (StatusCode, &'static str)> {
    require_platform_principal(&principal)?;

    let mem_repo = MembershipRepository::new(state.db.clone());
    let revoked = mem_repo
        .revoke(user_id, req.organization_id, Some(principal.user_id))
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to revoke memberships");
            (StatusCode::INTERNAL_SERVER_ERROR, "revoke failed")
        })?;

    Ok(Json(RevokeResponse {
        user_id,
        organization_id: req.organization_id,
        revoked_roles: revoked,
    }))
}

// ====================================================================
// Helpers
// ====================================================================

/// Generate a 32-byte URL-safe token. Plaintext is shown once at create.
fn generate_token() -> String {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.gen();
    use base64::Engine as _;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}
