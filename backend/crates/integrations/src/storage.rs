//! S3 Storage integration with presigned URL support (Story 84.1, Epic 103).
//!
//! Provides secure, time-limited access to files stored in S3-compatible storage
//! without exposing storage credentials to clients.
//!
//! Story 103.1: Real S3 upload implementation using aws-sdk-s3.

use aws_config::BehaviorVersion;
use aws_credential_types::Credentials;
use aws_sdk_s3::config::Region;
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client as S3Client;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration as StdDuration;
use thiserror::Error;

// ============================================================================
// Configuration
// ============================================================================

/// Default URL expiration for downloads (15 minutes).
pub const DEFAULT_DOWNLOAD_EXPIRATION_SECS: i64 = 15 * 60;

/// Default URL expiration for uploads (5 minutes).
pub const DEFAULT_UPLOAD_EXPIRATION_SECS: i64 = 5 * 60;

/// Maximum file size for presigned uploads (50MB).
pub const MAX_UPLOAD_SIZE_BYTES: i64 = 50 * 1024 * 1024;

/// Environment variable for S3 bucket name.
pub const S3_BUCKET_ENV: &str = "S3_BUCKET";

/// Environment variable for S3 region.
pub const S3_REGION_ENV: &str = "S3_REGION";

/// Environment variable for S3 endpoint (for S3-compatible services).
pub const S3_ENDPOINT_ENV: &str = "S3_ENDPOINT";

/// Environment variable for AWS access key ID.
pub const AWS_ACCESS_KEY_ID_ENV: &str = "AWS_ACCESS_KEY_ID";

/// Environment variable for AWS secret access key.
pub const AWS_SECRET_ACCESS_KEY_ENV: &str = "AWS_SECRET_ACCESS_KEY";

/// Environment variable for the presigned download URL TTL, in seconds.
///
/// Controls how long a `GET /documents/{id}/download` presigned URL stays
/// valid. Defaults to [`DEFAULT_DOWNLOAD_EXPIRATION_SECS`] (15 minutes) when
/// unset or unparsable. Keep this short — the URL grants unauthenticated S3
/// read access for its whole lifetime.
pub const S3_PRESIGNED_URL_TTL_ENV: &str = "S3_PRESIGNED_URL_TTL_SECS";

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during storage operations.
#[derive(Debug, Error)]
pub enum StorageError {
    /// Configuration error (missing environment variables).
    #[error("Storage configuration error: {0}")]
    Configuration(String),

    /// Failed to generate presigned URL.
    #[error("Failed to generate presigned URL: {0}")]
    PresignError(String),

    /// Invalid file key or path.
    #[error("Invalid file key: {0}")]
    InvalidKey(String),

    /// File not found in storage.
    #[error("File not found: {0}")]
    NotFound(String),

    /// Content type not allowed.
    #[error("Content type not allowed: {0}")]
    InvalidContentType(String),

    /// File size exceeds limit.
    #[error("File size {0} exceeds maximum allowed {1}")]
    FileTooLarge(i64, i64),

    /// Underlying HTTP client error.
    #[error("HTTP client error: {0}")]
    HttpError(String),

    /// S3 SDK error (Story 103.1).
    #[error("S3 error: {0}")]
    S3Error(String),

    /// Upload error (Story 103.1).
    #[error("Upload failed: {0}")]
    UploadError(String),

    /// Download error (Story 103.1).
    #[error("Download failed: {0}")]
    DownloadError(String),

    /// Delete error (Story 103.1).
    #[error("Delete failed: {0}")]
    DeleteError(String),
}

// ============================================================================
// Response Types
// ============================================================================

/// Presigned URL with expiration information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresignedUrl {
    /// The presigned URL for download or upload.
    pub url: String,

    /// When the URL expires.
    pub expires_at: DateTime<Utc>,
}

/// Response for a download URL request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadUrlResponse {
    /// The presigned download URL.
    pub url: String,

    /// When the URL expires.
    pub expires_at: DateTime<Utc>,

    /// Original filename for Content-Disposition.
    pub filename: String,

    /// MIME type of the file.
    pub content_type: String,

    /// File size in bytes.
    pub size_bytes: i64,
}

/// Response for an upload URL request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadUrlResponse {
    /// The presigned PUT URL for direct upload.
    pub url: String,

    /// The storage key where the file will be stored.
    pub key: String,

    /// When the URL expires.
    pub expires_at: DateTime<Utc>,

    /// Token for upload completion callback.
    pub callback_token: String,
}

// ============================================================================
// Storage Configuration
// ============================================================================

/// Storage service configuration.
#[derive(Debug, Clone)]
pub struct StorageConfig {
    /// S3 bucket name.
    pub bucket: String,

    /// AWS region.
    pub region: String,

    /// Optional custom endpoint for S3-compatible services.
    pub endpoint: Option<String>,

    /// AWS access key ID.
    pub access_key_id: String,

    /// AWS secret access key.
    pub secret_access_key: String,

    /// TTL (seconds) for presigned download URLs. Short-lived by design;
    /// defaults to [`DEFAULT_DOWNLOAD_EXPIRATION_SECS`] (15 minutes).
    pub download_ttl_secs: i64,
}

