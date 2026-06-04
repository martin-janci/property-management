//! Organization settings, branding, data-export, and feature-preference routes.

use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, put},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::models::UpdateOrganization;
use serde::{Deserialize, Serialize};
use tenant_ops::{export_tenant, TenantDataManifest};
use utoipa::ToSchema;
use uuid::Uuid;

use super::core::{extract_bearer_token, validate_access_token, OrganizationResponse};
use crate::state::AppState;

/// Create organizations settings router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/{id}/settings", get(get_organization_settings))
        .route("/{id}/settings", put(update_organization_settings))
        .route("/{id}/branding", get(get_organization_branding))
        .route("/{id}/branding", put(update_organization_branding))
        .route("/{id}/export", get(export_organization_data))
        .route("/{id}/features", get(list_organization_features))
        .route("/{id}/features", put(bulk_update_organization_features))
        .route("/{id}/features/{key}", put(toggle_organization_feature))
}

// ==================== Organization Settings (Story 2A.4) ====================

/// Organization settings response.
#[derive(Debug, Serialize, ToSchema)]
pub struct OrganizationSettingsResponse {
    /// Organization ID
    pub organization_id: Uuid,
    /// Settings JSON object
    pub settings: serde_json::Value,
}

/// Update organization settings request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateOrganizationSettingsRequest {
    /// Settings to update (will be merged with existing)
    pub settings: serde_json::Value,
}

/// Get organization settings.
#[utoipa::path(
    get,
    path = "/api/v1/organizations/{id}/settings",
    tag = "Organizations",
    params(
        ("id" = Uuid, Path, description = "Organization ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Settings retrieved", body = OrganizationSettingsResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Organization not found", body = ErrorResponse)
    )
)]
pub async fn get_organization_settings(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<OrganizationSettingsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let token = extract_bearer_token(&headers)?;
    let claims = validate_access_token(&state, &token)?;

    let user_id: Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    // Check if user is member of this organization
    match state
        .org_member_repo
        .find_by_org_and_user_rls(&mut **rls.conn(), id, user_id)
        .await
    {
        Ok(Some(_)) => {}
        Ok(None) => {
            rls.release().await;
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new(
                    "NOT_MEMBER",
                    "You are not a member of this organization",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to check membership");
            rls.release().await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to check membership",
                )),
            ));
        }
    }

    let org = match state.org_repo.find_by_id_rls(&mut **rls.conn(), id).await {
        Ok(Some(org)) => org,
        Ok(None) => {
            rls.release().await;
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "ORG_NOT_FOUND",
                    "Organization not found",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch organization");
            rls.release().await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to fetch organization",
                )),
            ));
        }
    };

    rls.release().await;
    Ok(Json(OrganizationSettingsResponse {
        organization_id: id,
        settings: org.settings,
    }))
}

/// Update organization settings.
#[utoipa::path(
    put,
    path = "/api/v1/organizations/{id}/settings",
    tag = "Organizations",
    params(
        ("id" = Uuid, Path, description = "Organization ID")
    ),
    request_body = UpdateOrganizationSettingsRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Settings updated", body = OrganizationSettingsResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Organization not found", body = ErrorResponse)
    )
)]
pub async fn update_organization_settings(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateOrganizationSettingsRequest>,
) -> Result<Json<OrganizationSettingsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let token = extract_bearer_token(&headers)?;
    let claims = validate_access_token(&state, &token)?;

    let user_id: Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    // Check membership and permissions
    let membership = match state
        .org_member_repo
        .find_by_org_and_user_rls(&mut **rls.conn(), id, user_id)
        .await
    {
        Ok(Some(m)) => m,
        Ok(None) => {
            rls.release().await;
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new(
                    "NOT_MEMBER",
                    "You are not a member of this organization",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to check membership");
            rls.release().await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to check membership",
                )),
            ));
        }
    };

    let role = match membership.role_id {
        Some(role_id) => match state
            .role_repo
            .find_by_id_rls(&mut **rls.conn(), role_id)
            .await
        {
            Ok(Some(r)) => r,
            _ => {
                rls.release().await;
                return Err((
                    StatusCode::FORBIDDEN,
                    Json(ErrorResponse::new("ROLE_NOT_FOUND", "User role not found")),
                ));
            }
        },
        None => {
            rls.release().await;
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new("NO_ROLE", "User has no role assigned")),
            ));
        }
    };

    if !role.has_permission("organization:update") {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "PERMISSION_DENIED",
                "You do not have permission to update organization settings",
            )),
        ));
    }

    // Update settings
    let update = UpdateOrganization {
        name: None,
        contact_email: None,
        logo_url: None,
        primary_color: None,
        settings: Some(req.settings.clone()),
    };

    let org = match state
        .org_repo
        .update_rls(&mut **rls.conn(), id, update)
        .await
    {
        Ok(Some(org)) => org,
        Ok(None) => {
            rls.release().await;
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "ORG_NOT_FOUND",
                    "Organization not found",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to update organization settings");
            rls.release().await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to update settings",
                )),
            ));
        }
    };

    tracing::info!(org_id = %id, user_id = %user_id, "Organization settings updated");

    rls.release().await;
    Ok(Json(OrganizationSettingsResponse {
        organization_id: id,
        settings: org.settings,
    }))
}

