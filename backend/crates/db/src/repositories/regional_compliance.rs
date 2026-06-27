//! Regional Compliance Repository (Epic 72).

use crate::models::regional_compliance::{
    ConfigureCzechSvj, ConfigureSlovakAccounting, ConfigureSlovakGdpr, ConfigureSlovakVoting,
    CzechSvjConfig, GdprConsentCategory, Jurisdiction, RecordGdprConsent, SlovakAccountingConfig,
    SlovakDecisionType, SlovakGdprConfig, SlovakGdprConsent, SlovakVotingConfig,
};
use crate::DbPool;
use rust_decimal::Decimal;
use sqlx::Error as SqlxError;
use uuid::Uuid;

/// Repository for regional legal compliance features.
#[derive(Clone)]
pub struct RegionalComplianceRepository {
    pool: DbPool,
}

impl RegionalComplianceRepository {
    /// Create a new RegionalComplianceRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // ========================================================================
    // JURISDICTION & GENERAL STATUS
    // ========================================================================

    /// Get current jurisdiction for an organization.
    pub async fn get_jurisdiction(&self, organization_id: Uuid) -> Result<Jurisdiction, SqlxError> {
        let row: Option<(Jurisdiction,)> = sqlx::query_as(
            "SELECT jurisdiction FROM regional_compliance_configs WHERE organization_id = $1",
        )
        .bind(organization_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.0).unwrap_or(Jurisdiction::Slovakia))
    }

    /// Set jurisdiction for an organization.
    pub async fn set_jurisdiction(
        &self,
        organization_id: Uuid,
        jurisdiction: Jurisdiction,
    ) -> Result<Jurisdiction, SqlxError> {
        sqlx::query(
            r#"
            INSERT INTO regional_compliance_configs (organization_id, jurisdiction, updated_at)
            VALUES ($1, $2, NOW())
            ON CONFLICT (organization_id) DO UPDATE SET
                jurisdiction = EXCLUDED.jurisdiction,
                updated_at = NOW()
            "#,
        )
        .bind(organization_id)
        .bind(jurisdiction)
        .execute(&self.pool)
        .await?;

        Ok(jurisdiction)
    }

    /// List all configured buildings for an organization.
    pub async fn get_configured_buildings(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<Uuid>, SqlxError> {
        let buildings = sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT building_id FROM slovak_voting_configs WHERE organization_id = $1 AND enabled = true
            UNION
            SELECT building_id FROM czech_svj_configs WHERE organization_id = $1
            "#,
        )
        .bind(organization_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(buildings)
    }

    /// Get a static rule (quorum/legal reference) from the database.
    pub async fn get_quorum_rule(
        &self,
        jurisdiction: Jurisdiction,
        decision_type: &str,
    ) -> Result<Option<(Decimal, String)>, SqlxError> {
        let row = sqlx::query_as::<_, (Decimal, String)>(
            r#"
            SELECT required_quorum_percentage, legal_reference
            FROM jurisdiction_rules
            WHERE jurisdiction = $1 AND decision_type = $2
            "#,
        )
        .bind(jurisdiction)
        .bind(decision_type)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    // ========================================================================
    // SLOVAK VOTING CONFIG
    // ========================================================================

    /// Get slovak voting config for a building.
    pub async fn get_slovak_voting_config(
        &self,
        organization_id: Uuid,
        building_id: Uuid,
    ) -> Result<Option<SlovakVotingConfig>, SqlxError> {
        sqlx::query_as::<_, SlovakVotingConfig>(
            "SELECT * FROM slovak_voting_configs WHERE organization_id = $1 AND building_id = $2",
        )
        .bind(organization_id)
        .bind(building_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Configure Slovak voting for a building.
    pub async fn configure_slovak_voting(
        &self,
        organization_id: Uuid,
        config: ConfigureSlovakVoting,
    ) -> Result<SlovakVotingConfig, SqlxError> {
        let default_decision_type = config
            .default_decision_type
            .unwrap_or(SlovakDecisionType::SimpleMajority)
            .legal_reference()
            .to_string();

        let row = sqlx::query_as::<_, SlovakVotingConfig>(
            r#"
            INSERT INTO slovak_voting_configs (
                organization_id, building_id, enabled, default_decision_type,
                use_ownership_weight, min_notice_days, allow_proxy_voting, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
            ON CONFLICT (organization_id, building_id) DO UPDATE SET
                enabled = EXCLUDED.enabled,
                default_decision_type = EXCLUDED.default_decision_type,
                use_ownership_weight = EXCLUDED.use_ownership_weight,
                min_notice_days = EXCLUDED.min_notice_days,
                allow_proxy_voting = EXCLUDED.allow_proxy_voting,
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(organization_id)
        .bind(config.building_id)
        .bind(config.enabled)
        .bind(default_decision_type)
        .bind(config.use_ownership_weight)
        .bind(config.min_notice_days)
        .bind(config.allow_proxy_voting)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    // ========================================================================
    // SLOVAK ACCOUNTING CONFIG
    // ========================================================================

    /// Get Slovak accounting config for an organization.
    pub async fn get_slovak_accounting_config(
        &self,
        organization_id: Uuid,
    ) -> Result<Option<SlovakAccountingConfig>, SqlxError> {
        sqlx::query_as::<_, SlovakAccountingConfig>(
            "SELECT * FROM slovak_accounting_configs WHERE organization_id = $1",
        )
        .bind(organization_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Configure Slovak accounting for an organization.
    pub async fn configure_slovak_accounting(
        &self,
        organization_id: Uuid,
        config: ConfigureSlovakAccounting,
    ) -> Result<SlovakAccountingConfig, SqlxError> {
        let account_mapping = config
            .account_mapping
            .unwrap_or_else(|| serde_json::json!({"cash": "211", "bank": "221"}));

        let row = sqlx::query_as::<_, SlovakAccountingConfig>(
            r#"
            INSERT INTO slovak_accounting_configs (
                organization_id, export_format, ico, dic, ic_dph, default_iban, account_mapping, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
            ON CONFLICT (organization_id) DO UPDATE SET
                export_format = EXCLUDED.export_format,
                ico = EXCLUDED.ico,
                dic = EXCLUDED.dic,
                ic_dph = EXCLUDED.ic_dph,
                default_iban = EXCLUDED.default_iban,
                account_mapping = EXCLUDED.account_mapping,
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(organization_id)
        .bind(config.export_format.to_string())
        .bind(config.ico)
        .bind(config.dic)
        .bind(config.ic_dph)
        .bind(config.default_iban)
        .bind(account_mapping)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    // ========================================================================
    // SLOVAK GDPR CONFIG & CONSENT
    // ========================================================================

    /// Get Slovak GDPR config for an organization.
    pub async fn get_slovak_gdpr_config(
        &self,
        organization_id: Uuid,
    ) -> Result<Option<SlovakGdprConfig>, SqlxError> {
        sqlx::query_as::<_, SlovakGdprConfig>(
            "SELECT * FROM slovak_gdpr_configs WHERE organization_id = $1",
        )
        .bind(organization_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Configure Slovak GDPR for an organization.
    pub async fn configure_slovak_gdpr(
        &self,
        organization_id: Uuid,
        config: ConfigureSlovakGdpr,
    ) -> Result<SlovakGdprConfig, SqlxError> {
        let processing_purposes = config
            .processing_purposes
            .unwrap_or_else(|| serde_json::json!([]));
        let consent_texts = config
            .consent_texts
            .unwrap_or_else(|| serde_json::json!({}));

        let row = sqlx::query_as::<_, SlovakGdprConfig>(
            r#"
            INSERT INTO slovak_gdpr_configs (
                organization_id, dpo_name, dpo_email, dpo_phone, org_address, processing_purposes, consent_texts, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
            ON CONFLICT (organization_id) DO UPDATE SET
                dpo_name = EXCLUDED.dpo_name,
                dpo_email = EXCLUDED.dpo_email,
                dpo_phone = EXCLUDED.dpo_phone,
                org_address = EXCLUDED.org_address,
                processing_purposes = EXCLUDED.processing_purposes,
                consent_texts = EXCLUDED.consent_texts,
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(organization_id)
        .bind(config.dpo_name)
        .bind(config.dpo_email)
        .bind(config.dpo_phone)
        .bind(config.org_address)
        .bind(processing_purposes)
        .bind(consent_texts)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    /// Get all GDPR consents for a user.
    pub async fn get_gdpr_consents_for_user(
        &self,
        organization_id: Option<Uuid>,
        user_id: Uuid,
    ) -> Result<Vec<SlovakGdprConsent>, SqlxError> {
        sqlx::query_as::<_, SlovakGdprConsent>(
            r#"
            SELECT * FROM slovak_gdpr_consents
            WHERE user_id = $1 AND (organization_id = $2 OR (organization_id IS NULL AND $2 IS NULL))
            "#,
        )
        .bind(user_id)
        .bind(organization_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Record GDPR consent.
    pub async fn record_gdpr_consent(
        &self,
        organization_id: Option<Uuid>,
        user_id: Uuid,
        consent: RecordGdprConsent,
    ) -> Result<SlovakGdprConsent, SqlxError> {
        let row = sqlx::query_as::<_, SlovakGdprConsent>(
            r#"
            INSERT INTO slovak_gdpr_consents (
                user_id, organization_id, category, granted, consent_version, consented_at, withdrawn_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, NOW(), NULL, NOW())
            ON CONFLICT (user_id, organization_id, category) DO UPDATE SET
                granted = EXCLUDED.granted,
                consent_version = EXCLUDED.consent_version,
                consented_at = NOW(),
                withdrawn_at = NULL,
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(organization_id)
        .bind(consent.category.to_string())
        .bind(consent.granted)
        .bind(consent.consent_version)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    /// Withdraw GDPR consent.
    pub async fn withdraw_gdpr_consent(
        &self,
        organization_id: Option<Uuid>,
        user_id: Uuid,
        consent: RecordGdprConsent,
    ) -> Result<SlovakGdprConsent, SqlxError> {
        let row = sqlx::query_as::<_, SlovakGdprConsent>(
            r#"
            INSERT INTO slovak_gdpr_consents (
                user_id, organization_id, category, granted, consent_version, consented_at, withdrawn_at, updated_at
            )
            VALUES ($1, $2, $3, false, $4, NOW(), NOW(), NOW())
            ON CONFLICT (user_id, organization_id, category) DO UPDATE SET
                granted = false,
                consent_version = EXCLUDED.consent_version,
                withdrawn_at = NOW(),
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(organization_id)
        .bind(consent.category.to_string())
        .bind(consent.consent_version)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    // ========================================================================
    // CZECH SVJ CONFIG
    // ========================================================================

    /// Get Czech SVJ config.
    pub async fn get_czech_svj_config(
        &self,
        organization_id: Uuid,
        building_id: Uuid,
    ) -> Result<Option<CzechSvjConfig>, SqlxError> {
        sqlx::query_as::<_, CzechSvjConfig>(
            "SELECT * FROM czech_svj_configs WHERE organization_id = $1 AND building_id = $2",
        )
        .bind(organization_id)
        .bind(building_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Configure Czech SVJ.
    pub async fn configure_czech_svj(
        &self,
        organization_id: Uuid,
        config: ConfigureCzechSvj,
    ) -> Result<CzechSvjConfig, SqlxError> {
        let has_stanovy = config.stanovy_document_id.is_some();
        let default_decision_type = config
            .default_decision_type
            .unwrap_or(crate::models::regional_compliance::CzechDecisionType::SimpleMajority)
            .legal_reference()
            .to_string();

        let row = sqlx::query_as::<_, CzechSvjConfig>(
            r#"
            INSERT INTO czech_svj_configs (
                organization_id, building_id, org_type, ico, has_stanovy,
                stanovy_document_id, stanovy_effective_date, default_decision_type,
                use_ownership_weight, notary_threshold_czk, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW())
            ON CONFLICT (organization_id, building_id) DO UPDATE SET
                org_type = EXCLUDED.org_type,
                ico = EXCLUDED.ico,
                has_stanovy = EXCLUDED.has_stanovy,
                stanovy_document_id = EXCLUDED.stanovy_document_id,
                stanovy_effective_date = EXCLUDED.stanovy_effective_date,
                default_decision_type = EXCLUDED.default_decision_type,
                use_ownership_weight = EXCLUDED.use_ownership_weight,
                notary_threshold_czk = EXCLUDED.notary_threshold_czk,
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(organization_id)
        .bind(config.building_id)
        .bind(config.org_type.to_string())
        .bind(config.ico)
        .bind(has_stanovy)
        .bind(config.stanovy_document_id)
        .bind(config.stanovy_effective_date)
        .bind(default_decision_type)
        .bind(config.use_ownership_weight)
        .bind(config.notary_threshold_czk)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    /// Calculate real accounting metrics for Slovak accounting exports
    pub async fn get_accounting_metrics(
        &self,
        organization_id: Uuid,
        from_date: NaiveDate,
        to_date: NaiveDate,
    ) -> Result<(i32, i32, Decimal, Decimal, Decimal, Decimal), SqlxError> {
        let (invoice_count, total_revenue): (i64, Option<Decimal>) = sqlx::query_as(
            r#"
            SELECT COUNT(*), SUM(total)
            FROM invoices
            WHERE organization_id = $1 AND issue_date >= $2 AND issue_date <= $3
            "#,
        )
        .bind(organization_id)
        .bind(from_date)
        .bind(to_date)
        .fetch_one(&self.pool)
        .await?;

        let (payment_count, total_collected): (i64, Option<Decimal>) = sqlx::query_as(
            r#"
            SELECT COUNT(*), SUM(amount)
            FROM payments
            WHERE organization_id = $1 AND payment_date >= $2 AND payment_date <= $3 AND status = 'completed'
            "#,
        )
        .bind(organization_id)
        .bind(from_date)
        .bind(to_date)
        .fetch_one(&self.pool)
        .await?;

        let (total_receivables,): (Option<Decimal>,) = sqlx::query_as(
            r#"
            SELECT SUM(balance_due)
            FROM invoices
            WHERE organization_id = $1 AND issue_date >= $2 AND issue_date <= $3 AND status != 'paid' AND status != 'cancelled'
            "#,
        )
        .bind(organization_id)
        .bind(from_date)
        .bind(to_date)
        .fetch_one(&self.pool)
        .await?;

        // Fallbacks for Option<Decimal>
        let total_revenue = total_revenue.unwrap_or(Decimal::ZERO);
        let total_receivables = total_receivables.unwrap_or(Decimal::ZERO);

        Ok((
            invoice_count as i32,
            payment_count as i32,
            total_revenue,
            Decimal::ZERO, // total_expenses
            total_receivables,
            Decimal::ZERO, // total_payables
        ))
    }
}
