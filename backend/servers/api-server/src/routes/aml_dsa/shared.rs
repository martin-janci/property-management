//! Shared helpers for the AML/DSA compliance routes.
//!
//! Auth-role guards, boundary input-validation helpers, length/size limits,
//! and the compliance audit-log writer. Pure functions live here so they can
//! be unit-tested without a DB or HTTP layer (see the `tests` module).

use crate::state::AppState;
use api_core::extractors::{AuthUser, RequestPrincipal};
use axum::http::StatusCode;
use chrono::{DateTime, Duration, Utc};
use common::TenantRole;
use db::models::{AuditAction, CreateAuditLog};
use uuid::Uuid;

use super::edd::UploadEddDocumentRequest;

/// Check if user has compliance officer role or higher.
pub(super) fn require_compliance_role(user: &AuthUser) -> Result<(), (StatusCode, String)> {
    match user.role {
        Some(TenantRole::SuperAdmin) | Some(TenantRole::PlatformAdmin) => Ok(()),
        _ => Err((
            StatusCode::FORBIDDEN,
            "This endpoint requires compliance officer privileges".to_string(),
        )),
    }
}

/// Restrict access to platform-operator compliance staff only.
///
/// SECURITY (PAP-46): DSA transparency reports are platform-wide — a period
/// rollup of whole-service moderation metrics with no `organization_id`. Only
/// the platform operator may read/generate/publish/download them. We gate on
/// `PrincipalKind::Platform` (resolved from the trusted `users.principal_kind`
/// column by the `RequestPrincipal` extractor), NOT on the JWT `TenantRole`: a
/// tenant-scoped admin must never reach these handlers even if their per-org
/// membership role happens to be SuperAdmin/PlatformAdmin. This mirrors the
/// `admin-core` platform-principal model and is a distinct guard from
/// `require_compliance_role`, which correctly gates the per-tenant AML/EDD
/// handlers and must not be weakened.
pub(super) fn require_platform_compliance_role(
    principal: &RequestPrincipal,
) -> Result<(), (StatusCode, String)> {
    if principal.is_platform() {
        Ok(())
    } else {
        Err((
            StatusCode::FORBIDDEN,
            "This endpoint requires platform-operator compliance privileges".to_string(),
        ))
    }
}

/// Check if user has moderator role or higher.
pub(super) fn require_moderator_role(user: &AuthUser) -> Result<(), (StatusCode, String)> {
    match user.role {
        Some(TenantRole::SuperAdmin)
        | Some(TenantRole::PlatformAdmin)
        | Some(TenantRole::Manager) => Ok(()),
        _ => Err((
            StatusCode::FORBIDDEN,
            "This endpoint requires moderator privileges".to_string(),
        )),
    }
}

/// Emit a compliance audit record for an AML/EDD/DSA decision or state change.
///
/// Best-effort: a logging failure must never turn a successful compliance
/// operation into a 500. We log the failure and continue, mirroring the
/// convention in `routes/listings.rs::global_publish`. The substantive
/// who/what/when (actor, tenant, resource, decision payload) lands in the
/// `audit_logs` table so AML record-keeping and EU DSA Art. 17
/// statement-of-reasons obligations (FR115) have a durable trail.
pub(super) async fn write_compliance_audit(
    state: &AppState,
    user: &AuthUser,
    action: AuditAction,
    resource_type: &str,
    resource_id: Uuid,
    details: serde_json::Value,
) {
    if let Err(e) = state
        .audit_log_repo
        .create(CreateAuditLog {
            user_id: Some(user.user_id),
            action,
            resource_type: Some(resource_type.to_string()),
            resource_id: Some(resource_id),
            org_id: user.tenant_id,
            details: Some(details),
            old_values: None,
            new_values: None,
            ip_address: None,
            user_agent: None,
        })
        .await
    {
        tracing::warn!(
            error = %e,
            resource_type = resource_type,
            resource_id = %resource_id,
            "Failed to write compliance audit log"
        );
    }
}

// ============================================================================
// INPUT-VALIDATION HELPERS (PAP-44)
// ============================================================================
//
// Boundary validation for untrusted request input. These are deliberately
// small, pure functions so they can be unit-tested without a DB or HTTP layer.

/// Default page size when the client does not specify `limit`.
pub(super) const DEFAULT_PAGE_LIMIT: i64 = 50;
/// Hard upper bound on `limit` to prevent abusive/accidental large scans.
pub(super) const MAX_PAGE_LIMIT: i64 = 200;

/// Maximum size of an EDD document (50 MiB).
pub(super) const MAX_EDD_DOCUMENT_BYTES: i64 = 50 * 1024 * 1024;