// ==================== Organization Branding (Story 2A.4) ====================

/// Organization branding response.
#[derive(Debug, Serialize, ToSchema)]
pub struct OrganizationBrandingResponse {
    /// Organization ID
    pub organization_id: Uuid,
    /// Logo URL
    pub logo_url: Option<String>,
    /// Primary brand color (hex)
    pub primary_color: Option<String>,
    /// Organization name (for branding)
    pub name: String,
}

/// Update organization branding request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateOrganizationBrandingRequest {
    /// Logo URL
    pub logo_url: Option<String>,
    /// Primary brand color (hex format, e.g., "#FF5733")
    pub primary_color: Option<String>,
}

/// Get organization branding.
#[utoipa::path(
    get,
    path = "/api/v1/organizations/{id}/branding",
    tag = "Organizations",
    params(
        ("id" = Uuid, Path, description = "Organization ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Branding retrieved", body = OrganizationBrandingResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Organization not found", body = ErrorResponse)
    )
)]
pub async fn get_organization_branding(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<OrganizationBrandingResponse>, (StatusCode, Json<ErrorResponse>)> {
    let token = extract_bearer_token(&headers)?;
    let claims = validate_access_token(&state, &token)?;

    let user_id: Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    // Check if user is member of this organization
    match state
        .org_member_repo
        .find_by_org_and_user_rls(&mut **rls.conn(), id, user_id)
        .await
    {
        Ok(Some(_)) => {}
        Ok(None) => {
            rls.release().await;
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new(
                    "NOT_MEMBER",
                    "You are not a member of this organization",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to check membership");
            rls.release().await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to check membership",
                )),
            ));
        }
    }

    let org = match state.org_repo.find_by_id_rls(&mut **rls.conn(), id).await {
        Ok(Some(org)) => org,
        Ok(None) => {
            rls.release().await;
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "ORG_NOT_FOUND",
                    "Organization not found",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch organization");
            rls.release().await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to fetch organization",
                )),
            ));
        }
    };

    rls.release().await;
    Ok(Json(OrganizationBrandingResponse {
        organization_id: id,
        logo_url: org.logo_url,
        primary_color: org.primary_color,
        name: org.name,
    }))
}

