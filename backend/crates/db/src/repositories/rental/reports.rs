//! Rental guest reports & report previews (Story 18.4).

use super::get_country_name;
use super::RentalRepository;
use crate::models::rental::{
    report_status, GenerateReport, NationalityStats, RentalGuestReport, ReportPreview,
    ReportSummary,
};
use chrono::{NaiveDate, Utc};
use sqlx::Error as SqlxError;
use uuid::Uuid;

impl RentalRepository {
    // ========================================================================
    // Reports (Story 18.4)
    // ========================================================================

    /// Generate report preview.
    pub async fn generate_report_preview(
        &self,
        org_id: Uuid,
        building_id: Uuid,
        period_start: NaiveDate,
        period_end: NaiveDate,
    ) -> Result<ReportPreview, SqlxError> {
        // Get building name
        let (building_name,): (String,) =
            sqlx::query_as(r#"SELECT name FROM buildings WHERE id = $1"#)
                .bind(building_id)
                .fetch_one(&self.pool)
                .await?;

        // Count guests and get nationality breakdown
        let stats = sqlx::query_as::<_, (i64, Option<String>)>(
            r#"
            SELECT COUNT(g.id), g.nationality
            FROM rental_guests g
            JOIN rental_bookings b ON b.id = g.booking_id
            JOIN units u ON u.id = b.unit_id
            WHERE b.organization_id = $1
                AND u.building_id = $2
                AND b.check_in >= $3
                AND b.check_out <= $4
                AND g.status IN ('registered', 'reported')
            GROUP BY g.nationality
            "#,
        )
        .bind(org_id)
        .bind(building_id)
        .bind(period_start)
        .bind(period_end)
        .fetch_all(&self.pool)
        .await?;

        let total_guests: i32 = stats.iter().map(|(count, _)| *count as i32).sum();

        let by_nationality: Vec<NationalityStats> = stats
            .into_iter()
            .map(|(count, nationality)| {
                let nat = nationality.unwrap_or_else(|| "UNK".to_string());
                NationalityStats {
                    nationality: nat.clone(),
                    country_name: get_country_name(&nat),
                    count: count as i32,
                    percentage: if total_guests > 0 {
                        (count as f64 / total_guests as f64) * 100.0
                    } else {
                        0.0
                    },
                }
            })
            .collect();

        // Count bookings
        let (bookings_count,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(DISTINCT b.id)
            FROM rental_bookings b
            JOIN units u ON u.id = b.unit_id
            WHERE b.organization_id = $1
                AND u.building_id = $2
                AND b.check_in >= $3
                AND b.check_out <= $4
            "#,
        )
        .bind(org_id)
        .bind(building_id)
        .bind(period_start)
        .bind(period_end)
        .fetch_one(&self.pool)
        .await?;

        Ok(ReportPreview {
            building_id,
            building_name,
            period_start,
            period_end,
            total_guests,
            by_nationality,
            bookings_count: bookings_count as i32,
        })
    }

    /// Create report.
    pub async fn create_report(
        &self,
        org_id: Uuid,
        data: GenerateReport,
        user_id: Uuid,
    ) -> Result<RentalGuestReport, SqlxError> {
        // Get preview data
        let preview = self
            .generate_report_preview(org_id, data.building_id, data.period_start, data.period_end)
            .await?;

        let authority_name = match data.authority_code.as_str() {
            "SK_UHUL" => "UHUL Slovakia",
            "CZ_CIZPOL" => "Czech Foreign Police",
            "AT_ZMR" => "Austria ZMR",
            "DE_MELDEWESEN" => "Germany Meldewesen",
            _ => "Unknown",
        };

        let guests_by_nationality = serde_json::to_value(&preview.by_nationality).ok();

        let report = sqlx::query_as::<_, RentalGuestReport>(
            r#"
            INSERT INTO rental_guest_reports (
                organization_id, building_id, report_type, period_start, period_end,
                authority_code, authority_name, total_guests, guests_by_nationality,
                report_format, status, generated_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.building_id)
        .bind(&data.report_type)
        .bind(data.period_start)
        .bind(data.period_end)
        .bind(&data.authority_code)
        .bind(authority_name)
        .bind(preview.total_guests)
        .bind(guests_by_nationality)
        .bind(&data.report_format)
        .bind(report_status::GENERATED)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(report)
    }

    /// Find report by ID.
    pub async fn find_report_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<RentalGuestReport>, SqlxError> {
        let report = sqlx::query_as::<_, RentalGuestReport>(
            r#"SELECT * FROM rental_guest_reports WHERE id = $1"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(report)
    }

    /// Find report by ID scoped to an organization.
    ///
    /// SECURITY (#804): prevents reading another org's authority report by UUID.
    pub async fn find_report_for_org(
        &self,
        org_id: Uuid,
        id: Uuid,
    ) -> Result<Option<RentalGuestReport>, SqlxError> {
        let report = sqlx::query_as::<_, RentalGuestReport>(
            r#"SELECT * FROM rental_guest_reports WHERE id = $1 AND organization_id = $2"#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(report)
    }

    /// Submit report scoped to an organization.
    ///
    /// SECURITY (#804): the `AND organization_id = $4` guard prevents a tenant
    /// from submitting another org's report. Returns `None` when no row matched.
    pub async fn submit_report_for_org(
        &self,
        org_id: Uuid,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<RentalGuestReport>, SqlxError> {
        let report = sqlx::query_as::<_, RentalGuestReport>(
            r#"
            UPDATE rental_guest_reports SET
                status = $2::guest_registration_status,
                submitted_at = NOW(),
                submitted_by = $3,
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $4
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(report_status::SUBMITTED)
        .bind(user_id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await?;

        // Mark guests as reported only when the report belonged to this org.
        if report.is_some() {
            sqlx::query(
                r#"
                UPDATE rental_guests SET
                    status = 'reported',
                    reported_at = NOW()
                WHERE booking_id IN (
                    SELECT b.id FROM rental_bookings b
                    JOIN units u ON u.id = b.unit_id
                    JOIN rental_guest_reports r ON r.building_id = u.building_id
                    WHERE r.id = $1
                        AND b.check_in >= r.period_start
                        AND b.check_out <= r.period_end
                )
                AND status = 'registered'
                "#,
            )
            .bind(id)
            .execute(&self.pool)
            .await?;
        }

        Ok(report)
    }

    /// Submit report.
    pub async fn submit_report(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<RentalGuestReport, SqlxError> {
        let report = sqlx::query_as::<_, RentalGuestReport>(
            r#"
            UPDATE rental_guest_reports SET
                status = $2::guest_registration_status,
                submitted_at = NOW(),
                submitted_by = $3,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(report_status::SUBMITTED)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        // Mark guests as reported
        sqlx::query(
            r#"
            UPDATE rental_guests SET
                status = 'reported',
                reported_at = NOW()
            WHERE booking_id IN (
                SELECT b.id FROM rental_bookings b
                JOIN units u ON u.id = b.unit_id
                JOIN rental_guest_reports r ON r.building_id = u.building_id
                WHERE r.id = $1
                    AND b.check_in >= r.period_start
                    AND b.check_out <= r.period_end
            )
            AND status = 'registered'
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(report)
    }

    /// List reports for organization.
    pub async fn list_reports(
        &self,
        org_id: Uuid,
        building_id: Option<Uuid>,
        limit: i32,
    ) -> Result<Vec<ReportSummary>, SqlxError> {
        let reports = sqlx::query_as::<
            _,
            (
                Uuid,
                Uuid,
                String,
                String,
                NaiveDate,
                NaiveDate,
                String,
                String,
                i32,
                String,
                Option<String>,
                Option<chrono::DateTime<Utc>>,
            ),
        >(
            r#"
            SELECT
                r.id, r.building_id, b.name, r.report_type,
                r.period_start, r.period_end, r.authority_code, r.authority_name,
                r.total_guests, r.status, r.report_file_url, r.submitted_at
            FROM rental_guest_reports r
            JOIN buildings b ON b.id = r.building_id
            WHERE r.organization_id = $1
                AND ($2::uuid IS NULL OR r.building_id = $2)
            ORDER BY r.period_start DESC
            LIMIT $3
            "#,
        )
        .bind(org_id)
        .bind(building_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(reports
            .into_iter()
            .map(
                |(
                    id,
                    building_id,
                    building_name,
                    report_type,
                    period_start,
                    period_end,
                    authority_code,
                    authority_name,
                    total_guests,
                    status,
                    report_file_url,
                    submitted_at,
                )| {
                    ReportSummary {
                        id,
                        building_id,
                        building_name,
                        report_type,
                        period_start,
                        period_end,
                        authority_code,
                        authority_name,
                        total_guests,
                        status,
                        report_file_url,
                        submitted_at,
                    }
                },
            )
            .collect())
    }
}
