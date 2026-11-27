//! Listing inquiries and realtor responses (Story 33.3).

use super::RealityPortalRepository;
use crate::models::reality_portal::*;
use sqlx::Error as SqlxError;
use uuid::Uuid;

impl RealityPortalRepository {
    // ========================================================================
    // Listing Inquiries (Story 33.3)
    // ========================================================================

    /// Create listing inquiry.
    pub async fn create_inquiry(
        &self,
        listing_id: Uuid,
        realtor_id: Uuid,
        user_id: Option<Uuid>,
        data: CreateListingInquiry,
    ) -> Result<ListingInquiry, SqlxError> {
        sqlx::query_as::<_, ListingInquiry>(
            r#"
            INSERT INTO listing_inquiries (
                listing_id, realtor_id, user_id, name, email, phone,
                message, inquiry_type, preferred_contact, preferred_time
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, COALESCE($8, 'info'), COALESCE($9, 'email'), $10)
            RETURNING *
            "#,
        )
        .bind(listing_id)
        .bind(realtor_id)
        .bind(user_id)
        .bind(&data.name)
        .bind(&data.email)
        .bind(&data.phone)
        .bind(&data.message)
        .bind(&data.inquiry_type)
        .bind(&data.preferred_contact)
        .bind(&data.preferred_time)
        .fetch_one(&self.pool)
        .await
    }