/// Allow-listed MIME types for EDD document uploads.
pub(super) const ALLOWED_EDD_MIME_TYPES: &[&str] = &[
    "application/pdf",
    "image/png",
    "image/jpeg",
    "image/tiff",
    "image/webp",
    "application/msword",
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
];

/// Maximum span of a DSA reporting period (~2 years).
pub(super) const MAX_DSA_REPORT_RANGE_DAYS: i64 = 732;

/// Length caps for free-text fields.
pub(super) const MAX_NOTE_LEN: usize = 10_000;
pub(super) const MAX_RATIONALE_LEN: usize = 10_000;
pub(super) const MAX_APPEAL_REASON_LEN: usize = 5_000;
pub(super) const MAX_FILENAME_LEN: usize = 255;
pub(super) const MAX_FILE_PATH_LEN: usize = 1024;

/// Clamp a client-supplied `limit` into `[1, MAX_PAGE_LIMIT]`, defaulting when absent.
pub(super) fn clamp_limit(limit: Option<i64>) -> i64 {
    limit.unwrap_or(DEFAULT_PAGE_LIMIT).clamp(1, MAX_PAGE_LIMIT)
}

/// Normalize a client-supplied `offset` to a non-negative value.
pub(super) fn sanitize_offset(offset: Option<i64>) -> i64 {
    offset.unwrap_or(0).max(0)
}

/// Reject paths that are absolute, contain a parent-directory (`..`) component,
/// use Windows-style separators/drives, contain control characters, or are
/// empty — i.e. anything that could escape an intended storage root.
pub(super) fn validate_storage_path(path: &str) -> Result<(), &'static str> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("must not be empty");
    }
    if trimmed.len() > MAX_FILE_PATH_LEN {
        return Err("is too long");
    }
    if trimmed.starts_with('/') || trimmed.starts_with('\\') {
        return Err("must be a relative path");
    }
    if trimmed.contains(':') || trimmed.contains('\\') {
        return Err("must not contain drive letters or backslashes");
    }
    if trimmed.chars().any(|c| c.is_control()) {
        return Err("must not contain control characters");
    }
    if trimmed.split('/').any(|component| component == "..") {
        return Err("must not contain '..' components");
    }
    Ok(())
}

/// Validate an uploaded filename: non-empty, length-capped, no path separators.
pub(super) fn validate_filename(name: &str) -> Result<(), &'static str> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("must not be empty");
    }
    if trimmed.len() > MAX_FILENAME_LEN {
        return Err("is too long");
    }
    if trimmed.contains('/') || trimmed.contains('\\') || trimmed.contains('\0') {
        return Err("must not contain path separators");
    }
    if trimmed == "." || trimmed == ".." {
        return Err("is invalid");
    }
    Ok(())
}

/// Validate EDD document upload metadata: path traversal, MIME allow-list, size bounds.
pub(super) fn validate_edd_document(req: &UploadEddDocumentRequest) -> Result<(), String> {
    validate_storage_path(&req.file_path).map_err(|e| format!("file_path {e}"))?;
    validate_filename(&req.original_filename).map_err(|e| format!("original_filename {e}"))?;
    if req.file_size_bytes <= 0 {
        return Err("file_size_bytes must be greater than 0".to_string());
    }
    if req.file_size_bytes > MAX_EDD_DOCUMENT_BYTES {
        return Err(format!(
            "file_size_bytes exceeds the maximum of {MAX_EDD_DOCUMENT_BYTES} bytes"
        ));
    }
    let mime = req.mime_type.trim().to_ascii_lowercase();
    if !ALLOWED_EDD_MIME_TYPES.contains(&mime.as_str()) {
        return Err(format!("mime_type '{}' is not allowed", req.mime_type));
    }
    Ok(())
}

/// Validate a DSA reporting period: end after start, not in the future, within max span.
pub(super) fn validate_report_period(
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<(), &'static str> {
    if end <= start {
        return Err("period_end must be after period_start");
    }
    if end > now {
        return Err("period_end must not be in the future");
    }
    if (end - start) > Duration::days(MAX_DSA_REPORT_RANGE_DAYS) {
        return Err("reporting period is too large");
    }
    Ok(())
}

/// Validate a required free-text field: non-empty (after trim) and within `max` chars.
pub(super) fn validate_text_field(value: &str, max: usize, field: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{field} must not be empty"));
    }
    if value.chars().count() > max {
        return Err(format!(
            "{field} exceeds the maximum length of {max} characters"
        ));
    }
    Ok(())
}

