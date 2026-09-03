//! GDPR data export repository (Epic 9, Story 9.3).

#![allow(clippy::type_complexity)]

use crate::models::data_export::{
    ActivityExport, AnnouncementExport, CreateDataExportRequest, DataExportReportSummary,
    DataExportRequest, DocumentExport, ExportFormat, ExportMetadata, FaultExport, MessageExport,
    OrganizationExport, ProfileExport, ResidencyExport, UserDataExport, VoteExport,
};
use crate::DbPool;
use chrono::{DateTime, Utc};
use sqlx::{Error as SqlxError, PgConnection};
use uuid::Uuid;

/// Repository for GDPR data export operations.
#[derive(Clone)]
pub struct DataExportRepository {
    pool: DbPool,
}

impl DataExportRepository {
    /// Create a new repository instance.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Create a new data export request.
    pub async fn create(
        &self,
        data: CreateDataExportRequest,
    ) -> Result<DataExportRequest, SqlxError> {
        let format_str = match data.format {
            ExportFormat::Json => "json",
            ExportFormat::Csv => "csv",
        };

        let categories = data
            .include_categories
            .map(|c| serde_json::to_value(c).unwrap_or_default());

        sqlx::query_as::<_, DataExportRequest>(
            r#"
            INSERT INTO data_export_requests (user_id, format, include_categories)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(data.user_id)
        .bind(format_str)
        .bind(categories)
        .fetch_one(&self.pool)
        .await
    }

    /// Get a data export request by ID.
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<DataExportRequest>, SqlxError> {
        sqlx::query_as::<_, DataExportRequest>(
            r#"
            SELECT * FROM data_export_requests WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Get all export requests for a user.
    pub async fn get_by_user_id(&self, user_id: Uuid) -> Result<Vec<DataExportRequest>, SqlxError> {
        sqlx::query_as::<_, DataExportRequest>(
            r#"
            SELECT * FROM data_export_requests
            WHERE user_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Get pending export requests for processing.
    pub async fn get_pending(&self, limit: i64) -> Result<Vec<DataExportRequest>, SqlxError> {
        sqlx::query_as::<_, DataExportRequest>(
            r#"
            SELECT * FROM data_export_requests
            WHERE status = 'pending'
            ORDER BY created_at ASC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    /// Aggregate counts + a bounded most-recent slice for the platform GDPR
    /// data-export compliance report (see [`DataExportReportSummary`]).
    ///
    /// The four counts are computed in one filtered aggregate so they stay
    /// mutually consistent. `from_date` / `to_date` bound `created_at` (either
    /// may be `None` to leave that side unbounded); `limit` bounds only the
    /// detail `entries`, never the counts.
    ///
    /// The aggregate and the detail slice are two statements, so a concurrent
    /// write landing between them would otherwise yield a summary whose counts
    /// disagree with its entries (issue #2924). Both reads run inside one
    /// `REPEATABLE READ` transaction: PostgreSQL freezes the transaction
    /// snapshot at the first statement, so the counts and the entries always
    /// observe the exact same set of rows regardless of concurrent commits.
    pub async fn report_summary(
        &self,
        from_date: Option<DateTime<Utc>>,
        to_date: Option<DateTime<Utc>>,
        limit: i64,
    ) -> Result<DataExportReportSummary, SqlxError> {
        // A read-only REPEATABLE READ transaction pins one snapshot for both
        // reads. `SET TRANSACTION` must precede any data statement, so it is
        // issued as the transaction's first command.
        let mut tx = self.pool.begin().await?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
            .execute(&mut *tx)
            .await?;

        // Both reads run against the *same* connection/transaction so they share
        // the frozen snapshot. The two statements are split into the seams below
        // (`count_summary` / `list_entries`) so the snapshot-consistency invariant
        // can be driven under concurrent contention in tests — see
        // `data_export_test.rs`.
        let (total_requests, pending_count, completed_count, downloaded_count) =
            Self::count_summary(&mut tx, from_date, to_date).await?;

        let entries = Self::list_entries(&mut tx, from_date, to_date, limit).await?;

        // Read-only transaction: commit is cheap and just releases the snapshot.
        tx.commit().await?;

        Ok(DataExportReportSummary {
            total_requests,
            pending_count,
            completed_count,
            downloaded_count,
            entries,
        })
    }

    /// Aggregate the four GDPR report counts for the given `created_at` window,
    /// on an arbitrary connection.
    ///
    /// This is the counts half of [`report_summary`]. It takes an explicit
    /// `&mut PgConnection` (rather than reading `&self.pool`) so it can be run
    /// on the same connection as [`list_entries`] inside one snapshot-pinned
    /// transaction — that shared connection is what makes the counts and the
    /// entries mutually consistent. Callers that pass two *different* pooled
    /// connections get the pre-#2924 behaviour where a concurrent commit can
    /// desync the two reads.
    pub(crate) async fn count_summary(
        conn: &mut PgConnection,
        from_date: Option<DateTime<Utc>>,
        to_date: Option<DateTime<Utc>>,
    ) -> Result<(i64, i64, i64, i64), SqlxError> {
        sqlx::query_as(
            r#"
            SELECT
                COUNT(*),
                COUNT(*) FILTER (WHERE status IN ('pending', 'processing')),
                COUNT(*) FILTER (WHERE status IN ('ready', 'downloaded', 'expired')),
                COUNT(*) FILTER (WHERE downloaded_at IS NOT NULL)
            FROM data_export_requests
            WHERE ($1::timestamptz IS NULL OR created_at >= $1)
              AND ($2::timestamptz IS NULL OR created_at <= $2)
            "#,
        )
        .bind(from_date)
        .bind(to_date)
        .fetch_one(&mut *conn)
        .await
    }

    /// Fetch the bounded most-recent detail slice for the given `created_at`
    /// window, on an arbitrary connection.
    ///
    /// This is the entries half of [`report_summary`]; see [`count_summary`]
    /// for why it takes an explicit connection.
    pub(crate) async fn list_entries(
        conn: &mut PgConnection,
        from_date: Option<DateTime<Utc>>,
        to_date: Option<DateTime<Utc>>,
        limit: i64,
    ) -> Result<Vec<DataExportRequest>, SqlxError> {
        sqlx::query_as::<_, DataExportRequest>(
            r#"
            SELECT * FROM data_export_requests
            WHERE ($1::timestamptz IS NULL OR created_at >= $1)
              AND ($2::timestamptz IS NULL OR created_at <= $2)
            ORDER BY created_at DESC
            LIMIT $3
            "#,
        )
        .bind(from_date)
        .bind(to_date)
        .bind(limit)
        .fetch_all(&mut *conn)
        .await
    }

    /// Mark an export as processing.
    pub async fn mark_processing(&self, id: Uuid) -> Result<(), SqlxError> {
        sqlx::query(
            r#"
            UPDATE data_export_requests
            SET status = 'processing', started_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Mark an export as ready with file information.
    pub async fn mark_ready(
        &self,
        id: Uuid,
        file_path: &str,
        file_size: i64,
        file_hash: &str,
    ) -> Result<Uuid, SqlxError> {
        let download_token = Uuid::new_v4();

        sqlx::query(
            r#"
            UPDATE data_export_requests
            SET status = 'ready',
                file_path = $2,
                file_size_bytes = $3,
                file_hash = $4,
                download_token = $5,
                completed_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(file_path)
        .bind(file_size)
        .bind(file_hash)
        .bind(download_token)
        .execute(&self.pool)
        .await?;

        Ok(download_token)
    }

    /// Mark an export as failed.
    pub async fn mark_failed(&self, id: Uuid, error_message: &str) -> Result<(), SqlxError> {
        sqlx::query(
            r#"
            UPDATE data_export_requests
            SET status = 'failed', error_message = $2, completed_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(error_message)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get an export request by download token.
    pub async fn get_by_token(&self, token: Uuid) -> Result<Option<DataExportRequest>, SqlxError> {
        sqlx::query_as::<_, DataExportRequest>(
            r#"
            SELECT * FROM data_export_requests
            WHERE download_token = $1 AND status = 'ready'
            "#,
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await
    }

    /// Mark export as downloaded.
    pub async fn mark_downloaded(&self, id: Uuid) -> Result<(), SqlxError> {
        sqlx::query(
            r#"
            UPDATE data_export_requests
            SET status = 'downloaded',
                downloaded_at = NOW(),
                download_count = download_count + 1
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Expire old export requests.
    pub async fn expire_old_requests(&self) -> Result<i64, SqlxError> {
        let result = sqlx::query(
            r#"
            UPDATE data_export_requests
            SET status = 'expired'
            WHERE status IN ('ready', 'downloaded')
              AND expires_at < NOW()
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as i64)
    }

    /// Check if user has a pending or processing export.
    pub async fn has_active_request(&self, user_id: Uuid) -> Result<bool, SqlxError> {
        let result: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM data_export_requests
            WHERE user_id = $1 AND status IN ('pending', 'processing')
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(result.0 > 0)
    }

    /// Collect all user data for export.
    pub async fn collect_user_data(
        &self,
        user_id: Uuid,
        categories: Option<Vec<String>>,
    ) -> Result<UserDataExport, SqlxError> {
        let include_all = categories.is_none();
        let cats = categories.unwrap_or_default();
        let include = |cat: &str| include_all || cats.contains(&cat.to_string());

        // Get user profile
        let profile = if include("profile") {
            self.get_profile_export(user_id).await?
        } else {
            None
        };

        // Get user email for metadata
        let user_email = profile
            .as_ref()
            .map(|p| p.email.clone())
            .unwrap_or_default();

        // Get organizations
        let organizations = if include("organizations") {
            Some(self.get_organization_exports(user_id).await?)
        } else {
            None
        };

        // Get residencies
        let residencies = if include("residencies") {
            Some(self.get_residency_exports(user_id).await?)
        } else {
            None
        };

        // Get activity logs
        let activity = if include("activity") {
            Some(self.get_activity_exports(user_id).await?)
        } else {
            None
        };

        // Get documents
        let documents = if include("documents") {
            Some(self.get_document_exports(user_id).await?)
        } else {
            None
        };

        // Get votes
        let votes = if include("votes") {
            Some(self.get_vote_exports(user_id).await?)
        } else {
            None
        };

        // Get faults
        let faults = if include("faults") {
            Some(self.get_fault_exports(user_id).await?)
        } else {
            None
        };

        // Get messages
        let messages = if include("messages") {
            Some(self.get_message_exports(user_id).await?)
        } else {
            None
        };

        // Get announcements
        let announcements = if include("announcements") {
            Some(self.get_announcement_exports(user_id).await?)
        } else {
            None
        };

        let categories_included: Vec<String> = [
            ("profile", profile.is_some()),
            ("organizations", organizations.is_some()),
            ("residencies", residencies.is_some()),
            ("activity", activity.is_some()),
            ("documents", documents.is_some()),
            ("votes", votes.is_some()),
            ("faults", faults.is_some()),
            ("messages", messages.is_some()),
            ("announcements", announcements.is_some()),
        ]
        .iter()
        .filter(|(_, included)| *included)
        .map(|(name, _)| name.to_string())
        .collect();

        Ok(UserDataExport {
            metadata: ExportMetadata {
                generated_at: Utc::now(),
                format: "json".to_string(),
                categories_included,
                user_id,
                user_email,
            },
            profile,
            organizations,
            residencies,
            activity,
            documents,
            votes,
            faults,
            messages,
            announcements,
        })
    }

    async fn get_profile_export(&self, user_id: Uuid) -> Result<Option<ProfileExport>, SqlxError> {
        let row: Option<(
            Uuid,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            String,
            String,
            bool,
            bool,
            chrono::DateTime<Utc>,
            chrono::DateTime<Utc>,
        )> = sqlx::query_as(
            r#"
            -- Issue #1008: `users` has a single `name` column (no first/last),
            -- `phone` (not phone_number), `locale` (not language), and
            -- `email_verified_at` (not a bool email_verified). Map them onto the
            -- ProfileExport shape: full name in first_name, last_name NULL.
            SELECT id, email, name AS first_name, NULL::text AS last_name, phone AS phone_number,
                   COALESCE(locale, 'en') as language,
                   profile_visibility, show_contact_info,
                   (email_verified_at IS NOT NULL) AS email_verified,
                   created_at, updated_at
            FROM users WHERE id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(
            |(
                id,
                email,
                first_name,
                last_name,
                phone_number,
                language,
                profile_visibility,
                show_contact_info,
                email_verified,
                created_at,
                updated_at,
            )| {
                ProfileExport {
                    id,
                    email,
                    first_name,
                    last_name,
                    phone_number,
                    language,
                    profile_visibility,
                    show_contact_info,
                    email_verified,
                    created_at,
                    updated_at,
                }
            },
        ))
    }

    async fn get_organization_exports(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<OrganizationExport>, SqlxError> {
        let rows: Vec<(Uuid, String, String, chrono::DateTime<Utc>)> = sqlx::query_as(
            r#"
            SELECT o.id, o.name, om.role, om.created_at
            FROM organization_members om
            JOIN organizations o ON o.id = om.org_id
            WHERE om.user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(organization_id, organization_name, role, joined_at)| OrganizationExport {
                    organization_id,
                    organization_name,
                    role,
                    joined_at,
                },
            )
            .collect())
    }

    async fn get_residency_exports(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<ResidencyExport>, SqlxError> {
        let rows: Vec<(
            Uuid,
            String,
            String,
            String,
            chrono::DateTime<Utc>,
            Option<chrono::DateTime<Utc>>,
        )> = sqlx::query_as(
            r#"
            SELECT u.id, b.name, u.unit_number, ur.resident_type::text,
                   ur.start_date, ur.end_date
            FROM unit_residents ur
            JOIN units u ON u.id = ur.unit_id
            JOIN buildings b ON b.id = u.building_id
            WHERE ur.user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(unit_id, building_name, unit_number, resident_type, start_date, end_date)| {
                    ResidencyExport {
                        unit_id,
                        building_name,
                        unit_number,
                        resident_type,
                        start_date,
                        end_date,
                    }
                },
            )
            .collect())
    }

    async fn get_activity_exports(&self, user_id: Uuid) -> Result<Vec<ActivityExport>, SqlxError> {
        let rows: Vec<(
            String,
            Option<String>,
            Option<serde_json::Value>,
            Option<String>,
            chrono::DateTime<Utc>,
        )> = sqlx::query_as(
            r#"
            SELECT action::text, resource_type, details, ip_address, created_at
            FROM audit_logs
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT 1000
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(action, resource_type, details, ip_address, created_at)| ActivityExport {
                    action,
                    resource_type,
                    details,
                    ip_address,
                    created_at,
                },
            )
            .collect())
    }

    async fn get_document_exports(&self, user_id: Uuid) -> Result<Vec<DocumentExport>, SqlxError> {
        let rows: Vec<(Uuid, String, String, String, i64, chrono::DateTime<Utc>)> = sqlx::query_as(
            r#"
            SELECT id, title, category::text, file_type, file_size, created_at
            FROM documents
            WHERE uploaded_by = $1
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(id, title, category, file_type, file_size, created_at)| DocumentExport {
                    id,
                    title,
                    category,
                    file_type,
                    file_size,
                    created_at,
                },
            )
            .collect())
    }

    async fn get_vote_exports(&self, user_id: Uuid) -> Result<Vec<VoteExport>, SqlxError> {
        let rows: Vec<(Uuid, String, String, String, chrono::DateTime<Utc>)> = sqlx::query_as(
            r#"
            SELECT v.id, v.title, vq.question_text, vr.response_value, vr.created_at
            FROM vote_responses vr
            JOIN vote_questions vq ON vq.id = vr.question_id
            JOIN votes v ON v.id = vq.vote_id
            WHERE vr.user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(vote_id, vote_title, question, response, voted_at)| VoteExport {
                    vote_id,
                    vote_title,
                    question,
                    response,
                    voted_at,
                },
            )
            .collect())
    }

    async fn get_fault_exports(&self, user_id: Uuid) -> Result<Vec<FaultExport>, SqlxError> {
        let rows: Vec<(Uuid, String, String, String, chrono::DateTime<Utc>)> = sqlx::query_as(
            r#"
            SELECT id, title, description, status::text, created_at
            FROM faults
            WHERE reported_by = $1
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(id, title, description, status, created_at)| FaultExport {
                id,
                title,
                description,
                status,
                created_at,
            })
            .collect())
    }

    async fn get_message_exports(&self, user_id: Uuid) -> Result<Vec<MessageExport>, SqlxError> {
        let rows: Vec<(Uuid, Uuid, String, chrono::DateTime<Utc>)> = sqlx::query_as(
            r#"
            SELECT id, thread_id, content, created_at
            FROM messages
            WHERE sender_id = $1
            ORDER BY created_at DESC
            LIMIT 1000
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(id, thread_id, content, sent_at)| MessageExport {
                id,
                thread_id,
                content,
                sent_at,
            })
            .collect())
    }

    async fn get_announcement_exports(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<AnnouncementExport>, SqlxError> {
        let rows: Vec<(Uuid, String, String, chrono::DateTime<Utc>)> = sqlx::query_as(
            r#"
            SELECT id, title, content, created_at
            FROM announcements
            WHERE author_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(id, title, content, created_at)| AnnouncementExport {
                id,
                title,
                content,
                created_at,
            })
            .collect())
    }
}
