//! Rental statistics & platform sync status.

use super::RentalRepository;
use crate::models::rental::{PlatformSyncStatus, RentalStatistics};
use chrono::{Datelike, Duration, NaiveDate, Utc};
use rust_decimal::Decimal;
use sqlx::Error as SqlxError;
use uuid::Uuid;

impl RentalRepository {
    // ========================================================================
    // Statistics
    // ========================================================================

    /// Get rental statistics for organization.
    pub async fn get_statistics(&self, org_id: Uuid) -> Result<RentalStatistics, SqlxError> {
        // Get unit counts
        let (total_units,): (i64,) =
            sqlx::query_as(r#"SELECT COUNT(DISTINCT id) FROM units WHERE organization_id = $1"#)
                .bind(org_id)
                .fetch_one(&self.pool)
                .await?;

        let (connected_units,): (i64,) = sqlx::query_as(
            r#"SELECT COUNT(DISTINCT unit_id) FROM rental_platform_connections WHERE organization_id = $1 AND is_active = true"#,
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await?;

        // Get booking counts
        let today = Utc::now().date_naive();
        let (active_bookings,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM rental_bookings
            WHERE organization_id = $1
                AND status IN ('confirmed', 'checked_in')
                AND check_in <= $2 AND check_out >= $2
            "#,
        )
        .bind(org_id)
        .bind(today)
        .fetch_one(&self.pool)
        .await?;

        let (upcoming_bookings,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM rental_bookings
            WHERE organization_id = $1
                AND status IN ('pending', 'confirmed')
                AND check_in > $2
            "#,
        )
        .bind(org_id)
        .bind(today)
        .fetch_one(&self.pool)
        .await?;

        let (pending_registrations,): (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*) FROM rental_guests WHERE organization_id = $1 AND status = 'pending'"#,
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await?;

        // Calculate occupancy (simplified)
        let occupancy_rate = if total_units > 0 {
            (active_bookings as f64 / total_units as f64) * 100.0
        } else {
            0.0
        };

        // Revenue calculations (simplified)
        let month_start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
        let last_month_start = month_start - Duration::days(30);

        let (revenue_this_month,): (Option<Decimal>,) = sqlx::query_as(
            r#"
            SELECT COALESCE(SUM(total_amount), 0)
            FROM rental_bookings
            WHERE organization_id = $1
                AND status NOT IN ('cancelled', 'no_show')
                AND check_in >= $2
            "#,
        )
        .bind(org_id)
        .bind(month_start)
        .fetch_one(&self.pool)
        .await?;

        let (revenue_last_month,): (Option<Decimal>,) = sqlx::query_as(
            r#"
            SELECT COALESCE(SUM(total_amount), 0)
            FROM rental_bookings
            WHERE organization_id = $1
                AND status NOT IN ('cancelled', 'no_show')
                AND check_in >= $2 AND check_in < $3
            "#,
        )
        .bind(org_id)
        .bind(last_month_start)
        .bind(month_start)
        .fetch_one(&self.pool)
        .await?;

        Ok(RentalStatistics {
            total_units,
            connected_units,
            active_bookings,
            upcoming_bookings,
            pending_registrations,
            occupancy_rate,
            revenue_this_month: revenue_this_month.unwrap_or_default(),
            revenue_last_month: revenue_last_month.unwrap_or_default(),
        })
    }

    /// Get platform sync status.
    pub async fn get_platform_sync_status(
        &self,
        org_id: Uuid,
    ) -> Result<Vec<PlatformSyncStatus>, SqlxError> {
        let statuses = sqlx::query_as::<_, (String, i64, Option<chrono::DateTime<Utc>>, i64)>(
            r#"
            SELECT
                platform::text,
                COUNT(*) as connections_count,
                MAX(last_sync_at) as last_sync,
                COUNT(*) FILTER (WHERE sync_error IS NOT NULL) as errors
            FROM rental_platform_connections
            WHERE organization_id = $1 AND is_active = true
            GROUP BY platform
            "#,
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(statuses
            .into_iter()
            .map(|(platform, count, last_sync, errors)| PlatformSyncStatus {
                platform,
                connections_count: count,
                last_sync_at: last_sync,
                sync_errors_count: errors,
            })
            .collect())
    }
}
