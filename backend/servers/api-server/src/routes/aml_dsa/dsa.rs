//! Story 67.3: DSA Transparency Reports endpoints.

use crate::state::AppState;
use api_core::extractors::{AuthUser, RequestPrincipal};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Duration, Utc};
use db::models::compliance::DsaReportStatus;
use db::models::AuditAction;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::shared::*;

/// DSA report summary statistics.
#[derive(Debug, Serialize)]
pub struct DsaReportSummary {
    pub total_moderation_actions: i64,
    pub content_removed: i64,
    pub content_restricted: i64,
    pub warnings_issued: i64,
    pub user_reports_received: i64,
    pub user_reports_resolved: i64,
    pub avg_resolution_time_hours: Option<f64>,
    pub automated_decisions: i64,
    pub automated_decisions_overturned: i64,
    pub appeals_received: i64,
    pub appeals_upheld: i64,
    pub appeals_rejected: i64,
}

/// Content type count.
#[derive(Debug, Serialize)]
pub struct ContentTypeCount {
    pub content_type: String,
    pub count: i64,
}

/// Violation type count.
#[derive(Debug, Serialize)]
pub struct ViolationTypeCountResponse {
    pub violation_type: String,
    pub count: i64,
}

/// DSA transparency report response.
#[derive(Debug, Serialize)]
pub struct DsaTransparencyReportResponse {
    pub id: Uuid,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub status: DsaReportStatus,
    pub summary: DsaReportSummary,
    pub content_type_breakdown: Vec<ContentTypeCount>,
    pub violation_type_breakdown: Vec<ViolationTypeCountResponse>,
    pub download_url: Option<String>,
    pub generated_at: Option<DateTime<Utc>>,
    pub published_at: Option<DateTime<Utc>>,
}

/// Request to generate DSA report.
#[derive(Debug, Deserialize)]
pub struct GenerateDsaReportRequest {
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

/// Short-lived presigned download URL for a DSA report file.
#[derive(Debug, Serialize)]
pub struct DsaReportDownloadResponse {
    pub url: String,
    pub expires_at: DateTime<Utc>,
}

// NOTE: `dsa_report_download_ref` is defined once near the top of this module
// (added by PAP-44). PAP-47 landed an identical second definition here, which
// collided (E0428) once both were on `dev`; the duplicate has been removed.
// The shared helper builds the opaque `/api/v1/aml-dsa/dsa/reports/{id}/download`
// reference and never discloses the internal `report_file_path`.
// ----------------------------------------------------------------------------
// DSA transparency reports (Epic 67 / Story 67.3) — platform-VLOP model.
//
// Per the PAP-40 -> PAP-47 decision, DSA transparency reporting is a
// PLATFORM-LEVEL regulatory artifact (32bit is the regulated VLOP/intermediary
// under the EU DSA), not a per-tenant one. These reports are platform-wide by
// design: every handler below is gated to platform roles via
// `require_compliance_role` (SuperAdmin/PlatformAdmin) and accepts NO
// client-supplied org/tenant filter, so scope cannot be narrowed or injected
// from request input.
// ----------------------------------------------------------------------------

/// List DSA transparency reports.
pub(super) async fn list_dsa_reports(
    State(state): State<AppState>,
    principal: RequestPrincipal,
) -> Result<Json<Vec<DsaTransparencyReportResponse>>, (StatusCode, String)> {
    require_platform_compliance_role(&principal)?;

    let reports = state
        .compliance_repo
        .list_dsa_reports(None, 50, 0)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list DSA reports: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to list reports".to_string(),
            )
        })?;

    let responses: Vec<DsaTransparencyReportResponse> = reports
        .into_iter()
        .map(|r| DsaTransparencyReportResponse {
            id: r.id,
            period_start: r.period_start,
            period_end: r.period_end,
            status: r.status,
            summary: DsaReportSummary {
                total_moderation_actions: r.total_moderation_actions,
                content_removed: r.content_removed_count,
                content_restricted: r.content_restricted_count,
                warnings_issued: r.warnings_issued_count,
                user_reports_received: r.user_reports_received,
                user_reports_resolved: r.user_reports_resolved,
                avg_resolution_time_hours: r.avg_resolution_time_hours,
                automated_decisions: r.automated_decisions_count,
                automated_decisions_overturned: r.automated_decisions_overturned,
                appeals_received: r.appeals_received,
                appeals_upheld: r.appeals_upheld,
                appeals_rejected: r.appeals_rejected,
            },
            content_type_breakdown: vec![],
            violation_type_breakdown: vec![],
            download_url: dsa_report_download_ref(r.id, &r.report_file_path),
            generated_at: r.generated_at,
            published_at: r.published_at,
        })
        .collect();

    Ok(Json(responses))
}