/// Accepted appeal decision tokens (DSA Art. 17 appeal outcome). These map
/// directly to the exact string the repository layer matches on, so they are
/// case-sensitive: `"upheld"` approves the appeal, `"rejected"` rejects it.
pub(super) const APPEAL_DECISIONS: [&str; 2] = ["upheld", "rejected"];

/// Validate an appeal decision against the accepted enum.
///
/// The repository maps `"upheld"` to an approval and treats *everything else*
/// as a rejection, so an unrecognised value (a typo or wrong casing such as
/// `"Upheld"`) would otherwise silently reject the appeal. Reject unknown
/// values here so the caller gets a `400` instead of a wrong, irreversible
/// outcome.
pub(super) fn validate_appeal_decision(decision: &str) -> Result<(), String> {
    if APPEAL_DECISIONS.contains(&decision) {
        Ok(())
    } else {
        Err(format!(
            "decision must be one of: {}",
            APPEAL_DECISIONS.join(", ")
        ))
    }
}

/// Build a scoped, opaque download reference for a DSA report that never
/// discloses the internal filesystem path. Returns `None` when no file exists.
pub(super) fn dsa_report_download_ref(
    report_id: Uuid,
    file_path: &Option<String>,
) -> Option<String> {
    file_path
        .as_ref()
        .map(|_| format!("/api/v1/aml-dsa/dsa/reports/{report_id}/download"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use db::models::PrincipalKind;

    fn sample_upload() -> UploadEddDocumentRequest {
        UploadEddDocumentRequest {
            document_type: "passport".to_string(),
            original_filename: "passport.pdf".to_string(),
            file_path: "edd/2026/passport.pdf".to_string(),
            file_size_bytes: 1024,
            mime_type: "application/pdf".to_string(),
            expiry_date: None,
        }
    }

    // ----- pagination -----

    #[test]
    fn clamp_limit_defaults_when_absent() {
        assert_eq!(clamp_limit(None), DEFAULT_PAGE_LIMIT);
    }

    #[test]
    fn clamp_limit_caps_at_max() {
        assert_eq!(clamp_limit(Some(100_000)), MAX_PAGE_LIMIT);
        assert_eq!(clamp_limit(Some(MAX_PAGE_LIMIT + 1)), MAX_PAGE_LIMIT);
    }

    #[test]
    fn clamp_limit_floors_non_positive() {
        assert_eq!(clamp_limit(Some(0)), 1);
        assert_eq!(clamp_limit(Some(-50)), 1);
    }

    #[test]
    fn clamp_limit_passes_through_in_range() {
        assert_eq!(clamp_limit(Some(75)), 75);
    }

    #[test]
    fn sanitize_offset_rejects_negative() {
        assert_eq!(sanitize_offset(Some(-10)), 0);
        assert_eq!(sanitize_offset(None), 0);
        assert_eq!(sanitize_offset(Some(40)), 40);
    }

    // ----- path traversal / file upload -----

    #[test]
    fn storage_path_accepts_relative() {
        assert!(validate_storage_path("edd/2026/doc.pdf").is_ok());
    }

    #[test]
    fn storage_path_rejects_traversal() {
        assert!(validate_storage_path("../../etc/passwd").is_err());
        assert!(validate_storage_path("edd/../../secret").is_err());
        assert!(validate_storage_path("a/../b").is_err());
    }

    #[test]
    fn storage_path_rejects_absolute_and_windows() {
        assert!(validate_storage_path("/etc/passwd").is_err());
        assert!(validate_storage_path("\\\\server\\share").is_err());
        assert!(validate_storage_path("C:\\Windows\\system32").is_err());
    }

    #[test]
    fn storage_path_rejects_empty_and_control_chars() {
        assert!(validate_storage_path("   ").is_err());
        assert!(validate_storage_path("doc\0.pdf").is_err());
    }

    #[test]
    fn edd_document_accepts_valid() {
        assert!(validate_edd_document(&sample_upload()).is_ok());
    }

    #[test]
    fn edd_document_rejects_path_traversal() {
        let mut req = sample_upload();
        req.file_path = "../../../etc/shadow".to_string();
        assert!(validate_edd_document(&req).is_err());
    }

    #[test]
    fn edd_document_rejects_filename_with_separators() {
        let mut req = sample_upload();
        req.original_filename = "../evil.pdf".to_string();
        assert!(validate_edd_document(&req).is_err());
    }

    #[test]
    fn edd_document_rejects_disallowed_mime() {
        let mut req = sample_upload();
        req.mime_type = "application/x-msdownload".to_string();
        assert!(validate_edd_document(&req).is_err());
    }

    #[test]
    fn edd_document_rejects_bad_size() {
        let mut req = sample_upload();
        req.file_size_bytes = 0;
        assert!(validate_edd_document(&req).is_err());
        req.file_size_bytes = -1;
        assert!(validate_edd_document(&req).is_err());
        req.file_size_bytes = MAX_EDD_DOCUMENT_BYTES + 1;
        assert!(validate_edd_document(&req).is_err());
    }

    #[test]
    fn edd_document_mime_is_case_insensitive() {
        let mut req = sample_upload();
        req.mime_type = "APPLICATION/PDF".to_string();
        assert!(validate_edd_document(&req).is_ok());
    }

    // ----- DSA report period -----

    #[test]
    fn report_period_accepts_valid_past_range() {
        let now = Utc::now();
        let end = now - Duration::days(1);
        let start = end - Duration::days(30);
        assert!(validate_report_period(start, end, now).is_ok());
    }

    #[test]
    fn report_period_rejects_end_before_start() {
        let now = Utc::now();
        let start = now - Duration::days(1);
        let end = now - Duration::days(10);
        assert!(validate_report_period(start, end, now).is_err());
    }

    #[test]
    fn report_period_rejects_future_end() {
        let now = Utc::now();
        let start = now - Duration::days(5);
        let end = now + Duration::days(5);
        assert!(validate_report_period(start, end, now).is_err());
    }

    #[test]
    fn report_period_rejects_absurd_range() {
        let now = Utc::now();
        let end = now - Duration::days(1);
        let start = end - Duration::days(MAX_DSA_REPORT_RANGE_DAYS + 10);
        assert!(validate_report_period(start, end, now).is_err());
    }

    // ----- free-text caps -----

    #[test]
    fn text_field_rejects_empty() {
        assert!(validate_text_field("", MAX_NOTE_LEN, "content").is_err());
        assert!(validate_text_field("   \n\t ", MAX_NOTE_LEN, "content").is_err());
    }

    #[test]
    fn text_field_rejects_over_max() {
        let too_long = "a".repeat(MAX_APPEAL_REASON_LEN + 1);
        assert!(validate_text_field(&too_long, MAX_APPEAL_REASON_LEN, "reason").is_err());
    }

    #[test]
    fn text_field_accepts_within_bounds() {
        assert!(validate_text_field("a reasonable note", MAX_NOTE_LEN, "content").is_ok());
    }

    // ----- appeal decision enum -----

    #[test]
    fn appeal_decision_accepts_known_tokens() {
        assert!(validate_appeal_decision("upheld").is_ok());
        assert!(validate_appeal_decision("rejected").is_ok());
    }

    #[test]
    fn appeal_decision_rejects_typo_and_casing() {
        // Regression: the repo treats anything but exactly "upheld" as a
        // rejection, so a typo/wrong-casing value must be a 400, not a silent
        // (and irreversible) appeal rejection.
        assert!(validate_appeal_decision("Upheld").is_err());
        assert!(validate_appeal_decision("REJECTED").is_err());
        assert!(validate_appeal_decision("uphel").is_err());
        assert!(validate_appeal_decision("approve").is_err());
        assert!(validate_appeal_decision("").is_err());
    }

    // ----- scoped download reference -----

    #[test]
    fn download_ref_hides_fs_path() {
        let id = Uuid::nil();
        let raw = Some("/var/lib/app/reports/secret.pdf".to_string());
        let reference = dsa_report_download_ref(id, &raw).unwrap();
        assert!(!reference.contains("/var/lib"));
        assert!(reference.contains(&id.to_string()));
    }

    #[test]
    fn download_ref_none_when_no_file() {
        assert!(dsa_report_download_ref(Uuid::nil(), &None).is_none());
    }

    // ----- platform-authz boundary (PAP-46) -----

    fn principal(kind: PrincipalKind) -> RequestPrincipal {
        RequestPrincipal {
            user_id: Uuid::nil(),
            kind,
            effective_org: None,
        }
    }

    #[test]
    fn platform_principal_passes_dsa_guard() {
        assert!(require_platform_compliance_role(&principal(PrincipalKind::Platform)).is_ok());
    }

    #[test]
    fn tenant_scoped_principals_get_403_from_dsa_guard() {
        // A customer-org admin (Staff principal) — even one whose per-org
        // membership role is SuperAdmin/PlatformAdmin — must be rejected from
        // the platform-wide DSA-report handlers, as must portal (Public) users.
        for kind in [PrincipalKind::Staff, PrincipalKind::Public] {
            let err = require_platform_compliance_role(&principal(kind)).unwrap_err();
            assert_eq!(err.0, StatusCode::FORBIDDEN);
        }
    }
}
