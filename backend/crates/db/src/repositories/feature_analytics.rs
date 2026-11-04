//! Feature analytics repository (Epic 109, Story 109.4).
//!
//! Repository for feature usage analytics, descriptors, packages, and user type access.

use crate::models::feature_analytics::{
    FeatureAccessState, FeatureDescriptor, FeatureEventType, FeaturePackage, FeaturePackageFeature,
    FeaturePackageItem, FeatureStatsByUserType, FeatureUsageEvent, FeatureUsageStats,
    OrganizationFeaturePackage, UserFeaturePreference, UserTypeFeatureAccess,
};
use crate::DbPool;
use chrono::{DateTime, Duration, Utc};
use sqlx::Error as SqlxError;
use uuid::Uuid;

/// Repository for feature analytics operations.
#[derive(Clone)]
pub struct FeatureAnalyticsRepository {
    pub pool: DbPool,
}

impl FeatureAnalyticsRepository {
    /// Create a new FeatureAnalyticsRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // ==================== Feature Descriptors ====================

    /// Get descriptor for a feature flag.
    pub async fn get_descriptor(
        &self,
        feature_flag_id: Uuid,
    ) -> Result<Option<FeatureDescriptor>, SqlxError> {
        sqlx::query_as::<_, FeatureDescriptor>(
            r#"
            SELECT id, feature_flag_id, display_name, short_description, long_description,
                   icon, badge_text, help_url, category, sort_order, is_premium,
                   created_at, updated_at
            FROM feature_descriptors
            WHERE feature_flag_id = $1
            "#,
        )
        .bind(feature_flag_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Upsert feature descriptor.
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_descriptor(
        &self,
        feature_flag_id: Uuid,
        display_name: &str,
        short_description: Option<&str>,
        long_description: Option<&str>,
        icon: Option<&str>,
        badge_text: Option<&str>,
        help_url: Option<&str>,
        category: Option<&str>,
        sort_order: i32,
        is_premium: bool,
    ) -> Result<FeatureDescriptor, SqlxError> {
        sqlx::query_as::<_, FeatureDescriptor>(
            r#"
            INSERT INTO feature_descriptors (
                feature_flag_id, display_name, short_description, long_description,
                icon, badge_text, help_url, category, sort_order, is_premium
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (feature_flag_id)
            DO UPDATE SET
                display_name = EXCLUDED.display_name,
                short_description = EXCLUDED.short_description,
                long_description = EXCLUDED.long_description,
                icon = EXCLUDED.icon,
                badge_text = EXCLUDED.badge_text,
                help_url = EXCLUDED.help_url,
                category = EXCLUDED.category,
                sort_order = EXCLUDED.sort_order,
                is_premium = EXCLUDED.is_premium,
                updated_at = NOW()
            RETURNING id, feature_flag_id, display_name, short_description, long_description,
                      icon, badge_text, help_url, category, sort_order, is_premium,
                      created_at, updated_at
            "#,
        )
        .bind(feature_flag_id)
        .bind(display_name)
        .bind(short_description)
        .bind(long_description)
        .bind(icon)
        .bind(badge_text)
        .bind(help_url)
        .bind(category)
        .bind(sort_order)
        .bind(is_premium)
        .fetch_one(&self.pool)
        .await
    }

    /// Delete feature descriptor.
    pub async fn delete_descriptor(&self, feature_flag_id: Uuid) -> Result<bool, SqlxError> {
        let result = sqlx::query("DELETE FROM feature_descriptors WHERE feature_flag_id = $1")
            .bind(feature_flag_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    // ==================== User Type Feature Access ====================

    /// Get access state for a user type.
    pub async fn get_user_type_access(
        &self,
        feature_flag_id: Uuid,
        user_type: &str,
    ) -> Result<Option<UserTypeFeatureAccess>, SqlxError> {
        sqlx::query_as::<_, UserTypeFeatureAccess>(
            r#"
            SELECT id, feature_flag_id, user_type, access_state, default_enabled,
                   created_at, updated_at
            FROM user_type_feature_access
            WHERE feature_flag_id = $1 AND user_type = $2
            "#,
        )
        .bind(feature_flag_id)
        .bind(user_type)
        .fetch_optional(&self.pool)
        .await
    }

    /// Get all access entries for a feature.
    pub async fn list_feature_access(
        &self,
        feature_flag_id: Uuid,
    ) -> Result<Vec<UserTypeFeatureAccess>, SqlxError> {
        sqlx::query_as::<_, UserTypeFeatureAccess>(
            r#"
            SELECT id, feature_flag_id, user_type, access_state, default_enabled,
                   created_at, updated_at
            FROM user_type_feature_access
            WHERE feature_flag_id = $1
            ORDER BY user_type
            "#,
        )
        .bind(feature_flag_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Get all features accessible to a user type.
    pub async fn list_user_type_features(
        &self,
        user_type: &str,
    ) -> Result<Vec<UserTypeFeatureAccess>, SqlxError> {
        sqlx::query_as::<_, UserTypeFeatureAccess>(
            r#"
            SELECT id, feature_flag_id, user_type, access_state, default_enabled,
                   created_at, updated_at
            FROM user_type_feature_access
            WHERE user_type = $1 AND access_state != 'excluded'
            "#,
        )
        .bind(user_type)
        .fetch_all(&self.pool)
        .await
    }

    /// Set user type feature access.
    pub async fn set_user_type_access(
        &self,
        feature_flag_id: Uuid,
        user_type: &str,
        access_state: FeatureAccessState,
        default_enabled: bool,
    ) -> Result<UserTypeFeatureAccess, SqlxError> {
        sqlx::query_as::<_, UserTypeFeatureAccess>(
            r#"
            INSERT INTO user_type_feature_access (feature_flag_id, user_type, access_state, default_enabled)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (feature_flag_id, user_type)
            DO UPDATE SET
                access_state = EXCLUDED.access_state,
                default_enabled = EXCLUDED.default_enabled,
                updated_at = NOW()
            RETURNING id, feature_flag_id, user_type, access_state, default_enabled,
                      created_at, updated_at
            "#,
        )
        .bind(feature_flag_id)
        .bind(user_type)
        .bind(&access_state)
        .bind(default_enabled)
        .fetch_one(&self.pool)
        .await
    }

    // ==================== Feature Packages ====================

    /// Create a new feature package.
    pub async fn create_package(
        &self,
        name: &str,
        slug: &str,
        description: Option<&str>,
        price_monthly_cents: Option<i32>,
        price_yearly_cents: Option<i32>,
    ) -> Result<FeaturePackage, SqlxError> {
        sqlx::query_as::<_, FeaturePackage>(
            r#"
            INSERT INTO feature_packages (name, slug, description, price_monthly_cents, price_yearly_cents)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, name, slug, description, is_active, price_monthly_cents,
                      price_yearly_cents, created_at, updated_at
            "#,
        )
        .bind(name)
        .bind(slug)
        .bind(description)
        .bind(price_monthly_cents)
        .bind(price_yearly_cents)
        .fetch_one(&self.pool)
        .await
    }

    /// Get a package by ID.
    pub async fn get_package(&self, id: Uuid) -> Result<Option<FeaturePackage>, SqlxError> {
        sqlx::query_as::<_, FeaturePackage>(
            r#"
            SELECT id, name, slug, description, is_active, price_monthly_cents,
                   price_yearly_cents, created_at, updated_at
            FROM feature_packages
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Get a package by slug.
    pub async fn get_package_by_slug(
        &self,
        slug: &str,
    ) -> Result<Option<FeaturePackage>, SqlxError> {
        sqlx::query_as::<_, FeaturePackage>(
            r#"
            SELECT id, name, slug, description, is_active, price_monthly_cents,
                   price_yearly_cents, created_at, updated_at
            FROM feature_packages
            WHERE slug = $1
            "#,
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await
    }

    /// List all active packages.
    pub async fn list_active_packages(&self) -> Result<Vec<FeaturePackage>, SqlxError> {
        sqlx::query_as::<_, FeaturePackage>(
            r#"
            SELECT id, name, slug, description, is_active, price_monthly_cents,
                   price_yearly_cents, created_at, updated_at
            FROM feature_packages
            WHERE is_active = true
            ORDER BY name
            "#,
        )
        .fetch_all(&self.pool)
        .await
    }

    /// List all packages.
    pub async fn list_all_packages(&self) -> Result<Vec<FeaturePackage>, SqlxError> {
        sqlx::query_as::<_, FeaturePackage>(
            r#"
            SELECT id, name, slug, description, is_active, price_monthly_cents,
                   price_yearly_cents, created_at, updated_at
            FROM feature_packages
            ORDER BY name
            "#,
        )
        .fetch_all(&self.pool)
        .await
    }

    /// Update a package.
    pub async fn update_package(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        is_active: Option<bool>,
        price_monthly_cents: Option<i32>,
        price_yearly_cents: Option<i32>,
    ) -> Result<Option<FeaturePackage>, SqlxError> {
        sqlx::query_as::<_, FeaturePackage>(
            r#"
            UPDATE feature_packages
            SET name = COALESCE($2, name),
                description = COALESCE($3, description),
                is_active = COALESCE($4, is_active),
                price_monthly_cents = COALESCE($5, price_monthly_cents),
                price_yearly_cents = COALESCE($6, price_yearly_cents),
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, name, slug, description, is_active, price_monthly_cents,
                      price_yearly_cents, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(description)
        .bind(is_active)
        .bind(price_monthly_cents)
        .bind(price_yearly_cents)
        .fetch_optional(&self.pool)
        .await
    }

    /// Add a feature to a package.
    pub async fn add_feature_to_package(
        &self,
        package_id: Uuid,
        feature_flag_id: Uuid,
    ) -> Result<FeaturePackageItem, SqlxError> {
        sqlx::query_as::<_, FeaturePackageItem>(
            r#"
            INSERT INTO feature_package_items (package_id, feature_flag_id)
            VALUES ($1, $2)
            ON CONFLICT (package_id, feature_flag_id) DO NOTHING
            RETURNING id, package_id, feature_flag_id, created_at
            "#,
        )
        .bind(package_id)
        .bind(feature_flag_id)
        .fetch_one(&self.pool)
        .await
    }

    /// Remove a feature from a package.
    pub async fn remove_feature_from_package(
        &self,
        package_id: Uuid,
        feature_flag_id: Uuid,
    ) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            "DELETE FROM feature_package_items WHERE package_id = $1 AND feature_flag_id = $2",
        )
        .bind(package_id)
        .bind(feature_flag_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Get features in a package.
    pub async fn get_package_features(
        &self,
        package_id: Uuid,
    ) -> Result<Vec<FeaturePackageFeature>, SqlxError> {
        sqlx::query_as::<_, FeaturePackageFeature>(
            r#"
            SELECT ff.id as feature_flag_id, ff.key, ff.name, ff.description
            FROM feature_package_items fpi
            JOIN feature_flags ff ON ff.id = fpi.feature_flag_id
            WHERE fpi.package_id = $1
            ORDER BY ff.key
            "#,
        )
        .bind(package_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Get packages containing a feature.
    pub async fn get_packages_with_feature(
        &self,
        feature_flag_id: Uuid,
    ) -> Result<Vec<FeaturePackage>, SqlxError> {
        sqlx::query_as::<_, FeaturePackage>(
            r#"
            SELECT fp.id, fp.name, fp.slug, fp.description, fp.is_active,
                   fp.price_monthly_cents, fp.price_yearly_cents, fp.created_at, fp.updated_at
            FROM feature_packages fp
            JOIN feature_package_items fpi ON fpi.package_id = fp.id
            WHERE fpi.feature_flag_id = $1 AND fp.is_active = true
            ORDER BY fp.price_monthly_cents NULLS LAST
            "#,
        )
        .bind(feature_flag_id)
        .fetch_all(&self.pool)
        .await
    }

    // ==================== Organization Feature Packages ====================

    /// Subscribe organization to a package.
    pub async fn subscribe_org_to_package(
        &self,
        organization_id: Uuid,
        package_id: Uuid,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<OrganizationFeaturePackage, SqlxError> {
        sqlx::query_as::<_, OrganizationFeaturePackage>(
            r#"
            INSERT INTO organization_feature_packages (organization_id, package_id, expires_at)
            VALUES ($1, $2, $3)
            ON CONFLICT (organization_id, package_id)
            DO UPDATE SET
                is_active = true,
                expires_at = EXCLUDED.expires_at
            RETURNING id, organization_id, package_id, is_active, started_at, expires_at, created_at
            "#,
        )
        .bind(organization_id)
        .bind(package_id)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await
    }

    /// Unsubscribe organization from a package.
    pub async fn unsubscribe_org_from_package(
        &self,
        organization_id: Uuid,
        package_id: Uuid,
    ) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            r#"
            UPDATE organization_feature_packages
            SET is_active = false
            WHERE organization_id = $1 AND package_id = $2
            "#,
        )
        .bind(organization_id)
        .bind(package_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Get organization's active packages.
    pub async fn get_org_packages(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<OrganizationFeaturePackage>, SqlxError> {
        sqlx::query_as::<_, OrganizationFeaturePackage>(
            r#"
            SELECT id, organization_id, package_id, is_active, started_at, expires_at, created_at
            FROM organization_feature_packages
            WHERE organization_id = $1 AND is_active = true
              AND (expires_at IS NULL OR expires_at > NOW())
            "#,
        )
        .bind(organization_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Get all features available to an organization through packages.
    pub async fn get_org_package_features(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<Uuid>, SqlxError> {
        let rows = sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT DISTINCT fpi.feature_flag_id
            FROM organization_feature_packages ofp
            JOIN feature_package_items fpi ON fpi.package_id = ofp.package_id
            WHERE ofp.organization_id = $1 AND ofp.is_active = true
              AND (ofp.expires_at IS NULL OR ofp.expires_at > NOW())
            "#,
        )
        .bind(organization_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    // ==================== User Feature Preferences ====================

    /// Get user preference for a feature.
    pub async fn get_user_preference(
        &self,
        user_id: Uuid,
        feature_flag_id: Uuid,
    ) -> Result<Option<UserFeaturePreference>, SqlxError> {
        sqlx::query_as::<_, UserFeaturePreference>(
            r#"
            SELECT id, user_id, feature_flag_id, is_enabled, created_at, updated_at
            FROM user_feature_preferences
            WHERE user_id = $1 AND feature_flag_id = $2
            "#,
        )
        .bind(user_id)
        .bind(feature_flag_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Get all user preferences.
    pub async fn get_user_preferences(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<UserFeaturePreference>, SqlxError> {
        sqlx::query_as::<_, UserFeaturePreference>(
            r#"
            SELECT id, user_id, feature_flag_id, is_enabled, created_at, updated_at
            FROM user_feature_preferences
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Set user preference for a feature.
    pub async fn set_user_preference(
        &self,
        user_id: Uuid,
        feature_flag_id: Uuid,
        is_enabled: bool,
    ) -> Result<UserFeaturePreference, SqlxError> {
        sqlx::query_as::<_, UserFeaturePreference>(
            r#"
            INSERT INTO user_feature_preferences (user_id, feature_flag_id, is_enabled)
            VALUES ($1, $2, $3)
            ON CONFLICT (user_id, feature_flag_id)
            DO UPDATE SET is_enabled = EXCLUDED.is_enabled, updated_at = NOW()
            RETURNING id, user_id, feature_flag_id, is_enabled, created_at, updated_at
            "#,
        )
        .bind(user_id)
        .bind(feature_flag_id)
        .bind(is_enabled)
        .fetch_one(&self.pool)
        .await
    }

    // ==================== Feature Usage Events ====================

    /// Log a feature usage event.
    pub async fn log_event(
        &self,
        feature_flag_id: Uuid,
        user_id: Option<Uuid>,
        organization_id: Option<Uuid>,
        event_type: FeatureEventType,
        user_type: Option<&str>,
        metadata: serde_json::Value,
    ) -> Result<FeatureUsageEvent, SqlxError> {
        sqlx::query_as::<_, FeatureUsageEvent>(
            r#"
            INSERT INTO feature_usage_events (feature_flag_id, user_id, organization_id, event_type, user_type, metadata)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, feature_flag_id, user_id, organization_id, event_type, user_type, metadata, created_at
            "#,
        )
        .bind(feature_flag_id)
        .bind(user_id)
        .bind(organization_id)
        .bind(&event_type)
        .bind(user_type)
        .bind(&metadata)
        .fetch_one(&self.pool)
        .await
    }

    /// Get feature usage statistics for a period.
    pub async fn get_feature_stats(
        &self,
        feature_flag_id: Uuid,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
    ) -> Result<FeatureUsageStats, SqlxError> {
        self.feature_stats(feature_flag_id, None, start_date, end_date)
            .await
    }

    /// Feature usage statistics restricted to one organization's events.
    pub async fn get_feature_stats_for_org(
        &self,
        feature_flag_id: Uuid,
        organization_id: Uuid,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
    ) -> Result<FeatureUsageStats, SqlxError> {
        self.feature_stats(feature_flag_id, Some(organization_id), start_date, end_date)
            .await
    }

    async fn feature_stats(
        &self,
        feature_flag_id: Uuid,
        organization_id: Option<Uuid>,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
    ) -> Result<FeatureUsageStats, SqlxError> {
        let end = end_date.unwrap_or_else(Utc::now);
        let start = start_date.unwrap_or_else(|| end - Duration::days(30));

        // Get feature key
        let feature_key =
            sqlx::query_scalar::<_, String>("SELECT key FROM feature_flags WHERE id = $1")
                .bind(feature_flag_id)
                .fetch_one(&self.pool)
                .await?;

        #[derive(sqlx::FromRow)]
        struct StatsRow {
            total_events: Option<i64>,
            access_count: Option<i64>,
            blocked_count: Option<i64>,
            upgrade_prompt_count: Option<i64>,
            upgrade_clicked_count: Option<i64>,
            toggled_on_count: Option<i64>,
            toggled_off_count: Option<i64>,
            unique_users: Option<i64>,
        }

        let row = sqlx::query_as::<_, StatsRow>(
            r#"
            SELECT
                COUNT(*) as total_events,
                COUNT(*) FILTER (WHERE event_type = 'access') as access_count,
                COUNT(*) FILTER (WHERE event_type = 'blocked') as blocked_count,
                COUNT(*) FILTER (WHERE event_type = 'upgrade_prompt') as upgrade_prompt_count,
                COUNT(*) FILTER (WHERE event_type = 'upgrade_clicked') as upgrade_clicked_count,
                COUNT(*) FILTER (WHERE event_type = 'toggled_on') as toggled_on_count,
                COUNT(*) FILTER (WHERE event_type = 'toggled_off') as toggled_off_count,
                COUNT(DISTINCT user_id) as unique_users
            FROM feature_usage_events
            WHERE feature_flag_id = $1 AND created_at >= $2 AND created_at <= $3
              AND ($4::uuid IS NULL OR organization_id = $4)
            "#,
        )
        .bind(feature_flag_id)
        .bind(start)
        .bind(end)
        .bind(organization_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(FeatureUsageStats {
            feature_flag_id,
            feature_key,
            total_events: row.total_events.unwrap_or(0),
            access_count: row.access_count.unwrap_or(0),
            blocked_count: row.blocked_count.unwrap_or(0),
            upgrade_prompt_count: row.upgrade_prompt_count.unwrap_or(0),
            upgrade_clicked_count: row.upgrade_clicked_count.unwrap_or(0),
            toggled_on_count: row.toggled_on_count.unwrap_or(0),
            toggled_off_count: row.toggled_off_count.unwrap_or(0),
            unique_users: row.unique_users.unwrap_or(0),
            period_start: start,
            period_end: end,
        })
    }

    /// Get feature stats grouped by user type.
    pub async fn get_stats_by_user_type(
        &self,
        feature_flag_id: Uuid,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
    ) -> Result<Vec<FeatureStatsByUserType>, SqlxError> {
        let end = end_date.unwrap_or_else(Utc::now);
        let start = start_date.unwrap_or_else(|| end - Duration::days(30));

        sqlx::query_as::<_, FeatureStatsByUserType>(
            r#"
            SELECT
                user_type,
                COUNT(*) as total_events,
                COUNT(*) FILTER (WHERE event_type = 'access') as access_count,
                COUNT(*) FILTER (WHERE event_type = 'blocked') as blocked_count,
                COUNT(*) FILTER (WHERE event_type = 'upgrade_prompt') as upgrade_prompt_count,
                COUNT(*) FILTER (WHERE event_type = 'upgrade_clicked') as upgrade_clicked_count
            FROM feature_usage_events
            WHERE feature_flag_id = $1 AND created_at >= $2 AND created_at <= $3
            GROUP BY user_type
            ORDER BY total_events DESC
            "#,
        )
        .bind(feature_flag_id)
        .bind(start)
        .bind(end)
        .fetch_all(&self.pool)
        .await
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_repository_creation() {
        // This test just verifies the repository pattern is correct
        // Actual database tests would be integration tests
    }
}