/// Update organization branding.
#[utoipa::path(
    put,
    path = "/api/v1/organizations/{id}/branding",
    tag = "Organizations",
    params(
        ("id" = Uuid, Path, description = "Organization ID")
    ),
    request_body = UpdateOrganizationBrandingRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Branding updated", body = OrganizationBrandingResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Organization not found", body = ErrorResponse)
    )
)]
pub async fn update_organization_branding(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateOrganizationBrandingRequest>,
) -> Result<Json<OrganizationBrandingResponse>, (StatusCode, Json<ErrorResponse>)> {
    let token = extract_bearer_token(&headers)?;
    let claims = validate_access_token(&state, &token)?;

    let user_id: Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    // Validate color format if provided
    if let Some(ref color) = req.primary_color {
        if !is_valid_hex_color(color) {
            rls.release().await;
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "INVALID_COLOR",
                    "Primary color must be a valid hex color (e.g., #FF5733)",
                )),
            ));
        }
    }

    // Check membership and permissions
    let membership = match state
        .org_member_repo
        .find_by_org_and_user_rls(&mut **rls.conn(), id, user_id)
        .await
    {
        Ok(Some(m)) => m,
        Ok(None) => {
            rls.release().await;
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new(
                    "NOT_MEMBER",
                    "You are not a member of this organization",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to check membership");
            rls.release().await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to check membership",
                )),
            ));
        }
    };

    let role = match membership.role_id {
        Some(role_id) => match state
            .role_repo
            .find_by_id_rls(&mut **rls.conn(), role_id)
            .await
        {
            Ok(Some(r)) => r,
            _ => {
                rls.release().await;
                return Err((
                    StatusCode::FORBIDDEN,
                    Json(ErrorResponse::new("ROLE_NOT_FOUND", "User role not found")),
                ));
            }
        },
        None => {
            rls.release().await;
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new("NO_ROLE", "User has no role assigned")),
            ));
        }
    };

    if !role.has_permission("organization:update") {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "PERMISSION_DENIED",
                "You do not have permission to update organization branding",
            )),
        ));
    }

    // Update branding
    let update = UpdateOrganization {
        name: None,
        contact_email: None,
        logo_url: req.logo_url.clone(),
        primary_color: req.primary_color.clone(),
        settings: None,
    };

    let org = match state
        .org_repo
        .update_rls(&mut **rls.conn(), id, update)
        .await
    {
        Ok(Some(org)) => org,
        Ok(None) => {
            rls.release().await;
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "ORG_NOT_FOUND",
                    "Organization not found",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to update organization branding");
            rls.release().await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to update branding",
                )),
            ));
        }
    };

    tracing::info!(org_id = %id, user_id = %user_id, "Organization branding updated");

    rls.release().await;
    Ok(Json(OrganizationBrandingResponse {
        organization_id: id,
        logo_url: org.logo_url,
        primary_color: org.primary_color,
        name: org.name,
    }))
}

/// Validate hex color format.
fn is_valid_hex_color(color: &str) -> bool {
    if !color.starts_with('#') {
        return false;
    }
    let hex = &color[1..];
    (hex.len() == 3 || hex.len() == 6) && hex.chars().all(|c| c.is_ascii_hexdigit())
}

/// Escape a field for CSV output to prevent injection and handle special characters.
/// - Fields containing commas, quotes, or newlines are wrapped in quotes
/// - Double quotes within fields are escaped by doubling them
/// - Fields starting with =, +, -, @ are prefixed with a single quote to prevent formula injection
fn escape_csv_field(field: &str) -> String {
    // Check for formula injection characters at start
    let needs_quote_prefix = field.starts_with('=')
        || field.starts_with('+')
        || field.starts_with('-')
        || field.starts_with('@');

    // Check if field needs quoting
    let needs_quotes = field.contains(',')
        || field.contains('"')
        || field.contains('\n')
        || field.contains('\r')
        || needs_quote_prefix;

    if needs_quotes {
        // Escape double quotes by doubling them
        let escaped = field.replace('"', "\"\"");
        if needs_quote_prefix {
            // Prefix with single quote to prevent formula execution
            format!("\"'{}\"", escaped)
        } else {
            format!("\"{}\"", escaped)
        }
    } else {
        field.to_string()
    }
}

/// Maximum number of members to export at once
const MAX_EXPORT_MEMBERS: i64 = 5000;

// ==================== Organization Data Export (Story 2A.7) ====================

/// Export member data.
#[derive(Debug, Serialize, ToSchema)]
pub struct ExportMember {
    /// User ID
    pub user_id: Uuid,
    /// User email
    pub email: Option<String>,
    /// User name
    pub name: Option<String>,
    /// Role name
    pub role_name: Option<String>,
    /// Role type
    pub role_type: String,
    /// Member status
    pub status: String,
    /// Join date
    pub joined_at: String,
}