impl StorageConfig {
    /// Create configuration from environment variables.
    pub fn from_env() -> Result<Self, StorageError> {
        let bucket = std::env::var(S3_BUCKET_ENV)
            .map_err(|_| StorageError::Configuration(format!("{S3_BUCKET_ENV} not set")))?;

        let region = std::env::var(S3_REGION_ENV).unwrap_or_else(|_| "us-east-1".to_string());

        let endpoint = std::env::var(S3_ENDPOINT_ENV).ok();

        let access_key_id = std::env::var(AWS_ACCESS_KEY_ID_ENV)
            .map_err(|_| StorageError::Configuration(format!("{AWS_ACCESS_KEY_ID_ENV} not set")))?;

        let secret_access_key = std::env::var(AWS_SECRET_ACCESS_KEY_ENV).map_err(|_| {
            StorageError::Configuration(format!("{AWS_SECRET_ACCESS_KEY_ENV} not set"))
        })?;

        // Presigned download TTL knob. Reject non-positive values (a zero or
        // negative TTL would mint an already-expired URL) and fall back to the
        // 15-minute default when unset or unparsable.
        let download_ttl_secs = std::env::var(S3_PRESIGNED_URL_TTL_ENV)
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .filter(|secs| *secs > 0)
            .unwrap_or(DEFAULT_DOWNLOAD_EXPIRATION_SECS);

        Ok(Self {
            bucket,
            region,
            endpoint,
            access_key_id,
            secret_access_key,
            download_ttl_secs,
        })
    }

    /// Create configuration with explicit values (for testing).
    pub fn new(
        bucket: impl Into<String>,
        region: impl Into<String>,
        access_key_id: impl Into<String>,
        secret_access_key: impl Into<String>,
    ) -> Self {
        Self {
            bucket: bucket.into(),
            region: region.into(),
            endpoint: None,
            access_key_id: access_key_id.into(),
            secret_access_key: secret_access_key.into(),
            download_ttl_secs: DEFAULT_DOWNLOAD_EXPIRATION_SECS,
        }
    }

    /// Set a custom endpoint (for S3-compatible services like MinIO).
    #[must_use]
    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    /// Override the presigned download URL TTL (seconds). Non-positive values
    /// are ignored to avoid minting already-expired URLs.
    #[must_use]
    pub fn with_download_ttl(mut self, secs: i64) -> Self {
        if secs > 0 {
            self.download_ttl_secs = secs;
        }
        self
    }
}

// ============================================================================
// Storage Service
// ============================================================================

/// Storage service for S3 operations with presigned URL support.
///
/// Story 103.1: Now includes real S3 client for actual file uploads.
#[derive(Clone)]
pub struct StorageService {
    config: StorageConfig,
    /// S3 client for actual file operations (Story 103.1)
    s3_client: Option<S3Client>,
}

impl std::fmt::Debug for StorageService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StorageService")
            .field("config", &self.config)
            .field("s3_client", &self.s3_client.is_some())
            .finish()
    }
}

impl StorageService {
    /// Create a new storage service with the given configuration.
    ///
    /// Note: This creates the service without an S3 client. Use `with_s3_client()`
    /// or `from_env_async()` to enable actual S3 operations.
    pub fn new(config: StorageConfig) -> Self {
        Self {
            config,
            s3_client: None,
        }
    }

    /// Create a storage service from environment variables (synchronous, no S3 client).
    pub fn from_env() -> Result<Self, StorageError> {
        Ok(Self::new(StorageConfig::from_env()?))
    }

    /// Create a storage service from environment variables with S3 client (Story 103.1).
    ///
    /// This is the preferred constructor for production use as it initializes
    /// the AWS S3 client for actual file operations.
    pub async fn from_env_async() -> Result<Self, StorageError> {
        let config = StorageConfig::from_env()?;
        Self::with_s3_client(config).await
    }

    /// Create a storage service with an initialized S3 client (Story 103.1).
    pub async fn with_s3_client(config: StorageConfig) -> Result<Self, StorageError> {
        let credentials = Credentials::new(
            &config.access_key_id,
            &config.secret_access_key,
            None,
            None,
            "property-management",
        );

        let region = Region::new(config.region.clone());

        let mut s3_config_builder = aws_sdk_s3::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(region)
            .credentials_provider(credentials);

        // Set custom endpoint for S3-compatible services (e.g., MinIO)
        if let Some(ref endpoint) = config.endpoint {
            s3_config_builder = s3_config_builder
                .endpoint_url(endpoint)
                .force_path_style(true);
        }

        let s3_config = s3_config_builder.build();
        let s3_client = S3Client::from_conf(s3_config);

        tracing::info!(
            bucket = %config.bucket,
            region = %config.region,
            endpoint = ?config.endpoint,
            "Initialized S3 client"
        );

        Ok(Self {
            config,
            s3_client: Some(s3_client),
        })
    }

    /// Check if S3 client is initialized.
    pub fn has_s3_client(&self) -> bool {
        self.s3_client.is_some()
    }

    /// Get the S3 client, returning an error if not initialized.
    fn get_s3_client(&self) -> Result<&S3Client, StorageError> {
        self.s3_client.as_ref().ok_or_else(|| {
            StorageError::Configuration(
                "S3 client not initialized. Use from_env_async() or with_s3_client()".to_string(),
            )
        })
    }

    /// Generate a presigned URL for downloading a file.
    ///
    /// # Arguments
    ///
    /// * `key` - The S3 object key (file path in bucket)
    /// * `filename` - Original filename for Content-Disposition header
    /// * `content_type` - MIME type of the file
    /// * `expires_in_secs` - URL validity duration in seconds (default: 15 minutes)
    ///
    /// # Returns
    ///
    /// A presigned URL that allows temporary download access.
    pub async fn generate_download_url(
        &self,
        key: &str,
        filename: &str,
        content_type: &str,
        expires_in_secs: Option<i64>,
    ) -> Result<PresignedUrl, StorageError> {
        let expires_in = expires_in_secs.unwrap_or(DEFAULT_DOWNLOAD_EXPIRATION_SECS);
        let expires_at = Utc::now() + Duration::seconds(expires_in);

        let url = self
            .build_presigned_get_url(key, filename, content_type, expires_in)
            .await?;

        Ok(PresignedUrl { url, expires_at })
    }

