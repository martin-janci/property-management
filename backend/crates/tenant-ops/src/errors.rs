//! Error type for tenant-ops.

use thiserror::Error;

/// Crate-wide result alias.
pub type TenantOpsResult<T> = Result<T, TenantOpsError>;

#[derive(Debug, Error)]
pub enum TenantOpsError {
    #[error("manifest IO: {0}")]
    ManifestIo(#[from] std::io::Error),

    #[error("manifest parse: {0}")]
    ManifestParse(String),

    #[error(
        "manifest is stale: table `{0}` was added to the schema but is not in the manifest. \
         Regenerate via `bash backend/scripts/check-rls-coverage.sh \
         --emit-manifest backend/manifests/tenant-data-manifest.json`."
    )]
    ManifestStale(String),

    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),

    #[error("organization not found: {0}")]
    NotFound(uuid::Uuid),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("export tarball error: {0}")]
    Tarball(String),

    #[error("restore validation: {0}")]
    RestoreInvalid(String),
}