/// Export role data.
#[derive(Debug, Serialize, ToSchema)]
pub struct ExportRole {
    /// Role ID
    pub id: Uuid,
    /// Role name
    pub name: String,
    /// Role description
    pub description: Option<String>,
    /// Permissions
    pub permissions: Vec<String>,
    /// Is system role
    pub is_system: bool,
}

/// Organization export query parameters.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ExportQuery {
    /// Export format (json or csv) for the synchronous response.
    #[serde(default = "default_format")]
    pub format: String,
    /// Issue #979: when true, runs a **full** multi-table export as a
    /// background job (reusing the tenant-ops exporter), uploads the resulting
    /// `.tar.gz` to S3, and emails the requester a 7-day download link.
    /// Returns `202 Accepted` immediately. When false/absent, the legacy
    /// synchronous members+roles JSON/CSV body is returned unchanged.
    #[serde(default)]
    pub background: bool,
}

fn default_format() -> String {
    "json".to_string()
}

/// Organization data export response.
#[derive(Debug, Serialize, ToSchema)]
pub struct OrganizationExportResponse {
    /// Export timestamp
    pub exported_at: String,
    /// Organization info
    pub organization: OrganizationResponse,
    /// Members list
    pub members: Vec<ExportMember>,
    /// Roles list
    pub roles: Vec<ExportRole>,
    /// Total members count
    pub total_members: usize,
    /// Total roles count
    pub total_roles: usize,
}

