//! Signature request repository for e-signature integration (Story 7B.3).
//!
//! Provides database operations for managing e-signature workflows.

use chrono::{Duration, Utc};
use sqlx::{Error as SqlxError, Executor, PgPool, Postgres, Row};
use uuid::Uuid;

use crate::models::{
    CreateSignatureRequest, SignatureRequest, SignatureRequestStatus, Signer, SignerStatus,
};

/// Repository for signature request operations.
#[derive(Clone)]
pub struct SignatureRequestRepository {
    pool: PgPool,
}

impl SignatureRequestRepository {
    /// Create a new SignatureRequestRepository.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new signature request.
    pub async fn create(
        &self,
        document_id: Uuid,
        organization_id: Uuid,
        created_by: Uuid,
        request: &CreateSignatureRequest,
    ) -> Result<SignatureRequest, SqlxError> {
        // Convert CreateSigners to Signers
        let signers: Vec<Signer> = request.signers.iter().map(|s| s.clone().into()).collect();
        let signers_json = serde_json::to_value(&signers).unwrap();

        // Calculate expiration
        let expires_at = request
            .expires_in_days
            .map(|days| Utc::now() + Duration::days(days.into()))
            .or_else(|| Some(Utc::now() + Duration::days(30))); // Default 30 days

        let record = sqlx::query_as::<_, SignatureRequest>(
            r#"
            INSERT INTO signature_requests (
                document_id, organization_id, subject, message, signers,
                provider, expires_at, created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(document_id)
        .bind(organization_id)
        .bind(&request.subject)
        .bind(&request.message)
        .bind(&signers_json)
        .bind(&request.provider)
        .bind(expires_at)
        .bind(created_by)
        .fetch_one(&self.pool)
        .await?;

        // Also update the document's signature status
        sqlx::query(
            r#"
            UPDATE documents
            SET signature_status = 'pending', signature_request_id = $1
            WHERE id = $2
            "#,
        )
        .bind(record.id)
        .bind(document_id)
        .execute(&self.pool)
        .await?;

        Ok(record)
    }

    /// Find a signature request by ID.
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<SignatureRequest>, SqlxError> {
        sqlx::query_as::<_, SignatureRequest>(
            r#"
            SELECT * FROM signature_requests
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Find a signature request by ID using an RLS-scoped connection.
    pub async fn find_by_id_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<SignatureRequest>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, SignatureRequest>(
            r#"
            SELECT * FROM signature_requests
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(executor)
        .await
    }

    /// Find signature requests for a document.
    pub async fn find_by_document(
        &self,
        document_id: Uuid,
    ) -> Result<Vec<SignatureRequest>, SqlxError> {
        sqlx::query_as::<_, SignatureRequest>(
            r#"
            SELECT * FROM signature_requests
            WHERE document_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(document_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Find signature request by provider request ID (for webhook handling).
    pub async fn find_by_provider_request_id(
        &self,
        provider: &str,
        provider_request_id: &str,
    ) -> Result<Option<SignatureRequest>, SqlxError> {
        sqlx::query_as::<_, SignatureRequest>(
            r#"
            SELECT * FROM signature_requests
            WHERE provider = $1 AND provider_request_id = $2
            "#,
        )
        .bind(provider)
        .bind(provider_request_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// List pending signature requests for an organization.
    pub async fn list_pending(
        &self,
        organization_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<SignatureRequest>, SqlxError> {
        sqlx::query_as::<_, SignatureRequest>(
            r#"
            SELECT * FROM signature_requests
            WHERE organization_id = $1
              AND status IN ('pending', 'in_progress')
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(organization_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    /// Count signature requests for a document.
    pub async fn count_by_document(&self, document_id: Uuid) -> Result<i64, SqlxError> {
        let row = sqlx::query(
            r#"
            SELECT COUNT(*) as count
            FROM signature_requests
            WHERE document_id = $1
            "#,
        )
        .bind(document_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.get("count"))
    }

    /// Update signer status.
    pub async fn update_signer_status(
        &self,
        id: Uuid,
        signer_email: &str,
        new_status: SignerStatus,
        decline_reason: Option<&str>,
    ) -> Result<SignatureRequest, SqlxError> {
        // First get the current request
        let request = self
            .find_by_id(id)
            .await?
            .ok_or_else(|| SqlxError::RowNotFound)?;

        // Update the signer
        let mut signers = request.signers.clone();
        let mut signer_found = false;
        for signer in &mut signers {
            if signer.email.eq_ignore_ascii_case(signer_email) {
                signer.status = new_status;
                if matches!(new_status, SignerStatus::Signed) {
                    signer.signed_at = Some(Utc::now());
                } else if matches!(new_status, SignerStatus::Declined) {
                    signer.declined_at = Some(Utc::now());
                    signer.declined_reason = decline_reason.map(String::from);
                }
                signer_found = true;
                break;
            }
        }

        if !signer_found {
            return Err(SqlxError::RowNotFound);
        }

        // Determine new request status
        let all_signed = signers
            .iter()
            .all(|s| matches!(s.status, SignerStatus::Signed));
        let any_declined = signers
            .iter()
            .any(|s| matches!(s.status, SignerStatus::Declined));
        let any_signed = signers
            .iter()
            .any(|s| matches!(s.status, SignerStatus::Signed));

        let new_request_status = if any_declined {
            SignatureRequestStatus::Declined
        } else if all_signed {
            SignatureRequestStatus::Completed
        } else if any_signed {
            SignatureRequestStatus::InProgress
        } else {
            SignatureRequestStatus::Pending
        };

        let completed_at = if all_signed { Some(Utc::now()) } else { None };

        let signers_json = serde_json::to_value(&signers).unwrap();

        let record = sqlx::query_as::<_, SignatureRequest>(
            r#"
            UPDATE signature_requests
            SET signers = $2,
                status = $3::signature_request_status,
                completed_at = COALESCE($4, completed_at),
                updated_at = now()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&signers_json)
        .bind(new_request_status.to_string())
        .bind(completed_at)
        .fetch_one(&self.pool)
        .await?;

        // Update document signature status
        let doc_status = match new_request_status {
            SignatureRequestStatus::Completed => "signed",
            SignatureRequestStatus::Declined
            | SignatureRequestStatus::Cancelled
            | SignatureRequestStatus::Expired => "none",
            SignatureRequestStatus::InProgress => "partial",
            SignatureRequestStatus::Pending => "pending",
        };

        sqlx::query("UPDATE documents SET signature_status = $1 WHERE signature_request_id = $2")
            .bind(doc_status)
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(record)
    }

    /// Cancel a signature request.
    pub async fn cancel(
        &self,
        id: Uuid,
        _reason: Option<&str>,
    ) -> Result<SignatureRequest, SqlxError> {
        let record = sqlx::query_as::<_, SignatureRequest>(
            r#"
            UPDATE signature_requests
            SET status = 'cancelled',
                updated_at = now()
            WHERE id = $1
              AND status IN ('pending', 'in_progress')
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| SqlxError::RowNotFound)?;

        // Update document signature status
        sqlx::query(
            "UPDATE documents SET signature_status = 'none', signature_request_id = NULL WHERE signature_request_id = $1",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(record)
    }

    /// Update provider request ID (after sending to external provider).
    pub async fn update_provider_request_id(
        &self,
        id: Uuid,
        provider_request_id: &str,
        provider_metadata: Option<serde_json::Value>,
    ) -> Result<SignatureRequest, SqlxError> {
        sqlx::query_as::<_, SignatureRequest>(
            r#"
            UPDATE signature_requests
            SET provider_request_id = $2,
                provider_metadata = COALESCE($3, provider_metadata),
                updated_at = now()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(provider_request_id)
        .bind(provider_metadata)
        .fetch_one(&self.pool)
        .await
    }

    /// Set signed document after completion.
    pub async fn set_signed_document(
        &self,
        id: Uuid,
        signed_document_id: Uuid,
    ) -> Result<SignatureRequest, SqlxError> {
        sqlx::query_as::<_, SignatureRequest>(
            r#"
            UPDATE signature_requests
            SET signed_document_id = $2,
                updated_at = now()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(signed_document_id)
        .fetch_one(&self.pool)
        .await
    }

    /// Expire old pending requests.
    pub async fn expire_old_requests(&self) -> Result<i64, SqlxError> {
        let result = sqlx::query(
            r#"
            UPDATE signature_requests
            SET status = 'expired', updated_at = now()
            WHERE status IN ('pending', 'in_progress')
              AND expires_at < now()
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Also update documents
        sqlx::query(
            r#"
            UPDATE documents
            SET signature_status = 'none', signature_request_id = NULL
            WHERE signature_request_id IN (
                SELECT id FROM signature_requests WHERE status = 'expired'
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as i64)
    }
}
