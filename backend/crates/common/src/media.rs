//! Shared media-type helpers.

/// Allowed MIME types for end-user file uploads.
///
/// Single source of truth (GH #2320) shared by the document model
/// (`db::models::ALLOWED_MIME_TYPES`) and the storage layer
/// (`integrations::ALLOWED_MIME_TYPES`), both of which re-export this const.
/// Previously the two crates kept independent, identical lists; drift between
/// them surfaced as a confusing 503 `STORAGE_ERROR` (handler accepted, storage
/// rejected) instead of a 400.
pub const ALLOWED_UPLOAD_MIME_TYPES: &[&str] = &[
    // Documents
    "application/pdf",
    "application/msword",
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    "application/vnd.ms-excel",
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    "text/plain",
    "text/csv",
    // Images
    "image/png",
    "image/jpeg",
    "image/gif",
    "image/webp",
];

/// Returns `true` for MIME types the platform renders **inline** (preview)
/// rather than forcing a download.
///
/// This is the single source of truth shared by the document model
/// (`db::models::Document::supports_preview`) and the storage presigner
/// (`integrations::storage::supports_inline_preview`), so the preview gate and
/// the `Content-Disposition: inline` decision can never diverge.
///
/// `text/plain` is intentionally included: browsers render it inline as plain
/// text (no script execution), and the storage layer already serves it inline.
/// Aligning the document preview gate to that — rather than 400-ing a file the
/// presigner would happily stream inline — keeps the two consistent.
pub fn supports_inline_preview(content_type: &str) -> bool {
    matches!(
        content_type,
        "application/pdf" | "image/png" | "image/jpeg" | "image/gif" | "image/webp" | "text/plain"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn previewable_types() {
        for ct in [
            "application/pdf",
            "image/png",
            "image/jpeg",
            "image/gif",
            "image/webp",
            "text/plain",
        ] {
            assert!(supports_inline_preview(ct), "{ct} should preview inline");
        }
    }

    #[test]
    fn non_previewable_types() {
        for ct in [
            "application/zip",
            "application/octet-stream",
            "application/msword",
            "text/csv",
        ] {
            assert!(!supports_inline_preview(ct), "{ct} should not preview");
        }
    }
}