/// Export organization data.
#[utoipa::path(
    get,
    path = "/api/v1/organizations/{id}/export",
    tag = "Organizations",
    params(
        ("id" = Uuid, Path, description = "Organization ID"),
        ("format" = Option<String>, Query, description = "Export format: json (default) or csv")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Data exported", body = OrganizationExportResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Organization not found", body = ErrorResponse)
    )
)]
pub async fn export_organization_data(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    axum::extract::Query(query): axum::extract::Query<ExportQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let token = extract_bearer_token(&headers)?;
    let claims = validate_access_token(&state, &token)?;

    let user_id: Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    // Check membership and permissions using RLS
    let membership = match state
        .org_member_repo
        .find_by_org_and_user_rls(&mut **rls.conn(), id, user_id)
        .await
    {
        Ok(Some(m)) => m,
        Ok(None) => {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new(
                    "NOT_MEMBER",
                    "You are not a member of this organization",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to check membership");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to check membership",
                )),
            ));
        }
    };

    let role = match membership.role_id {
        Some(role_id) => match state
            .role_repo
            .find_by_id_rls(&mut **rls.conn(), role_id)
            .await
        {
            Ok(Some(r)) => r,
            Ok(None) => {
                return Err((
                    StatusCode::FORBIDDEN,
                    Json(ErrorResponse::new("ROLE_NOT_FOUND", "User role not found")),
                ));
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to fetch role");
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new("DATABASE_ERROR", "Failed to fetch role")),
                ));
            }
        },
        None => {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new("NO_ROLE", "User has no role assigned")),
            ));
        }
    };

    // Check permission to export data (org:export or org:read + users:read)
    // Note: has_permission() already checks for wildcard "*" internally
    let can_export = role.has_permission("organization:export")
        || (role.has_permission("organization:read") && role.has_permission("users:read"));

    if !can_export {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "PERMISSION_DENIED",
                "You do not have permission to export organization data",
            )),
        ));
    }

    // Issue #979: full async export. Spawn a detached background job that
    // produces a complete per-table `.tar.gz` (via the tenant-ops exporter),
    // uploads it to S3, and emails the requester a 7-day presigned link. We
    // return 202 immediately rather than blocking the request. The permission
    // check above has already authorised this user to export this org, so the
    // job runs against the service-role pool (RLS-bypassing) for the same org.
    if query.background {
        // Release the RLS connection before the long-running work.
        rls.release().await;

        let pool = state.db.clone();
        let storage = state.storage_service.clone();
        let email = state.email_service.clone();
        let user_repo = state.user_repo.clone();
        let org_id = id;

        tokio::spawn(async move {
            let to_email = match user_repo.find_by_id(user_id).await {
                Ok(Some(u)) => u.email,
                _ => {
                    tracing::error!(user_id = %user_id, "[#979] org export: requester not found; aborting");
                    return;
                }
            };
            let Some(storage) = storage else {
                tracing::error!("[#979] org export: storage not configured; cannot deliver export");
                return;
            };

            // Load the tenant-data manifest (env-overridable, mirrors the
            // admin tenant-lifecycle export path).
            let manifest_path = std::env::var("PPT_TENANT_MANIFEST_PATH")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|_| TenantDataManifest::default_path());
            let manifest = match TenantDataManifest::load(&manifest_path) {
                Ok(m) => m,
                Err(e) => {
                    tracing::error!(error = %e, path = ?manifest_path, "[#979] org export: manifest load failed");
                    return;
                }
            };

            let out_dir = std::env::temp_dir().join(format!("ppt-org-export-{}", Uuid::new_v4()));
            let export = match export_tenant(&pool, org_id, &manifest, &out_dir).await {
                Ok(e) => e,
                Err(e) => {
                    tracing::error!(error = %e, org = %org_id, "[#979] org export: export_tenant failed");
                    let _ = std::fs::remove_dir_all(&out_dir);
                    return;
                }
            };

            let bytes = match std::fs::read(&export.tarball_path) {
                Ok(b) => b,
                Err(e) => {
                    tracing::error!(error = %e, "[#979] org export: reading tarball failed");
                    let _ = std::fs::remove_dir_all(&out_dir);
                    return;
                }
            };

            let key = format!("exports/org-{}/{}-export.tar.gz", org_id, Uuid::new_v4());
            let upload = storage
                .upload_system_artifact(&key, bytes, "application/gzip")
                .await;
            let _ = std::fs::remove_dir_all(&out_dir);
            if let Err(e) = upload {
                tracing::error!(error = %e, "[#979] org export: S3 upload failed");
                return;
            }

            // 7-day presigned download link.
            let presigned = match storage
                .generate_download_url(
                    &key,
                    "organization-export.tar.gz",
                    "application/gzip",
                    Some(7 * 24 * 3600),
                )
                .await
            {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!(error = %e, "[#979] org export: presign failed");
                    return;
                }
            };

            let expires = presigned.expires_at.to_rfc3339();
            let html = format!(
                "<p>Your organization data export is ready.</p>\
                 <p><a href=\"{url}\">Download the export</a> — link valid until {expires}.</p>",
                url = presigned.url,
            );
            let text = format!(
                "Your organization data export is ready.\nDownload (valid until {expires}): {url}",
                url = presigned.url,
            );
            match email
                .send_html_email(
                    &to_email,
                    "Your organization data export is ready",
                    &html,
                    &text,
                )
                .await
            {
                Ok(()) => tracing::info!(
                    org = %org_id,
                    rows = export.rows_exported,
                    tables = export.tables_exported,
                    "[#979] org export delivered via emailed link"
                ),
                Err(e) => tracing::error!(error = %e, "[#979] org export: email send failed"),
            }
        });

        return Ok((
            StatusCode::ACCEPTED,
            Json(serde_json::json!({
                "status": "accepted",
                "message": "Export started. A download link will be emailed when it is ready."
            })),
        )
            .into_response());
    }

    // Get organization using RLS
    let org = match state.org_repo.find_by_id_rls(&mut **rls.conn(), id).await {
        Ok(Some(org)) => org,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "ORG_NOT_FOUND",
                    "Organization not found",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch organization");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to fetch organization",
                )),
            ));
        }
    };

    // Get members with user info (paginated export to prevent memory issues) using RLS
    let (members, _total_member_count) = match state
        .org_member_repo
        .list_org_members_rls(&mut **rls.conn(), id, 0, MAX_EXPORT_MEMBERS, None)
        .await
    {
        Ok(m) => m,
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch members");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to fetch members",
                )),
            ));
        }
    };

    // Get roles using RLS
    let roles = match state.role_repo.list_by_org_rls(&mut **rls.conn(), id).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch roles");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to fetch roles",
                )),
            ));
        }
    };

    // Build export members
    let export_members: Vec<ExportMember> = members
        .into_iter()
        .map(|m| {
            let role_name = roles
                .iter()
                .find(|r| Some(r.id) == m.role_id)
                .map(|r| r.name.clone());

            ExportMember {
                user_id: m.user_id,
                email: Some(m.user_email),
                name: Some(m.user_name),
                role_name,
                role_type: m.role_type,
                status: m.status,
                joined_at: m.joined_at.map(|dt| dt.to_rfc3339()).unwrap_or_default(),
            }
        })
        .collect();

    // Build export roles
    let export_roles: Vec<ExportRole> = roles
        .into_iter()
        .map(|r| {
            let permissions = r.permission_list();
            ExportRole {
                id: r.id,
                name: r.name,
                description: r.description,
                permissions,
                is_system: r.is_system,
            }
        })
        .collect();

    let total_members = export_members.len();
    let total_roles = export_roles.len();

    let org_response = OrganizationResponse::from(org);

    // Handle CSV export
    if query.format.to_lowercase() == "csv" {
        let mut csv_data = String::new();
        csv_data.push_str("user_id,email,name,role_name,role_type,status,joined_at\n");

        for m in &export_members {
            csv_data.push_str(&format!(
                "{},{},{},{},{},{},{}\n",
                m.user_id,
                escape_csv_field(m.email.as_deref().unwrap_or("")),
                escape_csv_field(m.name.as_deref().unwrap_or("")),
                escape_csv_field(m.role_name.as_deref().unwrap_or("")),
                escape_csv_field(&m.role_type),
                escape_csv_field(&m.status),
                m.joined_at
            ));
        }

        tracing::info!(org_id = %id, format = "csv", members = total_members, "Organization data exported");

        return Ok((
            StatusCode::OK,
            [
                (axum::http::header::CONTENT_TYPE, "text/csv; charset=utf-8"),
                (
                    axum::http::header::CONTENT_DISPOSITION,
                    "attachment; filename=\"organization-export.csv\"",
                ),
            ],
            csv_data,
        )
            .into_response());
    }

    // Default to JSON export
    let export = OrganizationExportResponse {
        exported_at: chrono::Utc::now().to_rfc3339(),
        organization: org_response,
        members: export_members,
        roles: export_roles,
        total_members,
        total_roles,
    };

    tracing::info!(org_id = %id, format = "json", "Organization data exported");

    Ok(Json(export).into_response())
}