/// Generate a new DSA transparency report.
pub(super) async fn generate_dsa_report(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    user: AuthUser,
    Json(req): Json<GenerateDsaReportRequest>,
) -> Result<Json<DsaTransparencyReportResponse>, (StatusCode, String)> {
    require_platform_compliance_role(&principal)?;

    // Bound the reporting period: end after start, not in the future, sane span.
    validate_report_period(req.period_start, req.period_end, Utc::now())
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let report = state
        .compliance_repo
        .create_dsa_report(req.period_start, req.period_end, user.user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to generate DSA report: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to generate report".to_string(),
            )
        })?;

    Ok(Json(DsaTransparencyReportResponse {
        id: report.id,
        period_start: report.period_start,
        period_end: report.period_end,
        status: report.status,
        summary: DsaReportSummary {
            total_moderation_actions: report.total_moderation_actions,
            content_removed: report.content_removed_count,
            content_restricted: report.content_restricted_count,
            warnings_issued: report.warnings_issued_count,
            user_reports_received: report.user_reports_received,
            user_reports_resolved: report.user_reports_resolved,
            avg_resolution_time_hours: report.avg_resolution_time_hours,
            automated_decisions: report.automated_decisions_count,
            automated_decisions_overturned: report.automated_decisions_overturned,
            appeals_received: report.appeals_received,
            appeals_upheld: report.appeals_upheld,
            appeals_rejected: report.appeals_rejected,
        },
        content_type_breakdown: vec![],
        violation_type_breakdown: vec![],
        download_url: dsa_report_download_ref(report.id, &report.report_file_path),
        generated_at: report.generated_at,
        published_at: report.published_at,
    }))
}

/// Get a specific DSA report.
pub(super) async fn get_dsa_report(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<DsaTransparencyReportResponse>, (StatusCode, String)> {
    require_platform_compliance_role(&principal)?;

    let report = state
        .compliance_repo
        .get_dsa_report(id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get DSA report: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get report".to_string(),
            )
        })?
        .ok_or((StatusCode::NOT_FOUND, format!("Report {} not found", id)))?;

    Ok(Json(DsaTransparencyReportResponse {
        id: report.id,
        period_start: report.period_start,
        period_end: report.period_end,
        status: report.status,
        summary: DsaReportSummary {
            total_moderation_actions: report.total_moderation_actions,
            content_removed: report.content_removed_count,
            content_restricted: report.content_restricted_count,
            warnings_issued: report.warnings_issued_count,
            user_reports_received: report.user_reports_received,
            user_reports_resolved: report.user_reports_resolved,
            avg_resolution_time_hours: report.avg_resolution_time_hours,
            automated_decisions: report.automated_decisions_count,
            automated_decisions_overturned: report.automated_decisions_overturned,
            appeals_received: report.appeals_received,
            appeals_upheld: report.appeals_upheld,
            appeals_rejected: report.appeals_rejected,
        },
        content_type_breakdown: vec![],
        violation_type_breakdown: vec![],
        download_url: dsa_report_download_ref(report.id, &report.report_file_path),
        generated_at: report.generated_at,
        published_at: report.published_at,
    }))
}

