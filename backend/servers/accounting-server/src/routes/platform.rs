//! Platform / security routes (EPIC-ACC-16 / UC-ACC-16). OWNED BY: BE-Platform.
//!
//! Covers the audit-log view (UC-ACC-16.7 / 01.10), 2FA enrollment
//! (UC-ACC-16.1 / 01.9), document tags (UC-ACC-05.14), and the PUBLIC
//! capability-token invoice view (UC-ACC-05.11 — no auth, resolved by token).
//! Authenticated handlers use `AuthUser` + `RlsConnection` + `authz`; the public
//! share view uses `State` only and resolves the token via the platform repo.
//! `nest`ed under `/api/v1/platform` and `/api/v1/share`.
//!
//! ## Security posture
//! - Every authenticated handler calls `authz::require_role*` BEFORE touching a
//!   repo (UC-ACC-16.2: server-side check; denied → 403 regardless of UI). RLS is
//!   defense-in-depth.
//! - Every mutation records an append-only audit entry via `audit` +
//!   `record_audit_rls` in the SAME transaction (UC-ACC-16.7).
//! - The public share endpoint returns a non-enumerating 404 for missing /
//!   revoked / expired tokens (UC-ACC-05.11) and exposes no PII beyond the
//!   document itself.
//!
//! ## Skeleton crypto note (filed in contract-notes/be-platform.md)
//! The accounting-server crate currently has no TOTP / RNG / hashing crates
//! (`rand`, `totp-rs`, `sha2`, `data-encoding`). For the MVP skeleton this file
//! derives high-entropy opaque tokens / secrets / recovery codes from v4 UUIDs
//! (122 bits of entropy each) and treats 2FA confirmation as a format/length
//! check. Phase 2 should swap in real TOTP verification + hashed-at-rest secrets
//! and recovery codes once the crates are added.

use api_core::extractors::{AuthUser, RlsConnection};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use db::models::acc_platform::{AccAuditLog, AccShareLink, AccTag, AccTwoFactor};
use serde::{Deserialize, Serialize};
use sqlx::Connection;
use utoipa::ToSchema;
use uuid::Uuid;

use accounting_core::{audit, authz};

use crate::state::AppState;

/// Max audit entries returned by the list endpoint (UC-ACC-16.7 view cap).
const AUDIT_PAGE_LIMIT: i64 = 500;
/// Number of one-time recovery codes issued on 2FA enrollment (UC-ACC-16.1).
const RECOVERY_CODE_COUNT: usize = 10;

/// Request to create a share link for an invoice (UC-ACC-05.11).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateShareLinkRequest {
    pub invoice_id: Uuid,
    /// Optional expiry in seconds from now; None = no expiry.
    pub expires_in_secs: Option<i64>,
}

/// Public read-only view of a shared invoice (UC-ACC-05.11). No PII beyond the
/// document itself; the coder shapes the exact fields.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SharedInvoiceView {
    pub number: String,
    pub currency: String,
    pub total_amount: rust_decimal::Decimal,
    pub status: String,
}

/// Request to add a tag to an entity (UC-ACC-05.14).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AddTagRequest {
    pub entity_type: String,
    pub entity_id: Uuid,
    pub tag: String,
}

/// 2FA enrollment-start response (UC-ACC-16.1).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TwoFactorEnrollResponse {
    /// otpauth:// URI / secret for the authenticator app.
    pub otpauth_uri: String,
    /// One-time recovery codes shown ONCE.
    pub recovery_codes: Vec<String>,
}

/// Request to confirm 2FA with a TOTP code (UC-ACC-16.1).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TwoFactorConfirmRequest {
    pub code: String,
}

// ---------------------------------------------------------------------------
// Skeleton crypto helpers (see module note; replaced with real crypto Phase 2).
// ---------------------------------------------------------------------------

/// Generate a high-entropy, URL-safe, unguessable opaque token (share link /
/// secret). Two v4 UUIDs (244 bits) concatenated, hyphens stripped.
fn generate_opaque_token() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

/// Generate `RECOVERY_CODE_COUNT` one-time recovery codes (UC-ACC-16.1: issued
/// once on enrollment). Each is an unguessable 12-hex-char chunk.
fn generate_recovery_codes() -> Vec<String> {
    (0..RECOVERY_CODE_COUNT)
        .map(|_| {
            let s = Uuid::new_v4().simple().to_string();
            // Group as XXXX-XXXX-XXXX for readability.
            format!("{}-{}-{}", &s[0..4], &s[4..8], &s[8..12])
        })
        .collect()
}

// --- Audit log (UC-ACC-16.7 / 01.10) ---