// ==================== Organization Feature Preferences (Epic 110, Story 110.3) ====================

/// Organization feature preference response.
#[derive(Debug, Serialize, ToSchema)]
pub struct OrganizationFeatureResponse {
    /// Feature flag key
    pub key: String,
    /// Feature flag name
    pub name: String,
    /// Feature flag description
    pub description: Option<String>,
    /// Whether globally enabled
    pub global_enabled: bool,
    /// Whether enabled for this organization (override)
    pub org_enabled: Option<bool>,
    /// Resolved enabled state for this organization
    pub effective_enabled: bool,
    /// Whether the org can toggle this feature
    pub can_toggle: bool,
}

/// List organization features response.
#[derive(Debug, Serialize, ToSchema)]
pub struct ListOrganizationFeaturesResponse {
    /// List of features with their states
    pub features: Vec<OrganizationFeatureResponse>,
}

/// Request to update a single feature preference.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateFeaturePreferenceRequest {
    /// Whether to enable the feature
    pub is_enabled: bool,
}

/// Request to bulk update feature preferences.
#[derive(Debug, Deserialize, ToSchema)]
pub struct BulkUpdateFeaturesRequest {
    /// Map of feature keys to enabled states
    pub features: std::collections::HashMap<String, bool>,
}

/// Response after updating feature preferences.
#[derive(Debug, Serialize, ToSchema)]
pub struct UpdateFeaturesResponse {
    /// Number of features updated
    pub updated: usize,
    /// Success message
    pub message: String,
}

