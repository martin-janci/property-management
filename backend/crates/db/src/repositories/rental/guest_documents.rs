//! Guest ID document CRUD (Story 18.2, #1687).

use super::RentalRepository;
use crate::models::rental::{CreateGuestIdDocument, RentalGuestIdDocument};
use sqlx::Error as SqlxError;
use uuid::Uuid;

impl RentalRepository {
    // ========================================================================
    // Guest ID Documents (Story 18.2, #1687)
    // ========================================================================

    /// Insert a guest ID-document row, org-scoped.
    ///
    /// SECURITY (#1687): `organization_id` is taken from the authenticated
    /// tenant (never the request body), and the FORCE-RLS policy on
    /// `rental_guest_id_documents` re-asserts org isolation at the DB layer.
    pub async fn create_guest_id_document(
        &self,
        org_id: Uuid,
        data: CreateGuestIdDocument,
    ) -> Result<RentalGuestIdDocument, SqlxError> {
        let doc = sqlx::query_as::<_, RentalGuestIdDocument>(
            r#"
            INSERT INTO rental_guest_id_documents (
                guest_id, organization_id, file_key, mime, uploaded_by
            )
            VALUES ($1, $2, $3, $4, $5)
            RETURNING
                id, guest_id, organization_id, file_key, mime, uploaded_by,
                extraction_json, extracted_at, created_at
            "#,
        )
        .bind(data.guest_id)
        .bind(org_id)
        .bind(&data.file_key)
        .bind(&data.mime)
        .bind(data.uploaded_by)
        .fetch_one(&self.pool)
        .await?;

        Ok(doc)
    }

    /// Fetch the most-recently-uploaded ID document for a guest, org-scoped.
    ///
    /// SECURITY (#1687): the `AND organization_id = $2` guard plus FORCE RLS
    /// prevent reading another org's document. Returns `None` when the guest has
    /// no document (or the guest belongs to another org).
    pub async fn latest_guest_id_document_for_org(
        &self,
        org_id: Uuid,
        guest_id: Uuid,
    ) -> Result<Option<RentalGuestIdDocument>, SqlxError> {
        let doc = sqlx::query_as::<_, RentalGuestIdDocument>(
            r#"
            SELECT
                id, guest_id, organization_id, file_key, mime, uploaded_by,
                extraction_json, extracted_at, created_at
            FROM rental_guest_id_documents
            WHERE guest_id = $1 AND organization_id = $2
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(guest_id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(doc)
    }

    /// Persist an OCR extraction result onto a guest ID-document row, org-scoped.
    ///
    /// SECURITY (#1687): the `AND organization_id = $3` guard plus FORCE RLS keep
    /// the write inside the caller's org. Returns `None` when no row matched.
    pub async fn set_guest_id_document_extraction(
        &self,
        org_id: Uuid,
        document_id: Uuid,
        extraction: serde_json::Value,
    ) -> Result<Option<RentalGuestIdDocument>, SqlxError> {
        let doc = sqlx::query_as::<_, RentalGuestIdDocument>(
            r#"
            UPDATE rental_guest_id_documents SET
                extraction_json = $1,
                extracted_at = NOW()
            WHERE id = $2 AND organization_id = $3
            RETURNING
                id, guest_id, organization_id, file_key, mime, uploaded_by,
                extraction_json, extracted_at, created_at
            "#,
        )
        .bind(extraction)
        .bind(document_id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(doc)
    }
}