/// Publish a DSA report.
pub(super) async fn publish_dsa_report(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<DsaTransparencyReportResponse>, (StatusCode, String)> {
    require_platform_compliance_role(&principal)?;

    let report = state
        .compliance_repo
        .publish_dsa_report(id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to publish DSA report: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to publish report".to_string(),
            )
        })?;

    write_compliance_audit(
        &state,
        &user,
        AuditAction::ResourceUpdated,
        "dsa_transparency_report",
        report.id,
        serde_json::json!({
            "operation": "publish_dsa_report",
            "resulting_status": report.status,
            "published_at": report.published_at,
        }),
    )
    .await;

    Ok(Json(DsaTransparencyReportResponse {
        id: report.id,
        period_start: report.period_start,
        period_end: report.period_end,
        status: report.status,
        summary: DsaReportSummary {
            total_moderation_actions: report.total_moderation_actions,
            content_removed: report.content_removed_count,
            content_restricted: report.content_restricted_count,
            warnings_issued: report.warnings_issued_count,
            user_reports_received: report.user_reports_received,
            user_reports_resolved: report.user_reports_resolved,
            avg_resolution_time_hours: report.avg_resolution_time_hours,
            automated_decisions: report.automated_decisions_count,
            automated_decisions_overturned: report.automated_decisions_overturned,
            appeals_received: report.appeals_received,
            appeals_upheld: report.appeals_upheld,
            appeals_rejected: report.appeals_rejected,
        },
        content_type_breakdown: vec![],
        violation_type_breakdown: vec![],
        download_url: dsa_report_download_ref(report.id, &report.report_file_path),
        generated_at: report.generated_at,
        published_at: report.published_at,
    }))
}

/// Download DSA report as PDF.
///
/// PAP-47 / PAP-35: returns a short-lived presigned URL generated from the
/// stored storage key — the raw `report_file_path` is NEVER returned to the
/// client (that was the file-path disclosure flagged in the PAP-35 review).
pub(super) async fn download_dsa_report(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<DsaReportDownloadResponse>, (StatusCode, String)> {
    require_platform_compliance_role(&principal)?;

    let report = state
        .compliance_repo
        .get_dsa_report(id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get DSA report: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get report".to_string(),
            )
        })?
        .ok_or((StatusCode::NOT_FOUND, format!("Report {} not found", id)))?;

    let file_key = report.report_file_path.ok_or((
        StatusCode::NOT_FOUND,
        "Report file not yet generated".to_string(),
    ))?;

    let storage = state.storage_service.as_ref().ok_or_else(|| {
        tracing::error!("Storage service not configured — DSA report downloads unavailable");
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "Report storage is not configured.".to_string(),
        )
    })?;

    let filename = format!("dsa-transparency-report-{id}.pdf");
    let presigned = storage
        .generate_download_url(
            &file_key,
            &filename,
            "application/pdf",
            Some(storage.download_ttl_secs()),
        )
        .await
        .map_err(|e| {
            // Log the internal key for diagnostics; never surface it to the client.
            tracing::error!(error = %e, report_id = %id, "Failed to presign DSA report download");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                "Unable to generate download URL. Please try again later.".to_string(),
            )
        })?;

    Ok(Json(DsaReportDownloadResponse {
        url: presigned.url,
        expires_at: presigned.expires_at,
    }))
}

/// DSA metrics for current period.
#[derive(Debug, Serialize)]
pub struct DsaMetricsResponse {
    pub current_period_start: DateTime<Utc>,
    pub current_period_end: DateTime<Utc>,
    pub moderation_actions_this_period: i64,
    pub pending_cases: i64,
    pub avg_resolution_time_hours: f64,
    pub sla_compliance_rate: f64,
}

/// Get current DSA metrics.
pub(super) async fn get_dsa_metrics(
    State(state): State<AppState>,
    principal: RequestPrincipal,
) -> Result<Json<DsaMetricsResponse>, (StatusCode, String)> {
    require_platform_compliance_role(&principal)?;

    // DSA transparency metrics are platform-wide (all organizations), per the
    // PAP-40 tenancy decision and the platform-only gate above (PAP-46). There
    // is no org context here — a platform operator has `tenant_id = None`.
    let stats = state
        .compliance_repo
        .get_platform_moderation_queue_stats()
        .await
        .map_err(|e| {
            tracing::error!("Failed to get DSA metrics: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get metrics".to_string(),
            )
        })?;

    let now = Utc::now();
    let period_start = now - Duration::days(30);

    // SLA compliance: percentage of cases resolved within 24 hours
    let total_cases = stats.pending_count + stats.under_review_count;
    let sla_compliance_rate = if total_cases > 0 {
        ((total_cases - stats.overdue_count) as f64 / total_cases as f64) * 100.0
    } else {
        100.0
    };

    Ok(Json(DsaMetricsResponse {
        current_period_start: period_start,
        current_period_end: now,
        moderation_actions_this_period: stats.pending_count + stats.under_review_count,
        pending_cases: stats.pending_count,
        avg_resolution_time_hours: stats.avg_resolution_time_hours,
        sla_compliance_rate,
    }))
}