/// List all features for an organization with their toggle states.
#[utoipa::path(
    get,
    path = "/api/v1/organizations/{id}/features",
    tag = "Organizations",
    params(
        ("id" = Uuid, Path, description = "Organization ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of features", body = ListOrganizationFeaturesResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Organization not found", body = ErrorResponse)
    )
)]
pub async fn list_organization_features(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<ListOrganizationFeaturesResponse>, (StatusCode, Json<ErrorResponse>)> {
    let token = extract_bearer_token(&headers)?;
    let claims = validate_access_token(&state, &token)?;

    let user_id: Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    // Check membership using RLS
    let _membership = match state
        .org_member_repo
        .find_by_org_and_user_rls(&mut **rls.conn(), id, user_id)
        .await
    {
        Ok(Some(m)) => m,
        Ok(None) => {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new(
                    "NOT_MEMBER",
                    "You are not a member of this organization",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to check membership");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to check membership",
                )),
            ));
        }
    };

    // Get all feature flags
    let flags = match state.feature_flag_repo.list_all().await {
        Ok(flags) => flags,
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch feature flags");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to fetch feature flags",
                )),
            ));
        }
    };

    // Build response with org-level preferences
    let mut features = Vec::new();
    for flag in flags {
        // Get org-level override if exists
        let org_enabled = match state
            .feature_flag_repo
            .is_enabled_for_context(&flag.key, None, Some(id), None)
            .await
        {
            Ok(Some(enabled)) if enabled != flag.is_enabled => Some(enabled),
            Ok(_) => None,
            Err(_) => None,
        };

        // Resolve effective state
        let effective_enabled = org_enabled.unwrap_or(flag.is_enabled);

        features.push(OrganizationFeatureResponse {
            key: flag.key,
            name: flag.name,
            description: flag.description,
            global_enabled: flag.is_enabled,
            org_enabled,
            effective_enabled,
            can_toggle: true, // All features can be toggled by org admins
        });
    }

    Ok(Json(ListOrganizationFeaturesResponse { features }))
}

/// Bulk update feature preferences for an organization.
#[utoipa::path(
    put,
    path = "/api/v1/organizations/{id}/features",
    tag = "Organizations",
    params(
        ("id" = Uuid, Path, description = "Organization ID")
    ),
    request_body = BulkUpdateFeaturesRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Features updated", body = UpdateFeaturesResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Organization not found", body = ErrorResponse)
    )
)]
pub async fn bulk_update_organization_features(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<BulkUpdateFeaturesRequest>,
) -> Result<Json<UpdateFeaturesResponse>, (StatusCode, Json<ErrorResponse>)> {
    let token = extract_bearer_token(&headers)?;
    let claims = validate_access_token(&state, &token)?;

    let user_id: Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    // Check membership and permission using RLS
    let membership = match state
        .org_member_repo
        .find_by_org_and_user_rls(&mut **rls.conn(), id, user_id)
        .await
    {
        Ok(Some(m)) => m,
        Ok(None) => {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new(
                    "NOT_MEMBER",
                    "You are not a member of this organization",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to check membership");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to check membership",
                )),
            ));
        }
    };

    // Get role and check permissions using RLS
    let role = match membership.role_id {
        Some(role_id) => match state
            .role_repo
            .find_by_id_rls(&mut **rls.conn(), role_id)
            .await
        {
            Ok(Some(r)) => r,
            Ok(None) => {
                return Err((
                    StatusCode::FORBIDDEN,
                    Json(ErrorResponse::new("ROLE_NOT_FOUND", "User role not found")),
                ));
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to fetch role");
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new("DATABASE_ERROR", "Failed to fetch role")),
                ));
            }
        },
        None => {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new("NO_ROLE", "User has no role assigned")),
            ));
        }
    };

    // Check for organization:manage_features or organization:update permission
    if !role.has_permission("organization:manage_features")
        && !role.has_permission("organization:update")
    {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "PERMISSION_DENIED",
                "You do not have permission to manage feature preferences",
            )),
        ));
    }

    let mut updated = 0;
    for (key, enabled) in req.features {
        // Get flag by key
        let flag = match state.feature_flag_repo.get_by_key(&key).await {
            Ok(Some(f)) => f,
            Ok(None) => {
                tracing::warn!(key = %key, "Feature flag not found, skipping");
                continue;
            }
            Err(e) => {
                tracing::error!(error = %e, key = %key, "Failed to fetch feature flag");
                continue;
            }
        };

        // Create or update override for this organization
        match state
            .feature_flag_repo
            .create_override(
                flag.id,
                db::models::platform_admin::FeatureFlagScope::Organization,
                id,
                enabled,
            )
            .await
        {
            Ok(_) => {
                updated += 1;
                tracing::info!(
                    org_id = %id,
                    flag_key = %key,
                    enabled = enabled,
                    "Feature preference updated"
                );
            }
            Err(e) => {
                tracing::error!(error = %e, key = %key, "Failed to update feature preference");
            }
        }
    }

    Ok(Json(UpdateFeaturesResponse {
        updated,
        message: format!("Updated {} feature preferences", updated),
    }))
}