    /// Generate a presigned URL for inline (in-browser) preview of a file.
    ///
    /// Like `generate_download_url` but sets `Content-Disposition: inline`
    /// so browsers render the file in-place rather than triggering a download
    /// dialog. Only meaningful for MIME types browsers can display natively
    /// (PDF, images, plain text). Callers should check `supports_inline_preview`
    /// before calling.
    ///
    /// Default expiration: 1 hour (longer than download so embedded viewers
    /// can reload sub-resources without needing a fresh URL immediately).
    pub async fn generate_preview_url(
        &self,
        key: &str,
        content_type: &str,
        expires_in_secs: Option<i64>,
    ) -> Result<PresignedUrl, StorageError> {
        const DEFAULT_PREVIEW_EXPIRATION_SECS: i64 = 60 * 60; // 1 hour
        let expires_in = expires_in_secs.unwrap_or(DEFAULT_PREVIEW_EXPIRATION_SECS);
        let expires_at = Utc::now() + Duration::seconds(expires_in);
        let url = self
            .build_presigned_inline_url(key, content_type, expires_in)
            .await?;
        Ok(PresignedUrl { url, expires_at })
    }

    /// Generate a presigned URL for uploading a file.
    ///
    /// # Arguments
    ///
    /// * `key` - The S3 object key where the file will be stored
    /// * `content_type` - Expected MIME type of the upload
    /// * `content_length` - Exact byte count of the upload. Signed into the
    ///   URL (GH #2320), so S3 rejects a PUT whose `Content-Length` differs —
    ///   this is what actually enforces the 50 MiB cap on the direct PUT path
    ///   (presigned PUT has no `content-length-range` policy like POST does).
    /// * `expires_in_secs` - URL validity duration in seconds (default: 5 minutes)
    ///
    /// # Returns
    ///
    /// A presigned PUT URL that allows temporary upload access.
    pub async fn generate_upload_url(
        &self,
        key: &str,
        content_type: &str,
        content_length: i64,
        expires_in_secs: Option<i64>,
    ) -> Result<PresignedUrl, StorageError> {
        // Validate content type
        if !is_allowed_content_type(content_type) {
            return Err(StorageError::InvalidContentType(content_type.to_string()));
        }

        // Validate size before touching the S3 client so callers get a typed
        // error (and unit tests can pin this without a live client).
        if content_length < 0 {
            return Err(StorageError::UploadError(format!(
                "content_length must be non-negative, got {content_length}"
            )));
        }
        if content_length > MAX_UPLOAD_SIZE_BYTES {
            return Err(StorageError::FileTooLarge(
                content_length,
                MAX_UPLOAD_SIZE_BYTES,
            ));
        }

        let expires_in = expires_in_secs.unwrap_or(DEFAULT_UPLOAD_EXPIRATION_SECS);
        let expires_at = Utc::now() + Duration::seconds(expires_in);

        let url = self
            .build_presigned_put_url(key, content_type, content_length, expires_in)
            .await?;

        Ok(PresignedUrl { url, expires_at })
    }

    /// Build a presigned GET URL with AWS Signature V4.
    ///
    /// P0-05 (dev-team review): this function previously returned an
    /// unsigned URL string labelled "placeholder URL structure … in
    /// production, this would use AWS SigV4 signing", so any S3 bucket
    /// that actually required SigV4 (i.e. real S3 / authenticated MinIO)
    /// rejected every download link. Now uses `aws_sdk_s3`'s built-in
    /// presigner which emits an SigV4 URL with the correct X-Amz-*
    /// query parameters.
    async fn build_presigned_get_url(
        &self,
        key: &str,
        filename: &str,
        content_type: &str,
        expires_in: i64,
    ) -> Result<String, StorageError> {
        let client = self.get_s3_client()?;
        let expires = std::time::Duration::from_secs(expires_in.max(1) as u64);
        let presign_cfg = PresigningConfig::expires_in(expires)
            .map_err(|e| StorageError::PresignError(e.to_string()))?;

        let disposition = format!("attachment; filename=\"{}\"", filename);
        let presigned = client
            .get_object()
            .bucket(&self.config.bucket)
            .key(key)
            .response_content_disposition(disposition)
            .response_content_type(content_type)
            .presigned(presign_cfg)
            .await
            .map_err(|e| StorageError::PresignError(e.to_string()))?;

        tracing::debug!(
            key = %key,
            filename = %filename,
            expires_in = %expires_in,
            "Generated presigned download URL"
        );

        Ok(presigned.uri().to_string())
    }

    /// Build a presigned GET URL with `Content-Disposition: inline`.
    ///
    /// Same as `build_presigned_get_url` but omits attachment filename so
    /// the browser treats the response as an inline resource.
    async fn build_presigned_inline_url(
        &self,
        key: &str,
        content_type: &str,
        expires_in: i64,
    ) -> Result<String, StorageError> {
        let client = self.get_s3_client()?;
        let expires = std::time::Duration::from_secs(expires_in.max(1) as u64);
        let presign_cfg = PresigningConfig::expires_in(expires)
            .map_err(|e| StorageError::PresignError(e.to_string()))?;
        let presigned = client
            .get_object()
            .bucket(&self.config.bucket)
            .key(key)
            .response_content_disposition("inline")
            .response_content_type(content_type)
            .presigned(presign_cfg)
            .await
            .map_err(|e| StorageError::PresignError(e.to_string()))?;
        tracing::debug!(
            key = %key,
            content_type = %content_type,
            expires_in = %expires_in,
            "Generated presigned inline preview URL"
        );
        Ok(presigned.uri().to_string())
    }

