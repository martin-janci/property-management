//! Data Residency Repository (Epic 146).
//!
//! Handles configuration, region catalog, routing status,
//! compliance verification results, and tamper-evident audit logs.

use crate::models::data_residency::*;
use crate::DbPool;
use chrono::{DateTime, Utc};
use sqlx::{Error as SqlxError, Row};
use uuid::Uuid;

/// Helper to convert DataRegion enum to database VARCHAR string
pub fn data_region_to_string(r: &DataRegion) -> String {
    match r {
        DataRegion::EuWest => "eu_west".to_string(),
        DataRegion::EuCentral => "eu_central".to_string(),
        DataRegion::UsEast => "us_east".to_string(),
        DataRegion::UsWest => "us_west".to_string(),
        DataRegion::ApacSoutheast => "apac_southeast".to_string(),
        DataRegion::ApacSouth => "apac_south".to_string(),
        DataRegion::UkSouth => "uk_south".to_string(),
        DataRegion::ChNorth => "ch_north".to_string(),
    }
}

/// Helper to convert database VARCHAR string to DataRegion enum
pub fn string_to_data_region(s: &str) -> DataRegion {
    match s {
        "eu_west" => DataRegion::EuWest,
        "eu_central" => DataRegion::EuCentral,
        "us_east" => DataRegion::UsEast,
        "us_west" => DataRegion::UsWest,
        "apac_southeast" => DataRegion::ApacSoutheast,
        "apac_south" => DataRegion::ApacSouth,
        "uk_south" => DataRegion::UkSouth,
        "ch_north" => DataRegion::ChNorth,
        _ => DataRegion::EuWest, // Default fallback
    }
}

/// Repository for Data Residency operations.
#[derive(Clone)]
pub struct DataResidencyRepository {
    pool: DbPool,
}

impl DataResidencyRepository {
    /// Create a new DataResidencyRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // ========================================================================
    // CONFIGURATION
    // ========================================================================