    /// Get inquiries for a realtor.
    pub async fn get_realtor_inquiries(
        &self,
        realtor_id: Uuid,
        status: Option<String>,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<ListingInquiry>, SqlxError> {
        sqlx::query_as::<_, ListingInquiry>(
            r#"
            SELECT * FROM listing_inquiries
            WHERE realtor_id = $1 AND ($2::text IS NULL OR status = $2)
            ORDER BY created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(realtor_id)
        .bind(&status)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    /// Count inquiries for a realtor (matching the same filter as
    /// `get_realtor_inquiries`). Used by paginated routes to expose the
    /// true `total` instead of returning `len()` of the current page.
    pub async fn count_realtor_inquiries(
        &self,
        realtor_id: Uuid,
        status: Option<String>,
    ) -> Result<i64, SqlxError> {
        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) FROM listing_inquiries
            WHERE realtor_id = $1 AND ($2::text IS NULL OR status = $2)
            "#,
        )
        .bind(realtor_id)
        .bind(&status)
        .fetch_one(&self.pool)
        .await
    }

    /// Get inquiries submitted by a buyer (the authenticated `user_id` that
    /// created them). This is the buyer-axis counterpart to
    /// [`get_realtor_inquiries`](Self::get_realtor_inquiries): the realtor view
    /// scopes on `realtor_id`, the buyer view scopes on `user_id`.
    pub async fn get_buyer_inquiries(
        &self,
        user_id: Uuid,
        status: Option<String>,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<ListingInquiry>, SqlxError> {
        sqlx::query_as::<_, ListingInquiry>(
            r#"
            SELECT * FROM listing_inquiries
            WHERE user_id = $1 AND ($2::text IS NULL OR status = $2)
            ORDER BY created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(user_id)
        .bind(&status)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    /// Count inquiries submitted by a buyer (matching the same filter as
    /// [`get_buyer_inquiries`](Self::get_buyer_inquiries)). Exposes the true
    /// `total` to paginated routes instead of the current page's `len()`
    /// (the bug fixed for the realtor axis in PR #919).
    pub async fn count_buyer_inquiries(
        &self,
        user_id: Uuid,
        status: Option<String>,
    ) -> Result<i64, SqlxError> {
        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) FROM listing_inquiries
            WHERE user_id = $1 AND ($2::text IS NULL OR status = $2)
            "#,
        )
        .bind(user_id)
        .bind(&status)
        .fetch_one(&self.pool)
        .await
    }

    /// Get a single inquiry by id, scoped to the owning realtor.
    ///
    /// Single-row query — replaces the previous list-then-filter pattern in
    /// the handler, which silently 404'd any inquiry beyond the first 100
    /// for power users.
    pub async fn get_inquiry_for_realtor(
        &self,
        inquiry_id: Uuid,
        realtor_id: Uuid,
    ) -> Result<Option<ListingInquiry>, SqlxError> {
        sqlx::query_as::<_, ListingInquiry>(
            "SELECT * FROM listing_inquiries WHERE id = $1 AND realtor_id = $2",
        )
        .bind(inquiry_id)
        .bind(realtor_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Who to tell that a realtor answered: the inquirer's contact from the
    /// inquiry row, plus the account locale when the inquiry was sent by a
    /// signed-in portal user (anonymous inquirers have none).
    pub async fn get_inquiry_response_recipient(
        &self,
        inquiry_id: Uuid,
    ) -> Result<Option<(String, String, Option<String>)>, SqlxError> {
        sqlx::query_as::<_, (String, String, Option<String>)>(
            r#"
            SELECT i.email, i.name, u.locale::text
            FROM listing_inquiries i
            LEFT JOIN users u ON u.id = i.user_id
            WHERE i.id = $1
            "#,
        )
        .bind(inquiry_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// List the persisted conversation messages for an inquiry, oldest first.
    ///
    /// Ownership is expected to be verified by the caller (the detail handler
    /// resolves the inquiry via [`Self::get_inquiry_for_realtor`] first), so
    /// this query is scoped only by `inquiry_id`. Ordered by `created_at` so
    /// the realtor sees the thread in chronological order.
    pub async fn list_inquiry_messages(
        &self,
        inquiry_id: Uuid,
    ) -> Result<Vec<InquiryMessage>, SqlxError> {
        sqlx::query_as::<_, InquiryMessage>(
            "SELECT * FROM inquiry_messages WHERE inquiry_id = $1 ORDER BY created_at ASC",
        )
        .bind(inquiry_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Mark inquiry as read.
    pub async fn mark_inquiry_read(&self, id: Uuid) -> Result<(), SqlxError> {
        sqlx::query("UPDATE listing_inquiries SET status = 'read', read_at = NOW() WHERE id = $1 AND read_at IS NULL")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Mark an inquiry as read, scoped to the calling realtor.
    ///
    /// Returns `true` when the inquiry belongs to `realtor_id` (idempotent 204);
    /// `false` when not found or owned by another realtor (caller returns 404).
    pub async fn mark_inquiry_read_for_realtor(
        &self,
        id: Uuid,
        realtor_id: Uuid,
    ) -> Result<bool, SqlxError> {
        let owned: bool = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM listing_inquiries WHERE id = $1 AND realtor_id = $2)",
        )
        .bind(id)
        .bind(realtor_id)
        .fetch_one(&self.pool)
        .await?;

        if !owned {
            return Ok(false);
        }

        sqlx::query(
            "UPDATE listing_inquiries SET status = 'read', read_at = NOW() WHERE id = $1 AND read_at IS NULL",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(true)
    }

    /// Respond to inquiry.
    ///
    /// Enforces realtor ownership: the calling realtor (`realtor_id`) MUST own
    /// the inquiry (`listing_inquiries.realtor_id = realtor_id`). Returns
    /// `Ok(None)` when the inquiry does not exist or belongs to a different
    /// realtor so the route layer can return 404 (indistinguishable from
    /// "not found" to the caller — no information leakage).
    pub async fn respond_to_inquiry(
        &self,
        id: Uuid,
        realtor_id: Uuid,
        message: &str,
    ) -> Result<Option<InquiryMessage>, SqlxError> {
        let mut tx = self.pool.begin().await?;

        // Ownership check: only the realtor who owns the inquiry may respond.
        // This mirrors the pattern used by `get_inquiry_for_realtor` and
        // `mark_inquiry_read_for_realtor`. Without this guard, realtor B could
        // insert a message on realtor A's inquiry (IDOR).
        let owned: bool = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM listing_inquiries WHERE id = $1 AND realtor_id = $2)",
        )
        .bind(id)
        .bind(realtor_id)
        .fetch_one(&mut *tx)
        .await?;

        if !owned {
            // Rollback (nothing written yet) and signal caller to 404.
            tx.rollback().await?;
            return Ok(None);
        }

        // Create message
        let msg = sqlx::query_as::<_, InquiryMessage>(
            r#"
            INSERT INTO inquiry_messages (inquiry_id, sender_type, sender_id, message)
            VALUES ($1, 'realtor', $2, $3)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(realtor_id)
        .bind(message)
        .fetch_one(&mut *tx)
        .await?;

        // Update inquiry status (ownership already verified above)
        sqlx::query(
            "UPDATE listing_inquiries SET status = 'responded', responded_at = NOW() WHERE id = $1 AND realtor_id = $2",
        )
        .bind(id)
        .bind(realtor_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(Some(msg))
    }

    // ========================================================================
    // Buyer side of an inquiry thread
    // ========================================================================

    /// Get a single inquiry by id, scoped to the buyer who submitted it
    /// (`user_id`). Counterpart to [`Self::get_inquiry_for_realtor`]; another
    /// buyer's id returns `None` so the route layer can 404 without leaking.
    pub async fn get_inquiry_for_buyer(
        &self,
        inquiry_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<ListingInquiry>, SqlxError> {
        sqlx::query_as::<_, ListingInquiry>(
            "SELECT * FROM listing_inquiries WHERE id = $1 AND user_id = $2",
        )
        .bind(inquiry_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Add a buyer follow-up message to their own inquiry thread.
    ///
    /// Returns `Ok(None)` when the inquiry does not exist, belongs to another
    /// buyer, or is already closed. Re-opens the conversation for the realtor
    /// by moving a `responded`/`read` inquiry back to `new`, so it surfaces as
    /// unanswered in the realtor inbox.
    pub async fn buyer_reply_to_inquiry(
        &self,
        id: Uuid,
        user_id: Uuid,
        message: &str,
    ) -> Result<Option<InquiryMessage>, SqlxError> {
        let mut tx = self.pool.begin().await?;

        let open: bool = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM listing_inquiries
             WHERE id = $1 AND user_id = $2 AND status <> 'closed')",
        )
        .bind(id)
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;

        if !open {
            tx.rollback().await?;
            return Ok(None);
        }

        let msg = sqlx::query_as::<_, InquiryMessage>(
            r#"
            INSERT INTO inquiry_messages (inquiry_id, sender_type, sender_id, message)
            VALUES ($1, 'user', $2, $3)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(message)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query(
            "UPDATE listing_inquiries SET status = 'new', read_at = NULL
             WHERE id = $1 AND user_id = $2 AND status IN ('read', 'responded')",
        )
        .bind(id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(Some(msg))
    }

    /// Cancel a buyer's own viewing request (sets `status = 'closed'`).
    ///
    /// Only `inquiry_type = 'viewing'` rows owned by `user_id` qualify.
    /// Returns `true` when the row is (now or already) closed and `false`
    /// when there is no such viewing for this buyer, so the caller can 404.
    pub async fn cancel_viewing_for_buyer(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, SqlxError> {
        let updated = sqlx::query_scalar::<_, Uuid>(
            r#"
            UPDATE listing_inquiries SET status = 'closed'
            WHERE id = $1 AND user_id = $2 AND inquiry_type = 'viewing'
            RETURNING id
            "#,
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(updated.is_some())
    }

    /// Schedule a confirmed viewing for one of the realtor's own inquiries
    /// (Story 33.3). IDOR-safe: the `viewing_schedules` row is created only
    /// when `inquiry_id` resolves to an inquiry whose `realtor_id` matches
    /// `realtor_id`, mirroring the ownership guard used by the other
    /// realtor-scoped inquiry methods (`get_inquiry_for_realtor`,
    /// `respond_to_inquiry`). The attendee contact fields are copied from the
    /// originating inquiry so the schedule always reflects who actually
    /// requested the viewing.
    ///
    /// Returns `Ok(Some(..))` with the created schedule, or `Ok(None)` when
    /// there is no such inquiry for this realtor, so the caller can 404
    /// without leaking whether the inquiry exists for a different realtor.
    pub async fn schedule_viewing(
        &self,
        inquiry_id: Uuid,
        realtor_id: Uuid,
        data: ScheduleViewing,
    ) -> Result<Option<ViewingSchedule>, SqlxError> {
        sqlx::query_as::<_, ViewingSchedule>(
            r#"
            INSERT INTO viewing_schedules (
                inquiry_id, listing_id, realtor_id,
                attendee_name, attendee_email, attendee_phone,
                scheduled_at, duration_minutes, internal_notes
            )
            SELECT
                li.id, li.listing_id, li.realtor_id,
                li.name, li.email, li.phone,
                $3, COALESCE($4, 30), $5
            FROM listing_inquiries li
            WHERE li.id = $1 AND li.realtor_id = $2
            RETURNING *
            "#,
        )
        .bind(inquiry_id)
        .bind(realtor_id)
        .bind(data.scheduled_at)
        .bind(data.duration_minutes)
        .bind(&data.internal_notes)
        .fetch_optional(&self.pool)
        .await
    }
}