    /// Build a presigned PUT URL with AWS Signature V4.
    ///
    /// P0-05 (see build_presigned_get_url comment): real SigV4 via
    /// aws_sdk_s3 instead of the previous unsigned-URL placeholder.
    ///
    /// GH #2320: `Content-Length` is included as a signed header alongside
    /// `Content-Type`, so a PUT whose body length differs from
    /// `content_length` fails S3 signature validation. Without this the 50 MiB
    /// cap was purely advisory — a presigned PUT accepts any body size when
    /// only Content-Type is signed.
    async fn build_presigned_put_url(
        &self,
        key: &str,
        content_type: &str,
        content_length: i64,
        expires_in: i64,
    ) -> Result<String, StorageError> {
        let client = self.get_s3_client()?;
        let expires = std::time::Duration::from_secs(expires_in.max(1) as u64);
        let presign_cfg = PresigningConfig::expires_in(expires)
            .map_err(|e| StorageError::PresignError(e.to_string()))?;

        let presigned = client
            .put_object()
            .bucket(&self.config.bucket)
            .key(key)
            .content_type(content_type)
            .content_length(content_length)
            .presigned(presign_cfg)
            .await
            .map_err(|e| StorageError::PresignError(e.to_string()))?;

        tracing::debug!(
            key = %key,
            content_type = %content_type,
            content_length = %content_length,
            expires_in = %expires_in,
            "Generated presigned upload URL"
        );

        Ok(presigned.uri().to_string())
    }

    /// Get the bucket name.
    pub fn bucket(&self) -> &str {
        &self.config.bucket
    }

    /// Get the region.
    pub fn region(&self) -> &str {
        &self.config.region
    }

    /// Configured TTL (seconds) for presigned download URLs.
    ///
    /// Handlers pass this into `generate_download_url` so the validity window
    /// is driven by the `S3_PRESIGNED_URL_TTL_SECS` config knob rather than a
    /// hard-coded constant.
    pub fn download_ttl_secs(&self) -> i64 {
        self.config.download_ttl_secs
    }

    // =========================================================================
    // Story 103.1: Real S3 Operations
    // =========================================================================

    /// Upload a file to S3 (Story 103.1).
    ///
    /// # Arguments
    ///
    /// * `key` - The S3 object key (file path in bucket)
    /// * `content` - The file content as bytes
    /// * `content_type` - MIME type of the file
    ///
    /// # Returns
    ///
    /// The storage key on success.
    pub async fn upload(
        &self,
        key: &str,
        content: Vec<u8>,
        content_type: &str,
    ) -> Result<String, StorageError> {
        let client = self.get_s3_client()?;
        let size = content.len();

        // Validate content type
        if !is_allowed_content_type(content_type) {
            return Err(StorageError::InvalidContentType(content_type.to_string()));
        }

        // Validate size
        if size as i64 > MAX_UPLOAD_SIZE_BYTES {
            return Err(StorageError::FileTooLarge(
                size as i64,
                MAX_UPLOAD_SIZE_BYTES,
            ));
        }

        let body = ByteStream::from(content);

        client
            .put_object()
            .bucket(&self.config.bucket)
            .key(key)
            .body(body)
            .content_type(content_type)
            .send()
            .await
            .map_err(|e| StorageError::UploadError(format!("S3 PutObject failed: {}", e)))?;

        tracing::info!(
            key = %key,
            content_type = %content_type,
            size_bytes = %size,
            "Uploaded file to S3"
        );

        Ok(key.to_string())
    }

    /// Upload a **system-generated** artifact (e.g. an organization data export
    /// or tenant backup tarball) to S3.
    ///
    /// Unlike [`upload`](Self::upload), this does NOT enforce the user-content
    /// MIME allowlist — the bytes are produced by the server itself, not
    /// uploaded by an end user, so the allowlist (which exists to constrain
    /// user uploads) does not apply. The size cap is still enforced. Use this
    /// ONLY for trusted, server-produced content (issue #979).
    pub async fn upload_system_artifact(
        &self,
        key: &str,
        content: Vec<u8>,
        content_type: &str,
    ) -> Result<String, StorageError> {
        let client = self.get_s3_client()?;
        let size = content.len();

        if size as i64 > MAX_UPLOAD_SIZE_BYTES {
            return Err(StorageError::FileTooLarge(
                size as i64,
                MAX_UPLOAD_SIZE_BYTES,
            ));
        }

        let body = ByteStream::from(content);
        client
            .put_object()
            .bucket(&self.config.bucket)
            .key(key)
            .body(body)
            .content_type(content_type)
            .send()
            .await
            .map_err(|e| StorageError::UploadError(format!("S3 PutObject failed: {}", e)))?;

        tracing::info!(
            key = %key,
            content_type = %content_type,
            size_bytes = %size,
            "Uploaded system artifact to S3"
        );

        Ok(key.to_string())
    }

    /// Download a file from S3 (Story 103.1).
    ///
    /// # Arguments
    ///
    /// * `key` - The S3 object key (file path in bucket)
    ///
    /// # Returns
    ///
    /// The file content as bytes and the content type.
    pub async fn download(&self, key: &str) -> Result<(Vec<u8>, String), StorageError> {
        let client = self.get_s3_client()?;

        let response = client
            .get_object()
            .bucket(&self.config.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| {
                if e.to_string().contains("NoSuchKey") {
                    StorageError::NotFound(key.to_string())
                } else {
                    StorageError::DownloadError(format!("S3 GetObject failed: {}", e))
                }
            })?;

        let content_type = response
            .content_type()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "application/octet-stream".to_string());