    /// Get current configuration for an organization.
    pub async fn get_config(&self, org_id: Uuid) -> Result<Option<DataResidencyConfig>, SqlxError> {
        sqlx::query_as::<_, DataResidencyConfig>(
            "SELECT * FROM data_residency_config WHERE organization_id = $1",
        )
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Upsert configuration for an organization.
    pub async fn upsert_config(
        &self,
        org_id: Uuid,
        payload: ConfigureDataResidency,
        user_id: Uuid,
    ) -> Result<DataResidencyConfig, SqlxError> {
        let primary_region_str = data_region_to_string(&payload.primary_region);
        let backup_region_str = payload.backup_region.as_ref().map(data_region_to_string);
        let overrides_json = payload
            .data_type_overrides
            .as_ref()
            .map(|o| serde_json::to_value(o).unwrap_or(serde_json::Value::Null));

        let config = sqlx::query_as::<_, DataResidencyConfig>(
            r#"
            INSERT INTO data_residency_config (
                id, organization_id, primary_region, backup_region, status,
                allow_cross_region_access, data_type_overrides, compliance_notes,
                created_by, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, 'active', $5, $6, $7, $8, NOW(), NOW())
            ON CONFLICT (organization_id) DO UPDATE SET
                primary_region = EXCLUDED.primary_region,
                backup_region = EXCLUDED.backup_region,
                allow_cross_region_access = EXCLUDED.allow_cross_region_access,
                data_type_overrides = EXCLUDED.data_type_overrides,
                compliance_notes = EXCLUDED.compliance_notes,
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(org_id)
        .bind(primary_region_str)
        .bind(backup_region_str)
        .bind(payload.allow_cross_region_access)
        .bind(overrides_json)
        .bind(payload.compliance_notes)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(config)
    }

    // ========================================================================
    // REGIONS
    // ========================================================================

    /// List all available reference regions.
    pub async fn list_available_regions(&self) -> Result<Vec<RegionInfo>, SqlxError> {
        let rows = sqlx::query(
            r#"
            SELECT region::TEXT as region_str, display_name, location_code, compliance_frameworks, available
            FROM residency_regions
            WHERE available = true
            ORDER BY region ASC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let regions = rows
            .into_iter()
            .map(|row| {
                let r_str: String = row.get("region_str");
                RegionInfo {
                    region: string_to_data_region(&r_str),
                    display_name: row.get("display_name"),
                    location_code: row.get("location_code"),
                    compliance_frameworks: row.get("compliance_frameworks"),
                    available: row.get("available"),
                }
            })
            .collect();

        Ok(regions)
    }

    // ========================================================================
    // LOG ACCESS & ROUTING
    // ========================================================================

    /// Log a cross-region data access.
    pub async fn log_cross_region_access(
        &self,
        org_id: Uuid,
        user_id: Uuid,
        payload: LogCrossRegionAccess,
        ip_address: Option<String>,
    ) -> Result<CrossRegionAccessLog, SqlxError> {
        let log = sqlx::query_as::<_, CrossRegionAccessLog>(
            r#"
            INSERT INTO cross_region_access_log (
                id, organization_id, user_id, data_type, source_region, access_region,
                access_type, resource_id, reason, ip_address, accessed_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW())
            RETURNING *
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(org_id)
        .bind(user_id)
        .bind(payload.data_type.to_string())
        .bind(data_region_to_string(&payload.source_region))
        .bind(data_region_to_string(&payload.access_region))
        .bind(payload.access_type.to_string())
        .bind(payload.resource_id)
        .bind(payload.reason)
        .bind(ip_address)
        .fetch_one(&self.pool)
        .await?;

        Ok(log)
    }

    /// List cross-region access logs for an organization.
    pub async fn list_access_logs(
        &self,
        org_id: Uuid,
    ) -> Result<Vec<CrossRegionAccessLog>, SqlxError> {
        sqlx::query_as::<_, CrossRegionAccessLog>(
            "SELECT * FROM cross_region_access_log WHERE organization_id = $1 ORDER BY accessed_at DESC"
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Count recent cross-region accesses.
    pub async fn count_recent_cross_region_accesses(
        &self,
        org_id: Uuid,
        days: i64,
    ) -> Result<i64, SqlxError> {
        let row: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM cross_region_access_log
            WHERE organization_id = $1
              AND accessed_at >= NOW() - ($2 || ' days')::INTERVAL
            "#,
        )
        .bind(org_id)
        .bind(days.to_string())
        .fetch_one(&self.pool)
        .await?;

        Ok(row.0)
    }

    /// Get cross-region access counts by type.
    pub async fn get_cross_region_access_counts_by_type(
        &self,
        org_id: Uuid,
    ) -> Result<Vec<AccessTypeCount>, SqlxError> {
        let rows: Vec<(String, i64)> = sqlx::query_as(
            r#"
            SELECT access_type, COUNT(*) as count
            FROM cross_region_access_log
            WHERE organization_id = $1
            GROUP BY access_type
            "#,
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await?;

        let counts = rows
            .into_iter()
            .map(|(at, c)| {
                let access_type = match at.as_str() {
                    "write" => AccessType::Write,
                    "delete" => AccessType::Delete,
                    "migration" => AccessType::Migration,
                    _ => AccessType::Read,
                };
                AccessTypeCount {
                    access_type,
                    count: c,
                }
            })
            .collect();

        Ok(counts)
    }

    // ========================================================================
    // COMPLIANCE VERIFICATION
    // ========================================================================

    /// Create compliance verification result.
    pub async fn create_verification_result(
        &self,
        org_id: Uuid,
        is_compliant: bool,
        data_locations: serde_json::Value,
        issues: Option<serde_json::Value>,
        details: Option<serde_json::Value>,
        verified_by: Uuid,
    ) -> Result<ComplianceVerificationResult, SqlxError> {
        sqlx::query_as::<_, ComplianceVerificationResult>(
            r#"
            INSERT INTO compliance_verification_result (
                id, organization_id, is_compliant, verified_at, data_locations,
                issues, details, verified_by
            )
            VALUES ($1, $2, $3, NOW(), $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(org_id)
        .bind(is_compliant)
        .bind(data_locations)
        .bind(issues)
        .bind(details)
        .bind(verified_by)
        .fetch_one(&self.pool)
        .await
    }

    /// Get compliance verification result by ID.
    pub async fn get_verification_result(
        &self,
        org_id: Uuid,
        id: Uuid,
    ) -> Result<Option<ComplianceVerificationResult>, SqlxError> {
        sqlx::query_as::<_, ComplianceVerificationResult>(
            "SELECT * FROM compliance_verification_result WHERE id = $1 AND organization_id = $2",
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
    }

    // ========================================================================
    // AUDIT TRAIL
    // ========================================================================

    /// Create a new audit log entry with tamper-evident hashing.
    pub async fn create_audit_entry(
        &self,
        org_id: Uuid,
        user_id: Option<Uuid>,
        event: ResidencyAuditEvent,
        description: &str,
        previous_state: Option<serde_json::Value>,
        new_state: Option<serde_json::Value>,
        details: Option<serde_json::Value>,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<DataResidencyAuditLog, SqlxError> {
        // Fetch previous hash (most recent log entry)
        let prev_hash: Option<String> = sqlx::query_scalar(
            "SELECT record_hash FROM residency_audit_log WHERE organization_id = $1 ORDER BY created_at DESC LIMIT 1"
        )
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await?;

        let id = Uuid::new_v4();
        let created_at = Utc::now();

        // Compute hash
        let mut hasher = sha2::Sha256::new();
        use sha2::Digest;
        hasher.update(id.to_string());
        hasher.update(event.as_str());
        hasher.update(description);
        hasher.update(created_at.to_rfc3339());
        if let Some(ref ph) = prev_hash {
            hasher.update(ph);
        }
        let record_hash = hex::encode(hasher.finalize());

        let log = sqlx::query_as::<_, DataResidencyAuditLog>(
            r#"
            INSERT INTO residency_audit_log (
                id, organization_id, event_type, user_id, previous_state, new_state,
                details, ip_address, user_agent, record_hash, previous_hash, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(org_id)
        .bind(event.as_str())
        .bind(user_id)
        .bind(previous_state)
        .bind(new_state)
        .bind(details)
        .bind(ip_address)
        .bind(user_agent)
        .bind(record_hash)
        .bind(prev_hash)
        .bind(created_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(log)
    }

    /// List audit logs with pagination and filters.
    pub async fn list_audit_logs(
        &self,
        org_id: Uuid,
        query: AuditLogQuery,
    ) -> Result<(Vec<DataResidencyAuditLog>, i64), SqlxError> {
        let event_type_str = query.event_type.as_ref().map(|e| e.as_str().to_string());
        let limit = query.limit.unwrap_or(20);
        let offset = query.offset.unwrap_or(0);

        // Total count
        let total: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM residency_audit_log
            WHERE organization_id = $1
              AND ($2::VARCHAR IS NULL OR event_type = $2)
              AND ($3::UUID IS NULL OR user_id = $3)
              AND ($4::TIMESTAMPTZ IS NULL OR created_at >= $4)
              AND ($5::TIMESTAMPTZ IS NULL OR created_at <= $5)
            "#,
        )
        .bind(org_id)
        .bind(event_type_str.clone())
        .bind(query.user_id)
        .bind(query.from_date)
        .bind(query.to_date)
        .fetch_one(&self.pool)
        .await?;

        // Entries
        let entries = sqlx::query_as::<_, DataResidencyAuditLog>(
            r#"
            SELECT * FROM residency_audit_log
            WHERE organization_id = $1
              AND ($2::VARCHAR IS NULL OR event_type = $2)
              AND ($3::UUID IS NULL OR user_id = $3)
              AND ($4::TIMESTAMPTZ IS NULL OR created_at >= $4)
              AND ($5::TIMESTAMPTZ IS NULL OR created_at <= $5)
            ORDER BY created_at DESC
            LIMIT $6 OFFSET $7
            "#,
        )
        .bind(org_id)
        .bind(event_type_str)
        .bind(query.user_id)
        .bind(query.from_date)
        .bind(query.to_date)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok((entries, total.0))
    }

    /// Get specific audit entry.
    pub async fn get_audit_entry(
        &self,
        org_id: Uuid,
        id: Uuid,
    ) -> Result<Option<DataResidencyAuditLog>, SqlxError> {
        sqlx::query_as::<_, DataResidencyAuditLog>(
            "SELECT * FROM residency_audit_log WHERE id = $1 AND organization_id = $2",
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Verify tamper-evident audit chain.
    pub async fn verify_audit_chain(&self, org_id: Uuid) -> Result<serde_json::Value, SqlxError> {
        let logs = sqlx::query_as::<_, DataResidencyAuditLog>(
            "SELECT * FROM residency_audit_log WHERE organization_id = $1 ORDER BY created_at ASC",
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await?;

        let mut chain_valid = true;
        let mut prev_hash: Option<String> = None;
        let entries_verified = logs.len();

        let first_entry_date = logs.first().map(|l| l.created_at);
        let last_entry_date = logs.last().map(|l| l.created_at);

        for log in &logs {
            let mut hasher = sha2::Sha256::new();
            use sha2::Digest;
            hasher.update(log.id.to_string());
            hasher.update(&log.event_type);
            // Recompute description block if we want it to verify exactly
            // If details -> description was stored we can use it, else hash details string
            let details_str = log
                .details
                .as_ref()
                .map(|d| d.to_string())
                .unwrap_or_default();
            hasher.update(&details_str);
            hasher.update(log.created_at.to_rfc3339());
            if let Some(ref ph) = prev_hash {
                hasher.update(ph);
            }
            let computed = hex::encode(hasher.finalize());

            // If chain was ever broken, verify checks fail
            if let Some(ref ph) = log.previous_hash {
                if prev_hash.as_ref() != Some(ph) {
                    chain_valid = false;
                }
            } else if prev_hash.is_some() {
                chain_valid = false;
            }

            if computed != log.record_hash {
                // Keep checking but note it
                chain_valid = false;
            }

            prev_hash = Some(log.record_hash.clone());
        }

        Ok(serde_json::json!({
            "chain_valid": chain_valid,
            "entries_verified": entries_verified,
            "first_entry_date": first_entry_date,
            "last_entry_date": last_entry_date,
            "verification_timestamp": Utc::now()
        }))
    }
}
