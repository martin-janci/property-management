//! User repository (Epic 1, Story 1.1).

use crate::models::user::{
    CreateUser, EmailVerificationToken, NeighborRow, NeighborView, PrincipalKind, PrivacySettings,
    ProfileVisibility, UpdatePrivacySettings, UpdateUser, User,
};
use crate::DbPool;
use chrono::{Duration, Utc};
use sqlx::Error as SqlxError;
use uuid::Uuid;

/// Repository for user operations.
#[derive(Clone)]
pub struct UserRepository {
    pool: DbPool,
}

impl UserRepository {
    /// Create a new UserRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Create a new user.
    pub async fn create(&self, data: CreateUser) -> Result<User, SqlxError> {
        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (email, password_hash, name, phone, locale, status)
            VALUES ($1, $2, $3, $4, $5, 'pending')
            RETURNING *
            "#,
        )
        .bind(&data.email)
        .bind(&data.password_hash)
        .bind(&data.name)
        .bind(&data.phone)
        .bind(data.locale.as_str())
        .fetch_one(&self.pool)
        .await?;

        Ok(user)
    }

    /// Find user by ID.
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, SqlxError> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT * FROM users WHERE id = $1 AND status != 'deleted'
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    /// Find user by email.
    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>, SqlxError> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT * FROM users WHERE LOWER(email) = LOWER($1) AND status != 'deleted'
            "#,
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    /// Phase 2: find a user by email, filtered by `principal_kind`.
    /// Used by the reality-server flow to look up `public` (portal) users in
    /// the unified `users` table.
    pub async fn find_by_email_and_kind(
        &self,
        email: &str,
        kind: PrincipalKind,
    ) -> Result<Option<User>, SqlxError> {
        sqlx::query_as::<_, User>(
            r#"
            SELECT * FROM users
             WHERE LOWER(email) = LOWER($1)
               AND principal_kind = $2
               AND status != 'deleted'
            "#,
        )
        .bind(email)
        .bind(kind.as_str())
        .fetch_optional(&self.pool)
        .await
    }

    /// Phase 2: find a user by id, filtered by `principal_kind`.
    pub async fn find_by_id_and_kind(
        &self,
        id: Uuid,
        kind: PrincipalKind,
    ) -> Result<Option<User>, SqlxError> {
        sqlx::query_as::<_, User>(
            r#"
            SELECT * FROM users
             WHERE id = $1
               AND principal_kind = $2
               AND status != 'deleted'
            "#,
        )
        .bind(id)
        .bind(kind.as_str())
        .fetch_optional(&self.pool)
        .await
    }

    /// Check if email exists (including deleted accounts within 30 days).
    pub async fn email_exists(&self, email: &str) -> Result<bool, SqlxError> {
        let result = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) FROM users
            WHERE LOWER(email) = LOWER($1)
            AND (status != 'deleted' OR deleted_at > NOW() - INTERVAL '30 days')
            "#,
        )
        .bind(email)
        .fetch_one(&self.pool)
        .await?;

        Ok(result > 0)
    }

    /// Update user.
    pub async fn update(&self, id: Uuid, data: UpdateUser) -> Result<Option<User>, SqlxError> {
        let mut query = String::from("UPDATE users SET updated_at = NOW()");
        let mut param_count = 1;

        if data.name.is_some() {
            param_count += 1;
            query.push_str(&format!(", name = ${}", param_count));
        }
        if data.phone.is_some() {
            param_count += 1;
            query.push_str(&format!(", phone = ${}", param_count));
        }
        if data.locale.is_some() {
            param_count += 1;
            query.push_str(&format!(", locale = ${}", param_count));
        }
        if data.avatar_url.is_some() {
            param_count += 1;
            query.push_str(&format!(", profile_image_url = ${}", param_count));
        }

        query.push_str(" WHERE id = $1 AND status != 'deleted' RETURNING *");

        let mut q = sqlx::query_as::<_, User>(sqlx::AssertSqlSafe(query)).bind(id);

        if let Some(name) = &data.name {
            q = q.bind(name);
        }
        if let Some(phone) = &data.phone {
            q = q.bind(phone);
        }
        if let Some(locale) = &data.locale {
            q = q.bind(locale.as_str());
        }
        if let Some(avatar_url) = &data.avatar_url {
            q = q.bind(avatar_url);
        }

        let user = q.fetch_optional(&self.pool).await?;
        Ok(user)
    }

    /// Verify user email.
    pub async fn verify_email(&self, id: Uuid) -> Result<Option<User>, SqlxError> {
        let user = sqlx::query_as::<_, User>(
            r#"
            UPDATE users
            SET email_verified_at = NOW(), status = 'active', updated_at = NOW()
            WHERE id = $1 AND status = 'pending'
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    /// Suspend user.
    pub async fn suspend(&self, id: Uuid, by: Uuid) -> Result<Option<User>, SqlxError> {
        let user = sqlx::query_as::<_, User>(
            r#"
            UPDATE users
            SET status = 'suspended', suspended_at = NOW(), suspended_by = $2, updated_at = NOW()
            WHERE id = $1 AND status = 'active'
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(by)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    /// Reactivate suspended user.
    pub async fn reactivate(&self, id: Uuid) -> Result<Option<User>, SqlxError> {
        let user = sqlx::query_as::<_, User>(
            r#"
            UPDATE users
            SET status = 'active', suspended_at = NULL, suspended_by = NULL, updated_at = NOW()
            WHERE id = $1 AND status = 'suspended'
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    /// Soft delete user.
    pub async fn soft_delete(&self, id: Uuid, by: Uuid) -> Result<Option<User>, SqlxError> {
        let user = sqlx::query_as::<_, User>(
            r#"
            UPDATE users
            SET status = 'deleted', deleted_at = NOW(), deleted_by = $2, updated_at = NOW()
            WHERE id = $1 AND status != 'deleted'
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(by)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    /// Update password hash.
    pub async fn update_password(&self, id: Uuid, password_hash: &str) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            r#"
            UPDATE users SET password_hash = $2, updated_at = NOW()
            WHERE id = $1 AND status != 'deleted'
            "#,
        )
        .bind(id)
        .bind(password_hash)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    // ==================== Email Verification Tokens ====================

    /// Create email verification token.
    pub async fn create_verification_token(
        &self,
        user_id: Uuid,
        token_hash: &str,
    ) -> Result<EmailVerificationToken, SqlxError> {
        // Token valid for 24 hours
        let expires_at = Utc::now() + Duration::hours(24);

        let token = sqlx::query_as::<_, EmailVerificationToken>(
            r#"
            INSERT INTO email_verification_tokens (user_id, token_hash, expires_at)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(token_hash)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(token)
    }

    /// Find verification token by hash.
    pub async fn find_verification_token(
        &self,
        token_hash: &str,
    ) -> Result<Option<EmailVerificationToken>, SqlxError> {
        let token = sqlx::query_as::<_, EmailVerificationToken>(
            r#"
            SELECT * FROM email_verification_tokens
            WHERE token_hash = $1 AND used_at IS NULL
            "#,
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(token)
    }

    /// Mark verification token as used.
    pub async fn use_verification_token(&self, id: Uuid) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            r#"
            UPDATE email_verification_tokens SET used_at = NOW()
            WHERE id = $1 AND used_at IS NULL
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete expired verification tokens.
    pub async fn cleanup_expired_tokens(&self) -> Result<u64, SqlxError> {
        let result = sqlx::query(
            r#"
            DELETE FROM email_verification_tokens
            WHERE expires_at < NOW() OR used_at IS NOT NULL
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Invalidate all verification tokens for a user.
    pub async fn invalidate_user_tokens(&self, user_id: Uuid) -> Result<u64, SqlxError> {
        let result = sqlx::query(
            r#"
            UPDATE email_verification_tokens SET used_at = NOW()
            WHERE user_id = $1 AND used_at IS NULL
            "#,
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    // ==================== Admin Operations ====================

    /// List users with pagination and optional filters.
    ///
    /// The COUNT and the data SELECT are issued as two separate queries with
    /// independent placeholder numbering schemes (the count query has no
    /// LIMIT/OFFSET, the data query binds LIMIT/OFFSET first). Earlier
    /// revisions of this function used a shared placeholder counter across
    /// both queries which caused the WHERE clause to reference the wrong
    /// bind slot — admin user search could silently return rows for the
    /// wrong filter (SEC-004).
    pub async fn list_users(
        &self,
        offset: i64,
        limit: i64,
        status_filter: Option<&str>,
        search: Option<&str>,
    ) -> Result<(Vec<User>, i64), SqlxError> {
        // ---- COUNT query: placeholders start at $1 ----
        let mut count_conditions: Vec<String> = vec!["1=1".to_string()];
        let mut count_idx = 0usize;
        if status_filter.is_some() {
            count_idx += 1;
            count_conditions.push(format!("status = ${}", count_idx));
        }
        if search.is_some() {
            count_idx += 1;
            count_conditions.push(format!(
                "(LOWER(email) LIKE '%' || LOWER(${0}::text) || '%' OR LOWER(name) LIKE '%' || LOWER(${0}::text) || '%')",
                count_idx
            ));
        }
        let count_where = count_conditions.join(" AND ");
        let count_query = format!("SELECT COUNT(*) FROM users WHERE {}", count_where);

        let mut count_q = sqlx::query_scalar::<_, i64>(sqlx::AssertSqlSafe(count_query));
        if let Some(status) = status_filter {
            count_q = count_q.bind(status);
        }
        if let Some(s) = search {
            count_q = count_q.bind(s);
        }
        let total = count_q.fetch_one(&self.pool).await?;

        // ---- DATA query: $1=limit, $2=offset, then filter placeholders ----
        let mut data_conditions: Vec<String> = vec!["1=1".to_string()];
        let mut data_idx = 2usize; // $1 and $2 are reserved for limit/offset
        if status_filter.is_some() {
            data_idx += 1;
            data_conditions.push(format!("status = ${}", data_idx));
        }
        if search.is_some() {
            data_idx += 1;
            data_conditions.push(format!(
                "(LOWER(email) LIKE '%' || LOWER(${0}::text) || '%' OR LOWER(name) LIKE '%' || LOWER(${0}::text) || '%')",
                data_idx
            ));
        }
        let data_where = data_conditions.join(" AND ");
        let data_query = format!(
            "SELECT * FROM users WHERE {} ORDER BY created_at DESC LIMIT $1 OFFSET $2",
            data_where
        );

        let mut data_q = sqlx::query_as::<_, User>(sqlx::AssertSqlSafe(data_query))
            .bind(limit)
            .bind(offset);
        if let Some(status) = status_filter {
            data_q = data_q.bind(status);
        }
        if let Some(s) = search {
            data_q = data_q.bind(s);
        }
        let users = data_q.fetch_all(&self.pool).await?;

        Ok((users, total))
    }

    /// Find user by ID including deleted users (for admin).
    pub async fn find_by_id_admin(&self, id: Uuid) -> Result<Option<User>, SqlxError> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT * FROM users WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    // ==================== Privacy & Neighbor Operations (Story 6.6) ====================

    /// Get user's privacy settings.
    pub async fn get_privacy_settings(&self, user_id: Uuid) -> Result<PrivacySettings, SqlxError> {
        let (visibility, show_contact): (String, bool) = sqlx::query_as(
            r#"
            SELECT profile_visibility, show_contact_info
            FROM users WHERE id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(PrivacySettings {
            profile_visibility: ProfileVisibility::parse(&visibility),
            show_contact_info: show_contact,
        })
    }

    /// Update user's privacy settings.
    pub async fn update_privacy_settings(
        &self,
        user_id: Uuid,
        data: UpdatePrivacySettings,
    ) -> Result<PrivacySettings, SqlxError> {
        let mut updates = vec!["updated_at = NOW()".to_string()];
        let mut param_idx = 1;

        if data.profile_visibility.is_some() {
            param_idx += 1;
            updates.push(format!("profile_visibility = ${}", param_idx));
        }
        if data.show_contact_info.is_some() {
            param_idx += 1;
            updates.push(format!("show_contact_info = ${}", param_idx));
        }

        let query = format!(
            "UPDATE users SET {} WHERE id = $1 RETURNING profile_visibility, show_contact_info",
            updates.join(", ")
        );

        let mut q = sqlx::query_as::<_, (String, bool)>(sqlx::AssertSqlSafe(query)).bind(user_id);

        if let Some(ref visibility) = data.profile_visibility {
            q = q.bind(visibility.as_str());
        }
        if let Some(show_contact) = data.show_contact_info {
            q = q.bind(show_contact);
        }

        let (visibility, show_contact) = q.fetch_one(&self.pool).await?;

        Ok(PrivacySettings {
            profile_visibility: ProfileVisibility::parse(&visibility),
            show_contact_info: show_contact,
        })
    }

    /// Get neighbors for a user (residents in the same building but different units).
    /// Respects privacy settings of each neighbor.
    pub async fn get_neighbors(
        &self,
        user_id: Uuid,
        building_id: Uuid,
    ) -> Result<Vec<NeighborView>, SqlxError> {
        // First find the user's unit(s) in this building
        let user_unit_ids: Vec<Uuid> = sqlx::query_scalar(
            r#"
            SELECT ur.unit_id
            FROM unit_residents ur
            JOIN units u ON u.id = ur.unit_id
            WHERE ur.user_id = $1
              AND u.building_id = $2
              AND ur.end_date IS NULL
            "#,
        )
        .bind(user_id)
        .bind(building_id)
        .fetch_all(&self.pool)
        .await?;

        if user_unit_ids.is_empty() {
            return Ok(vec![]);
        }

        // Get neighbors: same building, different unit, active residents
        let rows = sqlx::query_as::<_, NeighborRow>(
            r#"
            SELECT
                u.id as user_id,
                u.name as user_name,
                u.email as user_email,
                u.phone as user_phone,
                u.profile_visibility,
                u.show_contact_info,
                un.id as unit_id,
                un.designation AS unit_number,
                b.name as building_name,
                ur.resident_type
            FROM unit_residents ur
            JOIN users u ON u.id = ur.user_id
            JOIN units un ON un.id = ur.unit_id
            JOIN buildings b ON b.id = un.building_id
            WHERE un.building_id = $1
              AND ur.user_id != $2
              AND NOT (ur.unit_id = ANY($3))
              AND ur.end_date IS NULL
              AND u.status = 'active'
            ORDER BY un.designation, u.name
            "#,
        )
        .bind(building_id)
        .bind(user_id)
        .bind(&user_unit_ids)
        .fetch_all(&self.pool)
        .await?;

        // Transform to privacy-aware views
        let neighbors = rows
            .into_iter()
            .map(|row| row.into_neighbor_view())
            .collect();

        Ok(neighbors)
    }

    /// Count neighbors in a building for a user.
    pub async fn count_neighbors(
        &self,
        user_id: Uuid,
        building_id: Uuid,
    ) -> Result<i64, SqlxError> {
        // First find the user's unit(s) in this building
        let user_unit_ids: Vec<Uuid> = sqlx::query_scalar(
            r#"
            SELECT ur.unit_id
            FROM unit_residents ur
            JOIN units u ON u.id = ur.unit_id
            WHERE ur.user_id = $1
              AND u.building_id = $2
              AND ur.end_date IS NULL
            "#,
        )
        .bind(user_id)
        .bind(building_id)
        .fetch_all(&self.pool)
        .await?;

        if user_unit_ids.is_empty() {
            return Ok(0);
        }

        let (count,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(DISTINCT ur.user_id)
            FROM unit_residents ur
            JOIN users u ON u.id = ur.user_id
            JOIN units un ON un.id = ur.unit_id
            WHERE un.building_id = $1
              AND ur.user_id != $2
              AND NOT (ur.unit_id = ANY($3))
              AND ur.end_date IS NULL
              AND u.status = 'active'
            "#,
        )
        .bind(building_id)
        .bind(user_id)
        .bind(&user_unit_ids)
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }

    // ==================== GDPR Deletion Operations (Story 9.4) ====================

    /// Schedule user account for deletion (GDPR Article 17).
    /// Sets a 30-day grace period before actual deletion.
    pub async fn schedule_deletion(
        &self,
        user_id: Uuid,
        scheduled_for: chrono::DateTime<Utc>,
    ) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            r#"
            UPDATE users
            SET scheduled_deletion_at = $2, updated_at = NOW()
            WHERE id = $1 AND status != 'deleted'
            "#,
        )
        .bind(user_id)
        .bind(scheduled_for)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Cancel a scheduled deletion request.
    pub async fn cancel_scheduled_deletion(&self, user_id: Uuid) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            r#"
            UPDATE users
            SET scheduled_deletion_at = NULL, updated_at = NOW()
            WHERE id = $1 AND scheduled_deletion_at IS NOT NULL
            "#,
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}