        let body = response
            .body
            .collect()
            .await
            .map_err(|e| StorageError::DownloadError(format!("Failed to read body: {}", e)))?;

        let content = body.into_bytes().to_vec();

        tracing::debug!(
            key = %key,
            content_type = %content_type,
            size_bytes = %content.len(),
            "Downloaded file from S3"
        );

        Ok((content, content_type))
    }

    /// Delete a file from S3 (Story 103.1).
    ///
    /// # Arguments
    ///
    /// * `key` - The S3 object key (file path in bucket)
    pub async fn delete(&self, key: &str) -> Result<(), StorageError> {
        let client = self.get_s3_client()?;

        client
            .delete_object()
            .bucket(&self.config.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| StorageError::DeleteError(format!("S3 DeleteObject failed: {}", e)))?;

        tracing::info!(key = %key, "Deleted file from S3");

        Ok(())
    }

    /// Check if a file exists in S3 (Story 103.1).
    pub async fn exists(&self, key: &str) -> Result<bool, StorageError> {
        let client = self.get_s3_client()?;

        let result = client
            .head_object()
            .bucket(&self.config.bucket)
            .key(key)
            .send()
            .await;

        match result {
            Ok(_) => Ok(true),
            Err(e) if e.to_string().contains("NotFound") => Ok(false),
            Err(e) => Err(StorageError::S3Error(format!("HeadObject failed: {}", e))),
        }
    }

    /// Generate a presigned download URL using the AWS SDK (Story 103.1).
    ///
    /// This uses proper AWS Signature V4 signing for production use.
    pub async fn generate_presigned_download_url(
        &self,
        key: &str,
        expires_in_secs: Option<i64>,
    ) -> Result<PresignedUrl, StorageError> {
        let client = self.get_s3_client()?;
        let expires_in = expires_in_secs.unwrap_or(DEFAULT_DOWNLOAD_EXPIRATION_SECS);
        let expires_at = Utc::now() + Duration::seconds(expires_in);

        let presigning_config =
            PresigningConfig::expires_in(StdDuration::from_secs(expires_in as u64))
                .map_err(|e| StorageError::PresignError(format!("Invalid expiration: {}", e)))?;

        let presigned_request = client
            .get_object()
            .bucket(&self.config.bucket)
            .key(key)
            .presigned(presigning_config)
            .await
            .map_err(|e| StorageError::PresignError(format!("Presigning failed: {}", e)))?;

        let url = presigned_request.uri().to_string();

        tracing::debug!(
            key = %key,
            expires_in = %expires_in,
            "Generated presigned download URL"
        );

        Ok(PresignedUrl { url, expires_at })
    }

    /// Generate a presigned upload URL using the AWS SDK (Story 103.1).
    ///
    /// This uses proper AWS Signature V4 signing for production use.
    pub async fn generate_presigned_upload_url(
        &self,
        key: &str,
        content_type: &str,
        expires_in_secs: Option<i64>,
    ) -> Result<PresignedUrl, StorageError> {
        let client = self.get_s3_client()?;
        let expires_in = expires_in_secs.unwrap_or(DEFAULT_UPLOAD_EXPIRATION_SECS);
        let expires_at = Utc::now() + Duration::seconds(expires_in);

        // Validate content type
        if !is_allowed_content_type(content_type) {
            return Err(StorageError::InvalidContentType(content_type.to_string()));
        }

        let presigning_config =
            PresigningConfig::expires_in(StdDuration::from_secs(expires_in as u64))
                .map_err(|e| StorageError::PresignError(format!("Invalid expiration: {}", e)))?;

        let presigned_request = client
            .put_object()
            .bucket(&self.config.bucket)
            .key(key)
            .content_type(content_type)
            .presigned(presigning_config)
            .await
            .map_err(|e| StorageError::PresignError(format!("Presigning failed: {}", e)))?;

        let url = presigned_request.uri().to_string();

        tracing::debug!(
            key = %key,
            content_type = %content_type,
            expires_in = %expires_in,
            "Generated presigned upload URL"
        );

        Ok(PresignedUrl { url, expires_at })
    }

    /// Check S3 connectivity (Story 103.1).
    ///
    /// Used for health checks to verify S3 is accessible.
    pub async fn health_check(&self) -> Result<bool, StorageError> {
        let client = self.get_s3_client()?;

        client
            .head_bucket()
            .bucket(&self.config.bucket)
            .send()
            .await
            .map_err(|e| StorageError::S3Error(format!("Bucket health check failed: {}", e)))?;

        Ok(true)
    }

    /// Copy an object within S3 (Story 103.1).
    pub async fn copy(&self, source_key: &str, dest_key: &str) -> Result<(), StorageError> {
        let client = self.get_s3_client()?;

        let copy_source = format!("{}/{}", self.config.bucket, source_key);

        client
            .copy_object()
            .bucket(&self.config.bucket)
            .copy_source(&copy_source)
            .key(dest_key)
            .send()
            .await
            .map_err(|e| StorageError::S3Error(format!("CopyObject failed: {}", e)))?;

        tracing::info!(
            source = %source_key,
            dest = %dest_key,
            "Copied object in S3"
        );

        Ok(())
    }

    /// List objects with a prefix (Story 103.1).
    pub async fn list_objects(&self, prefix: &str) -> Result<Vec<String>, StorageError> {
        let client = self.get_s3_client()?;

        let response = client
            .list_objects_v2()
            .bucket(&self.config.bucket)
            .prefix(prefix)
            .send()
            .await
            .map_err(|e| StorageError::S3Error(format!("ListObjects failed: {}", e)))?;

        let keys: Vec<String> = response
            .contents()
            .iter()
            .filter_map(|obj| obj.key().map(|k| k.to_string()))
            .collect();

        Ok(keys)
    }
}