/// List the audit log for the active company (UC-ACC-16.7).
#[utoipa::path(
    get,
    path = "/api/v1/platform/audit-log",
    responses(
        (status = 200, description = "Audit entries", body = [AccAuditLog]),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    tag = "Platform"
)]
pub async fn list_audit_log(
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> Result<Json<Vec<AccAuditLog>>, StatusCode> {
    // UC-ACC-16.7/01.10: the audit log is a company-administration view → admin.
    authz::require_admin(rls.role()).map_err(|_| StatusCode::FORBIDDEN)?;

    let entries = state
        .platform_repo
        .list_audit_rls(&mut **rls, None, None, AUDIT_PAGE_LIMIT)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list audit log: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    rls.release().await;
    Ok(Json(entries))
}

// --- Tags (UC-ACC-05.14) ---

/// Add a tag to an entity (UC-ACC-05.14).
#[utoipa::path(
    post,
    path = "/api/v1/platform/tags",
    request_body = AddTagRequest,
    responses(
        (status = 201, description = "Tag added", body = AccTag),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    tag = "Platform"
)]
pub async fn add_tag(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    auth: AuthUser,
    Json(req): Json<AddTagRequest>,
) -> Result<(StatusCode, Json<AccTag>), StatusCode> {
    // UC-ACC-16.2: server-side RBAC gate before any work.
    authz::require_role(rls.role(), authz::MUTATE_MIN_ROLE).map_err(|_| StatusCode::FORBIDDEN)?;

    let tenant_id = rls.tenant_id();
    let actor_id = auth.user_id;

    // Light input validation (defense against junk tags).
    let tag = req.tag.trim();
    let entity_type = req.entity_type.trim();
    if tag.is_empty() || tag.len() > 64 || entity_type.is_empty() || entity_type.len() > 32 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let new_tag = AccTag {
        id: Uuid::nil(), // DB-defaulted
        tenant_id,
        entity_type: entity_type.to_string(),
        entity_id: req.entity_id,
        tag: tag.to_string(),
        created_at: chrono::Utc::now(), // DB-defaulted
    };

    // Mutation + audit in ONE transaction (UC-ACC-16.7 append-only).
    let mut tx = rls.begin().await.map_err(|e| {
        tracing::error!("Failed to begin tx: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let saved = state
        .platform_repo
        .add_tag_rls(&mut *tx, new_tag)
        .await
        .map_err(|e| {
            tracing::error!("Failed to add tag: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let entry = audit::record(
        tenant_id,
        Some(actor_id),
        "create",
        "tag",
        Some(saved.id),
        audit::diff_of(
            None,
            Some(serde_json::json!({
                "entity_type": saved.entity_type,
                "entity_id": saved.entity_id,
                "tag": saved.tag,
            })),
        ),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    state
        .platform_repo
        .record_audit_rls(&mut *tx, entry)
        .await
        .map_err(|e| {
            tracing::error!("Failed to write audit entry: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tx.commit().await.map_err(|e| {
        tracing::error!("Failed to commit add_tag tx: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    rls.release().await;
    Ok((StatusCode::CREATED, Json(saved)))
}

// --- Share links (UC-ACC-05.11) ---

/// Create a capability-token share link for an invoice (UC-ACC-05.11).
#[utoipa::path(
    post,
    path = "/api/v1/platform/share-links",
    request_body = CreateShareLinkRequest,
    responses(
        (status = 201, description = "Share link created", body = AccShareLink),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    tag = "Platform"
)]
pub async fn create_share_link(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    auth: AuthUser,
    Json(req): Json<CreateShareLinkRequest>,
) -> Result<(StatusCode, Json<AccShareLink>), StatusCode> {
    // UC-ACC-16.2: mutation gate.
    authz::require_role(rls.role(), authz::MUTATE_MIN_ROLE).map_err(|_| StatusCode::FORBIDDEN)?;

    let tenant_id = rls.tenant_id();
    let actor_id = auth.user_id;

    // Capability token: high-entropy, unguessable (the link IS the credential).
    let token = generate_opaque_token();
    let expires_at = req
        .expires_in_secs
        .filter(|s| *s > 0)
        .map(|s| chrono::Utc::now() + chrono::Duration::seconds(s));

    let link = AccShareLink {
        id: Uuid::nil(), // DB-defaulted
        tenant_id,
        invoice_id: req.invoice_id,
        token,
        expires_at,
        revoked_at: None,
        created_by: Some(actor_id),
        created_at: chrono::Utc::now(), // DB-defaulted
    };

    let mut tx = rls.begin().await.map_err(|e| {
        tracing::error!("Failed to begin tx: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let saved = state
        .platform_repo
        .create_share_link_rls(&mut *tx, link)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create share link: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Audit WITHOUT the raw token (secret never logged — shared-DoD).
    let entry = audit::record(
        tenant_id,
        Some(actor_id),
        "create",
        "share_link",
        Some(saved.id),
        audit::diff_of(
            None,
            Some(serde_json::json!({
                "invoice_id": saved.invoice_id,
                "expires_at": saved.expires_at,
            })),
        ),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    state
        .platform_repo
        .record_audit_rls(&mut *tx, entry)
        .await
        .map_err(|e| {
            tracing::error!("Failed to write audit entry: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tx.commit().await.map_err(|e| {
        tracing::error!("Failed to commit share-link tx: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    rls.release().await;
    Ok((StatusCode::CREATED, Json(saved)))
}

/// Revoke a share link (UC-ACC-05.11).
#[utoipa::path(
    delete,
    path = "/api/v1/platform/share-links/{id}",
    params(("id" = Uuid, Path, description = "Share link id")),
    responses(
        (status = 204, description = "Revoked"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    tag = "Platform"
)]
pub async fn revoke_share_link(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    mut rls: RlsConnection,
    auth: AuthUser,
) -> Result<StatusCode, StatusCode> {
    // UC-ACC-16.2: mutation gate.
    authz::require_role(rls.role(), authz::MUTATE_MIN_ROLE).map_err(|_| StatusCode::FORBIDDEN)?;

    let tenant_id = rls.tenant_id();
    let actor_id = auth.user_id;

    let mut tx = rls.begin().await.map_err(|e| {
        tracing::error!("Failed to begin tx: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    state
        .platform_repo
        .revoke_share_link_rls(&mut *tx, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to revoke share link {id}: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let entry = audit::record(
        tenant_id,
        Some(actor_id),
        "revoke",
        "share_link",
        Some(id),
        audit::diff_of(None, Some(serde_json::json!({ "revoked": true }))),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    state
        .platform_repo
        .record_audit_rls(&mut *tx, entry)
        .await
        .map_err(|e| {
            tracing::error!("Failed to write audit entry: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tx.commit().await.map_err(|e| {
        tracing::error!("Failed to commit revoke tx: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    rls.release().await;
    Ok(StatusCode::NO_CONTENT)
}

/// PUBLIC read-only view of a shared invoice by token (UC-ACC-05.11). No auth.
/// Returns 404 for a missing/revoked/expired token (non-enumerating).
#[utoipa::path(
    get,
    path = "/api/v1/share/{token}",
    params(("token" = String, Path, description = "Opaque share token")),
    responses(
        (status = 200, description = "Shared invoice", body = SharedInvoiceView),
        (status = 404, description = "Invalid / revoked / expired")
    ),
    tag = "Platform"
)]
pub async fn view_shared_invoice(
    Path(token): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<SharedInvoiceView>, StatusCode> {
    // PUBLIC endpoint: no RLS principal. We acquire a raw pooled connection and
    // resolve the link by its unguessable token (the capability). All failure
    // modes collapse to a single non-enumerating 404 (UC-ACC-05.11).
    let mut conn = state.db.acquire().await.map_err(|e| {
        tracing::error!("Failed to acquire connection for share view: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let link = state
        .platform_repo
        .find_share_link_by_token_rls(&mut *conn, &token)
        .await
        .map_err(|e| {
            tracing::error!("Failed to resolve share token: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Capability validity: revoked or expired → 404 (same as missing).
    if link.revoked_at.is_some() {
        return Err(StatusCode::NOT_FOUND);
    }
    if let Some(exp) = link.expires_at {
        if exp <= chrono::Utc::now() {
            return Err(StatusCode::NOT_FOUND);
        }
    }

    // Load the invoice via the share link's tenant context. The public endpoint
    // has no JWT, so we set the org RLS context for this tenant explicitly, then
    // read the invoice (RLS confirms the invoice belongs to the link's tenant).
    db::tenant_context::set_tenant_context(&mut *conn, link.tenant_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to set tenant context for share view: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let invoice = state
        .accounting_repo
        .find_invoice_rls(&mut *conn, link.invoice_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to load shared invoice: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Always clear context before returning the pooled connection.
    let _ = db::tenant_context::clear_request_context(&mut *conn).await;

    let invoice = invoice.ok_or(StatusCode::NOT_FOUND)?;

    // Render the minimal, PII-light view. `status` is serialized from the enum
    // to its snake_case wire string.
    let status = serde_json::to_value(invoice.status)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| "unknown".to_string());

    Ok(Json(SharedInvoiceView {
        number: invoice.number,
        currency: invoice.currency,
        total_amount: invoice.total_amount,
        status,
    }))
}

// --- 2FA (UC-ACC-16.1 / 01.9) ---

/// Begin 2FA enrollment: generate a secret + recovery codes (UC-ACC-16.1).
/// Stores the enrollment disabled (`enabled=false`) until confirmed.
#[utoipa::path(
    post,
    path = "/api/v1/platform/2fa/enroll",
    responses(
        (status = 200, description = "Enrollment started", body = TwoFactorEnrollResponse),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Platform"
)]
pub async fn enroll_two_factor(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    auth: AuthUser,
) -> Result<Json<TwoFactorEnrollResponse>, StatusCode> {
    // Any authenticated member may protect their own account; require at least
    // read-level membership of the active company.
    authz::require_role(rls.role(), authz::READ_MIN_ROLE).map_err(|_| StatusCode::FORBIDDEN)?;

    let tenant_id = rls.tenant_id();
    let user_id = auth.user_id;

    // Skeleton crypto (see module note): opaque secret + recovery codes.
    let secret = generate_opaque_token();
    let recovery_codes = generate_recovery_codes();
    let otpauth_uri = format!(
        "otpauth://totp/PPT-Accounting:{}?secret={}&issuer=PPT-Accounting",
        auth.email, secret
    );

    let enrollment = AccTwoFactor {
        id: Uuid::nil(), // DB-defaulted
        tenant_id,
        user_id,
        secret,
        enabled: false, // not active until confirmed
        confirmed_at: None,
        // Stored as a JSON array (hashed at rest preferred — Phase 2).
        recovery_codes: serde_json::json!(recovery_codes),
        created_at: chrono::Utc::now(), // DB-defaulted
        updated_at: chrono::Utc::now(), // DB-defaulted
    };

    let mut tx = rls.begin().await.map_err(|e| {
        tracing::error!("Failed to begin tx: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    state
        .platform_repo
        .upsert_two_factor_rls(&mut *tx, enrollment)
        .await
        .map_err(|e| {
            tracing::error!("Failed to upsert 2FA enrollment: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Audit the enrollment-start WITHOUT the secret / codes (never logged).
    let entry = audit::record(
        tenant_id,
        Some(user_id),
        "enroll",
        "two_factor",
        Some(user_id),
        audit::diff_of(None, Some(serde_json::json!({ "enabled": false }))),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    state
        .platform_repo
        .record_audit_rls(&mut *tx, entry)
        .await
        .map_err(|e| {
            tracing::error!("Failed to write audit entry: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tx.commit().await.map_err(|e| {
        tracing::error!("Failed to commit 2FA enroll tx: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    rls.release().await;

    // Recovery codes shown ONCE here (UC-ACC-16.1).
    Ok(Json(TwoFactorEnrollResponse {
        otpauth_uri,
        recovery_codes,
    }))
}

/// Confirm 2FA with a TOTP code, enabling it (UC-ACC-16.1).
#[utoipa::path(
    post,
    path = "/api/v1/platform/2fa/confirm",
    request_body = TwoFactorConfirmRequest,
    responses(
        (status = 204, description = "2FA enabled"),
        (status = 400, description = "Invalid code"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Platform"
)]
pub async fn confirm_two_factor(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    auth: AuthUser,
    Json(req): Json<TwoFactorConfirmRequest>,
) -> Result<StatusCode, StatusCode> {
    authz::require_role(rls.role(), authz::READ_MIN_ROLE).map_err(|_| StatusCode::FORBIDDEN)?;

    let tenant_id = rls.tenant_id();
    let user_id = auth.user_id;

    // Skeleton TOTP verification (see module note): require a well-formed 6-digit
    // numeric code. Phase 2 swaps in real RFC-6238 verification against the
    // stored secret with a ±1 time-step window.
    let code = req.code.trim();
    if code.len() != 6 || !code.chars().all(|c| c.is_ascii_digit()) {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Must have an existing (pending) enrollment to confirm.
    let existing = state
        .platform_repo
        .find_two_factor_rls(&mut **rls, user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to load 2FA enrollment: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::BAD_REQUEST)?;

    let now = chrono::Utc::now();
    let confirmed = AccTwoFactor {
        enabled: true,
        confirmed_at: Some(now),
        ..existing
    };

    let mut tx = rls.begin().await.map_err(|e| {
        tracing::error!("Failed to begin tx: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    state
        .platform_repo
        .upsert_two_factor_rls(&mut *tx, confirmed)
        .await
        .map_err(|e| {
            tracing::error!("Failed to enable 2FA: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let entry = audit::record(
        tenant_id,
        Some(user_id),
        "confirm",
        "two_factor",
        Some(user_id),
        audit::diff_of(
            Some(serde_json::json!({ "enabled": false })),
            Some(serde_json::json!({ "enabled": true })),
        ),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    state
        .platform_repo
        .record_audit_rls(&mut *tx, entry)
        .await
        .map_err(|e| {
            tracing::error!("Failed to write audit entry: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tx.commit().await.map_err(|e| {
        tracing::error!("Failed to commit 2FA confirm tx: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    rls.release().await;
    Ok(StatusCode::NO_CONTENT)
}