/// Toggle a single feature for an organization.
#[utoipa::path(
    put,
    path = "/api/v1/organizations/{id}/features/{key}",
    tag = "Organizations",
    params(
        ("id" = Uuid, Path, description = "Organization ID"),
        ("key" = String, Path, description = "Feature flag key")
    ),
    request_body = UpdateFeaturePreferenceRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Feature updated", body = OrganizationFeatureResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Organization or feature not found", body = ErrorResponse)
    )
)]
pub async fn toggle_organization_feature(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    mut rls: RlsConnection,
    Path((id, key)): Path<(Uuid, String)>,
    Json(req): Json<UpdateFeaturePreferenceRequest>,
) -> Result<Json<OrganizationFeatureResponse>, (StatusCode, Json<ErrorResponse>)> {
    let token = extract_bearer_token(&headers)?;
    let claims = validate_access_token(&state, &token)?;

    let user_id: Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    // Check membership and permission using RLS
    let membership = match state
        .org_member_repo
        .find_by_org_and_user_rls(&mut **rls.conn(), id, user_id)
        .await
    {
        Ok(Some(m)) => m,
        Ok(None) => {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new(
                    "NOT_MEMBER",
                    "You are not a member of this organization",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to check membership");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to check membership",
                )),
            ));
        }
    };

    // Get role and check permissions using RLS
    let role = match membership.role_id {
        Some(role_id) => match state
            .role_repo
            .find_by_id_rls(&mut **rls.conn(), role_id)
            .await
        {
            Ok(Some(r)) => r,
            Ok(None) => {
                return Err((
                    StatusCode::FORBIDDEN,
                    Json(ErrorResponse::new("ROLE_NOT_FOUND", "User role not found")),
                ));
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to fetch role");
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new("DATABASE_ERROR", "Failed to fetch role")),
                ));
            }
        },
        None => {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new("NO_ROLE", "User has no role assigned")),
            ));
        }
    };

    // Check for organization:manage_features or organization:update permission
    if !role.has_permission("organization:manage_features")
        && !role.has_permission("organization:update")
    {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "PERMISSION_DENIED",
                "You do not have permission to manage feature preferences",
            )),
        ));
    }

    // Get flag by key
    let flag = match state.feature_flag_repo.get_by_key(&key).await {
        Ok(Some(f)) => f,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "FEATURE_NOT_FOUND",
                    "Feature flag not found",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch feature flag");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to fetch feature flag",
                )),
            ));
        }
    };

    // Create or update override for this organization
    match state
        .feature_flag_repo
        .create_override(
            flag.id,
            db::models::platform_admin::FeatureFlagScope::Organization,
            id,
            req.is_enabled,
        )
        .await
    {
        Ok(_) => {
            tracing::info!(
                org_id = %id,
                flag_key = %key,
                enabled = req.is_enabled,
                user_id = %user_id,
                "Feature preference updated"
            );
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to update feature preference");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to update feature preference",
                )),
            ));
        }
    }

    // Return updated feature state
    let feature_details = match state.feature_flag_repo.get_by_key(&key).await {
        Ok(Some(f)) => f,
        Ok(None) | Err(_) => flag.clone(),
    };

    Ok(Json(OrganizationFeatureResponse {
        key: feature_details.key,
        name: feature_details.name,
        description: feature_details.description,
        global_enabled: feature_details.is_enabled,
        org_enabled: Some(req.is_enabled),
        effective_enabled: req.is_enabled,
        can_toggle: true,
    }))
}