// ============================================================================
// Content Type Utilities
// ============================================================================

/// Allowed MIME types for upload.
///
/// Re-export of the single source of truth in `common` (GH #2320) — the same
/// list the document handlers (`db::models::ALLOWED_MIME_TYPES`) enforce, so
/// handler-side and storage-side validation can never drift. Same pattern as
/// [`supports_inline_preview`] (GH #1413).
pub use common::ALLOWED_UPLOAD_MIME_TYPES as ALLOWED_MIME_TYPES;

/// Check if a content type is allowed for upload.
pub fn is_allowed_content_type(content_type: &str) -> bool {
    ALLOWED_MIME_TYPES.contains(&content_type)
}

/// Get content type from filename extension.
pub fn get_content_type(filename: &str) -> &'static str {
    let extension = filename.rsplit('.').next().unwrap_or("");
    match extension.to_lowercase().as_str() {
        "pdf" => "application/pdf",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "txt" => "text/plain",
        "csv" => "text/csv",
        _ => "application/octet-stream",
    }
}

/// Check if a content type supports inline preview (not download).
///
/// Thin re-export of [`common::supports_inline_preview`] — the single source of
/// truth shared with `db::models::Document::supports_preview` (GH #1413).
pub fn supports_inline_preview(content_type: &str) -> bool {
    common::supports_inline_preview(content_type)
}

/// Generate a unique storage key for a file.
///
/// Format: `{org_id}/{year}/{month}/{uuid}_{filename}`
pub fn generate_storage_key(org_id: uuid::Uuid, filename: &str) -> String {
    let now = Utc::now();
    let file_uuid = uuid::Uuid::new_v4();

    // Sanitize filename
    let safe_filename: String = filename
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();

    format!(
        "{}/{}/{}/{}_{safe_filename}",
        org_id,
        now.format("%Y"),
        now.format("%m"),
        file_uuid
    )
}

/// Generate a callback token for upload completion verification.
///
/// Uses OS CSPRNG directly; callback tokens authenticate the uploader to the
/// completion endpoint.
pub fn generate_callback_token() -> String {
    use rand::TryRng;
    let mut bytes = [0u8; 32];
    rand::rngs::SysRng
        .try_fill_bytes(&mut bytes)
        .expect("OS rng failed");
    hex::encode(bytes)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_content_type() {
        assert_eq!(get_content_type("document.pdf"), "application/pdf");
        assert_eq!(get_content_type("image.PNG"), "image/png");
        assert_eq!(get_content_type("photo.jpeg"), "image/jpeg");
        assert_eq!(
            get_content_type("data.xlsx"),
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
        );
        assert_eq!(get_content_type("unknown.xyz"), "application/octet-stream");
    }

    #[test]
    fn test_is_allowed_content_type() {
        assert!(is_allowed_content_type("application/pdf"));
        assert!(is_allowed_content_type("image/png"));
        assert!(!is_allowed_content_type("application/javascript"));
        assert!(!is_allowed_content_type("text/html"));
    }

    #[test]
    fn test_supports_inline_preview() {
        assert!(supports_inline_preview("application/pdf"));
        assert!(supports_inline_preview("image/png"));
        assert!(!supports_inline_preview(
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        ));
    }

    #[test]
    fn test_generate_storage_key() {
        let org_id = uuid::Uuid::new_v4();
        let key = generate_storage_key(org_id, "test document.pdf");

        assert!(key.starts_with(&org_id.to_string()));
        assert!(key.ends_with("test_document.pdf"));
        assert!(key.contains('/'));
    }

    #[test]
    fn test_generate_callback_token() {
        let token = generate_callback_token();
        assert_eq!(token.len(), 64); // 32 bytes as hex
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_storage_config_new() {
        let config = StorageConfig::new("my-bucket", "us-west-2", "key", "secret");
        assert_eq!(config.bucket, "my-bucket");
        assert_eq!(config.region, "us-west-2");
        assert!(config.endpoint.is_none());
    }

    #[test]
    fn test_storage_config_with_endpoint() {
        let config = StorageConfig::new("my-bucket", "us-west-2", "key", "secret")
            .with_endpoint("http://localhost:9000");
        assert_eq!(config.endpoint, Some("http://localhost:9000".to_string()));
    }

    #[test]
    fn test_storage_config_default_download_ttl() {
        let config = StorageConfig::new("my-bucket", "us-west-2", "key", "secret");
        assert_eq!(config.download_ttl_secs, DEFAULT_DOWNLOAD_EXPIRATION_SECS);
        assert_eq!(config.download_ttl_secs, 15 * 60);
    }

    #[test]
    fn test_storage_config_with_download_ttl() {
        let config =
            StorageConfig::new("my-bucket", "us-west-2", "key", "secret").with_download_ttl(300);
        assert_eq!(config.download_ttl_secs, 300);
    }

    #[test]
    fn test_storage_config_with_download_ttl_rejects_non_positive() {
        // A zero / negative TTL would mint an already-expired URL — keep the default.
        let base = StorageConfig::new("my-bucket", "us-west-2", "key", "secret");
        assert_eq!(
            base.clone().with_download_ttl(0).download_ttl_secs,
            DEFAULT_DOWNLOAD_EXPIRATION_SECS
        );
        assert_eq!(
            base.with_download_ttl(-1).download_ttl_secs,
            DEFAULT_DOWNLOAD_EXPIRATION_SECS
        );
    }

    #[test]
    fn test_storage_service_exposes_download_ttl() {
        let config =
            StorageConfig::new("my-bucket", "us-west-2", "key", "secret").with_download_ttl(900);
        let service = StorageService::new(config);
        assert_eq!(service.download_ttl_secs(), 900);
    }

    #[test]
    fn test_storage_service_no_s3_client_by_default() {
        let config = StorageConfig::new("test-bucket", "us-east-1", "key", "secret");
        let service = StorageService::new(config);
        assert!(!service.has_s3_client());
        assert!(service.get_s3_client().is_err());
    }

    #[test]
    fn test_supports_inline_preview_exhaustive() {
        for ct in &[
            "application/pdf",
            "image/png",
            "image/jpeg",
            "image/gif",
            "image/webp",
            "text/plain",
        ] {
            assert!(supports_inline_preview(ct), "{ct} should support preview");
        }
        for ct in &[
            "application/msword",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            "application/vnd.ms-excel",
            "application/octet-stream",
            "text/csv",
        ] {
            assert!(
                !supports_inline_preview(ct),
                "{ct} should not support preview"
            );
        }
    }

    // ========================================================================
    // Presigned-URL minting / expiry coverage (#1377)
    //
    // SigV4 presigning is computed entirely client-side by the AWS SDK — no
    // network call is made when we `.presigned(...)`. That lets us mint a real
    // signed URL with a fixed (fake) credential and assert its shape + expiry
    // deterministically in CI without any live S3 / MinIO. These tests guard
    // the two minting entry points the document download/preview handlers call:
    // `generate_download_url` (attachment) and `generate_preview_url` (inline).
    // ========================================================================

    /// Build a StorageService backed by a presigner-capable S3 client using a
    /// static fake credential. Path-style + a dummy endpoint keep the SDK from
    /// resolving any real region/endpoint. No I/O is performed by presigning.
    async fn presigning_service(ttl: i64) -> StorageService {
        let config = StorageConfig::new("test-bucket", "us-east-1", "AKIATEST", "secretkey")
            .with_endpoint("http://s3.local:9000")
            .with_download_ttl(ttl);
        StorageService::with_s3_client(config)
            .await
            .expect("with_s3_client should build offline (no network for presign)")
    }

    /// Parse the `X-Amz-Expires` query parameter (seconds) out of a presigned
    /// URL. Returns `None` if absent.
    fn amz_expires_secs(url: &str) -> Option<i64> {
        url.split(['?', '&'])
            .find_map(|kv| kv.strip_prefix("X-Amz-Expires="))
            .and_then(|v| v.parse::<i64>().ok())
    }

    #[tokio::test]
    async fn test_generate_download_url_mints_signed_attachment_url() {
        let service = presigning_service(DEFAULT_DOWNLOAD_EXPIRATION_SECS).await;
        let before = Utc::now();
        let presigned = service
            .generate_download_url(
                "org/2024/report.pdf",
                "report.pdf",
                "application/pdf",
                Some(900),
            )
            .await
            .expect("download URL should mint");

        // SigV4 query params prove the URL is actually signed (P0-05 regression:
        // an unsigned placeholder URL would lack these and be rejected by S3).
        assert!(
            presigned.url.contains("X-Amz-Signature="),
            "minted URL must carry a SigV4 signature, got {}",
            presigned.url
        );
        assert!(
            presigned.url.contains("X-Amz-Credential="),
            "minted URL must carry the SigV4 credential scope, got {}",
            presigned.url
        );
        assert!(
            presigned.url.contains("report.pdf"),
            "minted URL must reference the object key, got {}",
            presigned.url
        );

        // Expiry: expires_at must reflect the requested 900s TTL (allow a small
        // window for clock advance during the call).
        let delta = (presigned.expires_at - before).num_seconds();
        assert!(
            (895..=905).contains(&delta),
            "expires_at should be ~900s in the future, was {delta}s"
        );
        // The signed URL itself must encode the same TTL.
        assert_eq!(
            amz_expires_secs(&presigned.url),
            Some(900),
            "X-Amz-Expires must match the requested TTL, got {}",
            presigned.url
        );
    }

    #[tokio::test]
    async fn test_generate_download_url_defaults_to_15_minute_expiry() {
        let service = presigning_service(DEFAULT_DOWNLOAD_EXPIRATION_SECS).await;
        let before = Utc::now();
        // `None` TTL → DEFAULT_DOWNLOAD_EXPIRATION_SECS (15 min).
        let presigned = service
            .generate_download_url("org/k.pdf", "k.pdf", "application/pdf", None)
            .await
            .expect("download URL should mint");

        let delta = (presigned.expires_at - before).num_seconds();
        assert!(
            (DEFAULT_DOWNLOAD_EXPIRATION_SECS - 5..=DEFAULT_DOWNLOAD_EXPIRATION_SECS + 5)
                .contains(&delta),
            "default download expiry should be {DEFAULT_DOWNLOAD_EXPIRATION_SECS}s, was {delta}s"
        );
        assert_eq!(
            amz_expires_secs(&presigned.url),
            Some(DEFAULT_DOWNLOAD_EXPIRATION_SECS),
            "X-Amz-Expires must encode the 15-minute default"
        );
    }

    #[tokio::test]
    async fn test_generate_preview_url_mints_inline_url_with_default_one_hour() {
        let service = presigning_service(DEFAULT_DOWNLOAD_EXPIRATION_SECS).await;
        let before = Utc::now();
        // `None` TTL → 1-hour preview default (longer than download).
        let presigned = service
            .generate_preview_url("org/img.png", "image/png", None)
            .await
            .expect("preview URL should mint");

        assert!(
            presigned.url.contains("X-Amz-Signature="),
            "minted preview URL must be SigV4-signed, got {}",
            presigned.url
        );
        // Inline disposition is the whole point of preview vs download. The SDK
        // percent-encodes the `response-content-disposition` query value, so
        // match the key and the (encoded or raw) `inline` token loosely.
        assert!(
            presigned.url.contains("response-content-disposition="),
            "preview URL must set a response-content-disposition, got {}",
            presigned.url
        );
        assert!(
            presigned.url.contains("inline"),
            "preview URL disposition must be inline, got {}",
            presigned.url
        );

        let delta = (presigned.expires_at - before).num_seconds();
        assert!(
            (3595..=3605).contains(&delta),
            "default preview expiry should be 3600s, was {delta}s"
        );
        assert_eq!(
            amz_expires_secs(&presigned.url),
            Some(3600),
            "X-Amz-Expires must encode the 1-hour preview default"
        );
    }

    #[tokio::test]
    async fn test_generate_preview_url_honours_explicit_ttl() {
        let service = presigning_service(DEFAULT_DOWNLOAD_EXPIRATION_SECS).await;
        let presigned = service
            .generate_preview_url("org/img.png", "image/png", Some(120))
            .await
            .expect("preview URL should mint");
        assert_eq!(
            amz_expires_secs(&presigned.url),
            Some(120),
            "explicit preview TTL must be honoured in the signed URL"
        );
    }

    // ========================================================================
    // GH #2320 — presigned PUT must sign Content-Length.
    //
    // A presigned PUT that signs only Content-Type accepts a body of any size
    // (up to S3's 5 GB single-PUT limit) — the 50 MiB cap was advisory. These
    // pin that `generate_upload_url` (a) signs the exact byte count so S3
    // rejects a mismatched PUT, and (b) rejects oversize/negative sizes before
    // ever building a URL.
    // ========================================================================

    #[tokio::test]
    async fn test_generate_upload_url_signs_content_length() {
        let service = presigning_service(DEFAULT_DOWNLOAD_EXPIRATION_SECS).await;
        let presigned = service
            .generate_upload_url("org/2026/07/file.pdf", "application/pdf", 1024, None)
            .await
            .expect("upload URL should mint");

        assert!(
            presigned.url.contains("X-Amz-Signature="),
            "minted upload URL must be SigV4-signed, got {}",
            presigned.url
        );
        // Content-Length (and Content-Type) must be *signed* headers — that is
        // what makes S3 reject a PUT whose body length differs from the
        // declared size. The `;` separator is percent-encoded in the URL.
        let signed_headers = presigned
            .url
            .split(['?', '&'])
            .find_map(|kv| kv.strip_prefix("X-Amz-SignedHeaders="))
            .expect("presigned URL must carry X-Amz-SignedHeaders")
            .to_lowercase()
            .replace("%3b", ";");
        assert!(
            signed_headers.contains("content-length"),
            "content-length must be a signed header, got {signed_headers}"
        );
        assert!(
            signed_headers.contains("content-type"),
            "content-type must be a signed header, got {signed_headers}"
        );
    }

    #[tokio::test]
    async fn test_generate_upload_url_allows_exactly_max_size() {
        let service = presigning_service(DEFAULT_DOWNLOAD_EXPIRATION_SECS).await;
        assert!(service
            .generate_upload_url("org/k.pdf", "application/pdf", MAX_UPLOAD_SIZE_BYTES, None)
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn test_generate_upload_url_rejects_oversize_content_length() {
        let service = presigning_service(DEFAULT_DOWNLOAD_EXPIRATION_SECS).await;
        let err = service
            .generate_upload_url(
                "org/k.pdf",
                "application/pdf",
                MAX_UPLOAD_SIZE_BYTES + 1,
                None,
            )
            .await
            .expect_err("oversize content_length must be rejected");
        assert!(
            matches!(err, StorageError::FileTooLarge(..)),
            "expected FileTooLarge, got {err:?}"
        );
    }

    #[tokio::test]
    async fn test_generate_upload_url_rejects_negative_content_length() {
        let service = presigning_service(DEFAULT_DOWNLOAD_EXPIRATION_SECS).await;
        let err = service
            .generate_upload_url("org/k.pdf", "application/pdf", -1, None)
            .await
            .expect_err("negative content_length must be rejected");
        assert!(
            matches!(err, StorageError::UploadError(_)),
            "expected UploadError, got {err:?}"
        );
    }

    #[tokio::test]
    async fn test_generate_upload_url_rejects_disallowed_content_type() {
        let service = presigning_service(DEFAULT_DOWNLOAD_EXPIRATION_SECS).await;
        let err = service
            .generate_upload_url("org/k.html", "text/html", 10, None)
            .await
            .expect_err("disallowed content type must be rejected");
        assert!(
            matches!(err, StorageError::InvalidContentType(_)),
            "expected InvalidContentType, got {err:?}"
        );
    }

    #[tokio::test]
    async fn test_minting_without_s3_client_errors_cleanly() {
        // The download/preview handlers gate on storage being configured; this
        // pins the no-client failure mode so a missing S3 client surfaces as a
        // typed StorageError rather than a panic.
        let service = StorageService::new(StorageConfig::new(
            "b",
            "us-east-1",
            "AKIATEST",
            "secretkey",
        ));
        let err = service
            .generate_download_url("k", "k.pdf", "application/pdf", Some(300))
            .await
            .expect_err("minting without an S3 client must error");
        assert!(
            matches!(err, StorageError::Configuration(_)),
            "expected Configuration error, got {err:?}"
        );
    }
}
