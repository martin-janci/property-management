// Epic 135: Enhanced Tenant Screening with AI Risk Scoring
// Repository for AI-powered tenant screening operations
//
// # RLS Integration (PAP-67 / PAP-74)
//
// Migration `00179` put `FORCE ROW LEVEL SECURITY` + the canonical
// `get_current_org_id()` policy on the screening tables (`screening_*`,
// `ai_risk_scoring_models`). Under `FORCE` the api-server's owner connection is
// no longer exempt, so a query issued on a connection without
// `app.current_org_id` set collapses to deny-all (own-org reads return empty,
// writes fail).
//
// Every method therefore takes an **executor whose connection already has RLS
// context set** (org + user GUCs) — in handlers this comes from the
// `RlsConnection` extractor via `&mut **rls.conn()`. The repository holds **no
// pool**, so there is no way to issue a query that bypasses RLS. This mirrors
// the `work_order.rs` / `budget.rs` precedent.

use rust_decimal::Decimal;
use sqlx::{Acquire, Executor, PgConnection, PgPool, Postgres};
use uuid::Uuid;

use crate::models::enhanced_tenant_screening::*;

/// Repository for enhanced tenant screening operations.
///
/// Stateless: every method receives an RLS-context-bearing executor. The repo
/// holds no pool so it cannot issue an un-scoped (deny-all under `FORCE`) query.
#[derive(Debug, Clone)]
pub struct EnhancedTenantScreeningRepository;

impl EnhancedTenantScreeningRepository {
    /// Create a new repository instance.
    ///
    /// The pool argument is retained for construction-site compatibility with
    /// the other repositories on `AppState`; this repo deliberately does not
    /// store it (see module docs — all queries run on a context-set connection
    /// supplied by the handler's `RlsConnection`).
    pub fn new(_pool: PgPool) -> Self {
        Self
    }

    // =========================================================================
    // AI Risk Scoring Models
    // =========================================================================

    /// Create a new AI risk scoring model.
    pub async fn create_risk_model<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        user_id: Uuid,
        req: CreateAiRiskScoringModel,
    ) -> Result<AiRiskScoringModel, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO ai_risk_scoring_models (
                organization_id, name, description, created_by,
                credit_history_weight, rental_history_weight, income_stability_weight,
                employment_stability_weight, eviction_history_weight, criminal_background_weight,
                identity_verification_weight, reference_quality_weight,
                excellent_threshold, good_threshold, fair_threshold, poor_threshold,
                auto_approve_threshold, auto_reject_threshold
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(&req.name)
        .bind(&req.description)
        .bind(user_id)
        .bind(
            req.credit_history_weight
                .unwrap_or_else(|| Decimal::new(25, 2)),
        )
        .bind(
            req.rental_history_weight
                .unwrap_or_else(|| Decimal::new(20, 2)),
        )
        .bind(
            req.income_stability_weight
                .unwrap_or_else(|| Decimal::new(20, 2)),
        )
        .bind(
            req.employment_stability_weight
                .unwrap_or_else(|| Decimal::new(15, 2)),
        )
        .bind(
            req.eviction_history_weight
                .unwrap_or_else(|| Decimal::new(10, 2)),
        )
        .bind(
            req.criminal_background_weight
                .unwrap_or_else(|| Decimal::new(5, 2)),
        )
        .bind(
            req.identity_verification_weight
                .unwrap_or_else(|| Decimal::new(3, 2)),
        )
        .bind(
            req.reference_quality_weight
                .unwrap_or_else(|| Decimal::new(2, 2)),
        )
        .bind(req.excellent_threshold.unwrap_or(80))
        .bind(req.good_threshold.unwrap_or(60))
        .bind(req.fair_threshold.unwrap_or(40))
        .bind(req.poor_threshold.unwrap_or(20))
        .bind(req.auto_approve_threshold)
        .bind(req.auto_reject_threshold)
        .fetch_one(executor)
        .await
    }

    /// Get AI risk scoring model by ID, scoped to the owning organization.
    ///
    /// Issue #834 (cross-tenant PII): models carry provider weighting / scoring
    /// configuration. The `organization_id = $2` predicate is defense-in-depth
    /// alongside RLS — under FORCE-RLS a foreign model is already invisible.
    pub async fn get_risk_model<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        id: Uuid,
    ) -> Result<Option<AiRiskScoringModel>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            "SELECT * FROM ai_risk_scoring_models WHERE id = $1 AND organization_id = $2",
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    /// Get active AI risk scoring model for organization.
    pub async fn get_active_risk_model<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
    ) -> Result<Option<AiRiskScoringModel>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            "SELECT * FROM ai_risk_scoring_models WHERE organization_id = $1 AND is_active = true",
        )
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    /// List all risk models for organization.
    pub async fn list_risk_models<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
    ) -> Result<Vec<AiRiskScoringModel>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            "SELECT * FROM ai_risk_scoring_models WHERE organization_id = $1 ORDER BY created_at DESC",
        )
        .bind(org_id)
        .fetch_all(executor)
        .await
    }

    /// Activate a risk model (deactivates others).
    ///
    /// Multi-statement (deactivate-all then activate-one), so it takes a
    /// connection and runs both writes inside a transaction on that
    /// RLS-context-bearing connection.
    pub async fn activate_risk_model(
        &self,
        conn: &mut PgConnection,
        org_id: Uuid,
        id: Uuid,
    ) -> Result<(), sqlx::Error> {
        let mut tx = conn.begin().await?;

        // Deactivate all models for org
        sqlx::query(
            "UPDATE ai_risk_scoring_models SET is_active = false WHERE organization_id = $1",
        )
        .bind(org_id)
        .execute(&mut *tx)
        .await?;

        // Activate the specified model
        sqlx::query(
            "UPDATE ai_risk_scoring_models SET is_active = true WHERE id = $1 AND organization_id = $2",
        )
        .bind(id)
        .bind(org_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await
    }

    /// Delete a risk model.
    ///
    /// RLS scopes the delete to the caller's org; a foreign id matches no row.
    pub async fn delete_risk_model<'e, E>(&self, executor: E, id: Uuid) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query("DELETE FROM ai_risk_scoring_models WHERE id = $1")
            .bind(id)
            .execute(executor)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    // =========================================================================
    // Screening Provider Configs
    // =========================================================================

    /// Create screening provider config.
    pub async fn create_provider_config<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        req: CreateScreeningProviderConfig,
    ) -> Result<ScreeningProviderConfig, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Note: In production, encrypt api_key and api_secret before storing
        sqlx::query_as(
            r#"
            INSERT INTO screening_provider_configs (
                organization_id, provider_name, provider_type, api_endpoint,
                rate_limit_per_hour, priority, cost_per_check
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(&req.provider_name)
        .bind(req.provider_type)
        .bind(&req.api_endpoint)
        .bind(req.rate_limit_per_hour.unwrap_or(100))
        .bind(req.priority.unwrap_or(1))
        .bind(req.cost_per_check)
        .fetch_one(executor)
        .await
    }

    /// Get provider config by ID, scoped to the owning organization.
    ///
    /// Issue #834: provider configs hold third-party integration settings
    /// (endpoints, rate limits, cost). Scope by org so a foreign caller cannot
    /// read another tenant's provider wiring by UUID.
    pub async fn get_provider_config<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        id: Uuid,
    ) -> Result<Option<ScreeningProviderConfig>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            "SELECT * FROM screening_provider_configs WHERE id = $1 AND organization_id = $2",
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    /// List provider configs for organization.
    pub async fn list_provider_configs<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
    ) -> Result<Vec<ScreeningProviderConfig>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            "SELECT * FROM screening_provider_configs WHERE organization_id = $1 ORDER BY priority",
        )
        .bind(org_id)
        .fetch_all(executor)
        .await
    }

    /// List active provider configs by type.
    pub async fn list_active_providers_by_type<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        provider_type: ScreeningProviderType,
    ) -> Result<Vec<ScreeningProviderConfig>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT * FROM screening_provider_configs
            WHERE organization_id = $1 AND provider_type = $2 AND status = 'active'
            ORDER BY priority
            "#,
        )
        .bind(org_id)
        .bind(provider_type)
        .fetch_all(executor)
        .await
    }

    /// Update provider status.
    ///
    /// RLS scopes the update to the caller's org; a foreign id matches no row.
    pub async fn update_provider_status<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        status: ProviderIntegrationStatus,
        error_message: Option<&str>,
    ) -> Result<(), sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query(
            r#"
            UPDATE screening_provider_configs
            SET status = $2, error_message = $3, last_health_check = NOW(), updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(status)
        .bind(error_message)
        .execute(executor)
        .await?;
        Ok(())
    }

    /// Delete provider config.
    ///
    /// RLS scopes the delete to the caller's org; a foreign id matches no row.
    pub async fn delete_provider_config<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query("DELETE FROM screening_provider_configs WHERE id = $1")
            .bind(id)
            .execute(executor)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    // =========================================================================
    // AI Screening Results
    // =========================================================================

    /// Create AI screening result.
    pub async fn create_ai_result<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        screening_id: Uuid,
        model: &AiRiskScoringModel,
        component_scores: ComponentScores,
    ) -> Result<ScreeningAiResult, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Calculate weighted score
        let ai_risk_score = calculate_weighted_score(model, &component_scores);
        let risk_category = AiRiskCategory::from_score(ai_risk_score, model);
        let recommendation = get_recommendation(ai_risk_score, model);

        sqlx::query_as(
            r#"
            INSERT INTO screening_ai_results (
                screening_id, organization_id, model_id, ai_risk_score, risk_category,
                credit_score_component, rental_history_component, income_stability_component,
                employment_stability_component, eviction_history_component, criminal_background_component,
                identity_verification_component, reference_quality_component,
                recommendation, recommendation_reason
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            RETURNING *
            "#,
        )
        .bind(screening_id)
        .bind(org_id)
        .bind(model.id)
        .bind(ai_risk_score)
        .bind(risk_category)
        .bind(component_scores.credit_history)
        .bind(component_scores.rental_history)
        .bind(component_scores.income_stability)
        .bind(component_scores.employment_stability)
        .bind(component_scores.eviction_history)
        .bind(component_scores.criminal_background)
        .bind(component_scores.identity_verification)
        .bind(component_scores.reference_quality)
        .bind(&recommendation)
        .bind(get_recommendation_reason(ai_risk_score, &risk_category))
        .fetch_one(executor)
        .await
    }

    /// Get the AI result for a screening, scoped to the owning organization.
    ///
    /// Issue #834 (cross-tenant PII): AI results carry risk scores and
    /// recommendations. Scoping by `organization_id` closes the IDOR where any
    /// org could read another org's result by guessing the screening UUID.
    pub async fn get_ai_result_by_screening<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        screening_id: Uuid,
    ) -> Result<Option<ScreeningAiResult>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            "SELECT * FROM screening_ai_results WHERE screening_id = $1 AND organization_id = $2",
        )
        .bind(screening_id)
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    /// Get AI result by ID.
    pub async fn get_ai_result<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<ScreeningAiResult>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as("SELECT * FROM screening_ai_results WHERE id = $1")
            .bind(id)
            .fetch_optional(executor)
            .await
    }

    /// List AI results for organization.
    pub async fn list_ai_results<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<ScreeningAiResult>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT * FROM screening_ai_results
            WHERE organization_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(org_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
    }

    // =========================================================================
    // Risk Factors
    // =========================================================================

    /// Create risk factor.
    pub async fn create_risk_factor<'e, E>(
        &self,
        executor: E,
        ai_result_id: Uuid,
        req: CreateScreeningRiskFactor,
    ) -> Result<ScreeningRiskFactor, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO screening_risk_factors (
                ai_result_id, category, factor_name, factor_description,
                impact, score_impact, source_provider, source_data,
                is_compliant, compliance_notes
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#,
        )
        .bind(ai_result_id)
        .bind(req.category)
        .bind(&req.factor_name)
        .bind(&req.factor_description)
        .bind(req.impact)
        .bind(req.score_impact)
        .bind(&req.source_provider)
        .bind(&req.source_data)
        .bind(req.is_compliant)
        .bind(&req.compliance_notes)
        .fetch_one(executor)
        .await
    }

    /// Get risk factors for AI result.
    pub async fn get_risk_factors<'e, E>(
        &self,
        executor: E,
        ai_result_id: Uuid,
    ) -> Result<Vec<ScreeningRiskFactor>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            "SELECT * FROM screening_risk_factors WHERE ai_result_id = $1 ORDER BY score_impact DESC",
        )
        .bind(ai_result_id)
        .fetch_all(executor)
        .await
    }

    /// Get negative/critical risk factors.
    pub async fn get_negative_risk_factors<'e, E>(
        &self,
        executor: E,
        ai_result_id: Uuid,
    ) -> Result<Vec<ScreeningRiskFactor>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT * FROM screening_risk_factors
            WHERE ai_result_id = $1 AND impact IN ('negative', 'critical')
            ORDER BY score_impact
            "#,
        )
        .bind(ai_result_id)
        .fetch_all(executor)
        .await
    }

    // =========================================================================
    // Credit Results
    // =========================================================================

    /// Create credit result.
    pub async fn create_credit_result<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        req: CreateScreeningCreditResult,
    ) -> Result<ScreeningCreditResult, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO screening_credit_results (
                screening_id, organization_id, provider_config_id, provider_name,
                credit_score, score_model, score_date, total_accounts, open_accounts,
                delinquent_accounts, collections_count, total_debt, available_credit,
                utilization_ratio, on_time_payments_pct, late_30_days_count,
                late_60_days_count, late_90_plus_count, bankruptcies_count,
                most_recent_bankruptcy_date, public_records_count
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21)
            RETURNING *
            "#,
        )
        .bind(req.screening_id)
        .bind(org_id)
        .bind(req.provider_config_id)
        .bind(&req.provider_name)
        .bind(req.credit_score)
        .bind(&req.score_model)
        .bind(req.score_date)
        .bind(req.total_accounts)
        .bind(req.open_accounts)
        .bind(req.delinquent_accounts)
        .bind(req.collections_count)
        .bind(req.total_debt)
        .bind(req.available_credit)
        .bind(req.utilization_ratio)
        .bind(req.on_time_payments_pct)
        .bind(req.late_30_days_count)
        .bind(req.late_60_days_count)
        .bind(req.late_90_plus_count)
        .bind(req.bankruptcies_count)
        .bind(req.most_recent_bankruptcy_date)
        .bind(req.public_records_count)
        .fetch_one(executor)
        .await
    }

    /// Get the credit result for a screening, scoped to the owning organization.
    ///
    /// Issue #834: credit results are sensitive PII (FICO scores, accounts).
    pub async fn get_credit_result_by_screening<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        screening_id: Uuid,
    ) -> Result<Option<ScreeningCreditResult>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            "SELECT * FROM screening_credit_results WHERE screening_id = $1 AND organization_id = $2",
        )
        .bind(screening_id)
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    // =========================================================================
    // Background Results
    // =========================================================================

    /// Create background result.
    pub async fn create_background_result<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        req: CreateScreeningBackgroundResult,
    ) -> Result<ScreeningBackgroundResult, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO screening_background_results (
                screening_id, organization_id, provider_config_id, provider_name,
                has_criminal_records, felony_count, misdemeanor_count, most_recent_offense_date,
                offense_categories, sex_offender_check_passed, watchlist_check_passed,
                identity_verified, identity_match_score, ssn_verified, address_history_verified
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            RETURNING *
            "#,
        )
        .bind(req.screening_id)
        .bind(org_id)
        .bind(req.provider_config_id)
        .bind(&req.provider_name)
        .bind(req.has_criminal_records)
        .bind(req.felony_count)
        .bind(req.misdemeanor_count)
        .bind(req.most_recent_offense_date)
        .bind(&req.offense_categories)
        .bind(req.sex_offender_check_passed)
        .bind(req.watchlist_check_passed)
        .bind(req.identity_verified)
        .bind(req.identity_match_score)
        .bind(req.ssn_verified)
        .bind(req.address_history_verified)
        .fetch_one(executor)
        .await
    }

    /// Get background result by screening ID, scoped to the owning organization.
    ///
    /// Issue #834: background results are sensitive PII (criminal records,
    /// identity verification).
    pub async fn get_background_result_by_screening<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        screening_id: Uuid,
    ) -> Result<Option<ScreeningBackgroundResult>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            "SELECT * FROM screening_background_results WHERE screening_id = $1 AND organization_id = $2",
        )
        .bind(screening_id)
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    // =========================================================================
    // Eviction Results
    // =========================================================================

    /// Create eviction result.
    pub async fn create_eviction_result<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        req: CreateScreeningEvictionResult,
    ) -> Result<ScreeningEvictionResult, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO screening_eviction_results (
                screening_id, organization_id, provider_config_id, provider_name,
                has_eviction_records, eviction_count, most_recent_eviction_date,
                eviction_records, unlawful_detainer_count
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(req.screening_id)
        .bind(org_id)
        .bind(req.provider_config_id)
        .bind(&req.provider_name)
        .bind(req.has_eviction_records)
        .bind(req.eviction_count)
        .bind(req.most_recent_eviction_date)
        .bind(&req.eviction_records)
        .bind(req.unlawful_detainer_count)
        .fetch_one(executor)
        .await
    }

    /// Get eviction result by screening ID, scoped to the owning organization.
    ///
    /// Issue #834: eviction history is sensitive PII.
    pub async fn get_eviction_result_by_screening<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        screening_id: Uuid,
    ) -> Result<Option<ScreeningEvictionResult>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            "SELECT * FROM screening_eviction_results WHERE screening_id = $1 AND organization_id = $2",
        )
        .bind(screening_id)
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    // =========================================================================
    // Request Queue
    // =========================================================================

    /// Create queue item.
    pub async fn create_queue_item<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        req: CreateScreeningQueueItem,
    ) -> Result<ScreeningRequestQueueItem, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO screening_request_queue (
                screening_id, organization_id, check_type, provider_config_id, priority
            )
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(req.screening_id)
        .bind(org_id)
        .bind(req.check_type)
        .bind(req.provider_config_id)
        .bind(req.priority.unwrap_or(5))
        .fetch_one(executor)
        .await
    }

    /// Get pending queue items.
    ///
    /// RLS scopes the rows to the caller's org under FORCE-RLS.
    pub async fn get_pending_queue_items<'e, E>(
        &self,
        executor: E,
        limit: i32,
    ) -> Result<Vec<ScreeningRequestQueueItem>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT * FROM screening_request_queue
            WHERE status = 'pending' OR (status = 'retry' AND next_retry_at <= NOW())
            ORDER BY priority, created_at
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(executor)
        .await
    }

    /// Update queue item status.
    ///
    /// Exactly one branch runs (one statement), so a generic executor suffices.
    /// RLS scopes the update to the caller's org; a foreign id matches no row.
    pub async fn update_queue_item_status<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        status: &str,
        error: Option<&str>,
    ) -> Result<(), sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        match status {
            "processing" => {
                sqlx::query(
                    "UPDATE screening_request_queue SET status = $2, started_at = NOW(), updated_at = NOW() WHERE id = $1",
                )
                .bind(id)
                .bind(status)
                .execute(executor)
                .await?;
            }
            "completed" => {
                sqlx::query(
                    "UPDATE screening_request_queue SET status = $2, completed_at = NOW(), updated_at = NOW() WHERE id = $1",
                )
                .bind(id)
                .bind(status)
                .execute(executor)
                .await?;
            }
            "failed" | "retry" => {
                sqlx::query(
                    r#"
                    UPDATE screening_request_queue
                    SET status = $2, last_error = $3, attempt_count = attempt_count + 1,
                        next_retry_at = NOW() + INTERVAL '5 minutes', updated_at = NOW()
                    WHERE id = $1
                    "#,
                )
                .bind(id)
                .bind(status)
                .bind(error)
                .execute(executor)
                .await?;
            }
            _ => {
                sqlx::query(
                    "UPDATE screening_request_queue SET status = $2, updated_at = NOW() WHERE id = $1",
                )
                .bind(id)
                .bind(status)
                .execute(executor)
                .await?;
            }
        }
        Ok(())
    }

    // =========================================================================
    // Reports
    // =========================================================================

    /// Create screening report.
    pub async fn create_report<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        user_id: Uuid,
        req: CreateScreeningReport,
    ) -> Result<ScreeningReport, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO screening_reports (
                screening_id, organization_id, report_type, generated_by,
                file_url, file_size_bytes, content_hash
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(req.screening_id)
        .bind(org_id)
        .bind(&req.report_type)
        .bind(user_id)
        .bind(&req.file_url)
        .bind(req.file_size_bytes)
        .bind(&req.content_hash)
        .fetch_one(executor)
        .await
    }

    /// Get reports for a screening, scoped to the owning organization.
    ///
    /// Issue #834: screening reports aggregate the sensitive PII above.
    pub async fn get_reports_by_screening<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        screening_id: Uuid,
    ) -> Result<Vec<ScreeningReport>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            "SELECT * FROM screening_reports WHERE screening_id = $1 AND organization_id = $2 AND deleted_at IS NULL ORDER BY generated_at DESC",
        )
        .bind(screening_id)
        .bind(org_id)
        .fetch_all(executor)
        .await
    }

    // =========================================================================
    // Statistics
    // =========================================================================

    /// Get screening statistics for organization.
    pub async fn get_statistics<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
    ) -> Result<ScreeningStatistics, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT
                COUNT(*) as total_screenings,
                COUNT(*) FILTER (WHERE s.status IN ('pending_consent', 'pending', 'processing')) as pending_screenings,
                COUNT(*) FILTER (WHERE s.status = 'completed') as completed_screenings,
                AVG(ar.ai_risk_score)::decimal as average_risk_score,
                (COUNT(*) FILTER (WHERE ar.recommendation = 'approve') * 100.0 / NULLIF(COUNT(ar.id), 0))::decimal as approval_rate,
                AVG(EXTRACT(EPOCH FROM (s.completed_at - s.started_at)) / 3600)::decimal as avg_processing_time_hours
            FROM tenant_screenings s
            LEFT JOIN screening_ai_results ar ON ar.screening_id = s.id
            WHERE s.organization_id = $1
            "#,
        )
        .bind(org_id)
        .fetch_one(executor)
        .await
    }

    /// Get risk category distribution.
    pub async fn get_risk_distribution<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
    ) -> Result<Vec<RiskCategoryDistribution>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            WITH totals AS (
                SELECT COUNT(*) as total FROM screening_ai_results WHERE organization_id = $1
            )
            SELECT
                risk_category as category,
                COUNT(*) as count,
                (COUNT(*) * 100.0 / NULLIF(t.total, 0))::decimal as percentage
            FROM screening_ai_results ar, totals t
            WHERE ar.organization_id = $1
            GROUP BY risk_category, t.total
            ORDER BY count DESC
            "#,
        )
        .bind(org_id)
        .fetch_all(executor)
        .await
    }

    /// Get complete screening data, scoped to the owning organization.
    ///
    /// Issue #834: this aggregates every sensitive screening artefact, so each
    /// component lookup is org-scoped. `get_risk_factors` keys off the
    /// AI-result id, which is itself org-scoped above, so it needs no separate
    /// org predicate.
    ///
    /// Multi-call: composes the per-artefact helpers above, so it takes a
    /// connection and reborrows `&mut *conn` for each query.
    pub async fn get_complete_screening_data(
        &self,
        conn: &mut PgConnection,
        org_id: Uuid,
        screening_id: Uuid,
    ) -> Result<CompleteScreeningData, sqlx::Error> {
        let ai_result = self
            .get_ai_result_by_screening(&mut *conn, org_id, screening_id)
            .await?;
        let risk_factors = if let Some(ref ai) = ai_result {
            self.get_risk_factors(&mut *conn, ai.id).await?
        } else {
            vec![]
        };
        let credit_result = self
            .get_credit_result_by_screening(&mut *conn, org_id, screening_id)
            .await?;
        let background_result = self
            .get_background_result_by_screening(&mut *conn, org_id, screening_id)
            .await?;
        let eviction_result = self
            .get_eviction_result_by_screening(&mut *conn, org_id, screening_id)
            .await?;

        Ok(CompleteScreeningData {
            ai_result,
            risk_factors,
            credit_result,
            background_result,
            eviction_result,
        })
    }
}

// =============================================================================
// Helper Types and Functions
// =============================================================================

/// Component scores for AI risk calculation.
pub struct ComponentScores {
    pub credit_history: Option<i32>,
    pub rental_history: Option<i32>,
    pub income_stability: Option<i32>,
    pub employment_stability: Option<i32>,
    pub eviction_history: Option<i32>,
    pub criminal_background: Option<i32>,
    pub identity_verification: Option<i32>,
    pub reference_quality: Option<i32>,
}

/// Calculate weighted AI risk score from component scores.
fn calculate_weighted_score(model: &AiRiskScoringModel, scores: &ComponentScores) -> i32 {
    let mut total_weight = Decimal::ZERO;
    let mut weighted_sum = Decimal::ZERO;

    if let Some(score) = scores.credit_history {
        weighted_sum += Decimal::from(score) * model.credit_history_weight;
        total_weight += model.credit_history_weight;
    }
    if let Some(score) = scores.rental_history {
        weighted_sum += Decimal::from(score) * model.rental_history_weight;
        total_weight += model.rental_history_weight;
    }
    if let Some(score) = scores.income_stability {
        weighted_sum += Decimal::from(score) * model.income_stability_weight;
        total_weight += model.income_stability_weight;
    }
    if let Some(score) = scores.employment_stability {
        weighted_sum += Decimal::from(score) * model.employment_stability_weight;
        total_weight += model.employment_stability_weight;
    }
    if let Some(score) = scores.eviction_history {
        weighted_sum += Decimal::from(score) * model.eviction_history_weight;
        total_weight += model.eviction_history_weight;
    }
    if let Some(score) = scores.criminal_background {
        weighted_sum += Decimal::from(score) * model.criminal_background_weight;
        total_weight += model.criminal_background_weight;
    }
    if let Some(score) = scores.identity_verification {
        weighted_sum += Decimal::from(score) * model.identity_verification_weight;
        total_weight += model.identity_verification_weight;
    }
    if let Some(score) = scores.reference_quality {
        weighted_sum += Decimal::from(score) * model.reference_quality_weight;
        total_weight += model.reference_quality_weight;
    }

    if total_weight > Decimal::ZERO {
        let normalized = weighted_sum / total_weight;
        normalized.to_string().parse::<f64>().unwrap_or(0.0) as i32
    } else {
        50 // Default neutral score if no components
    }
}

/// Get recommendation based on score and model thresholds.
fn get_recommendation(score: i32, model: &AiRiskScoringModel) -> String {
    if let Some(auto_approve) = model.auto_approve_threshold {
        if score >= auto_approve {
            return "approve".to_string();
        }
    }
    if let Some(auto_reject) = model.auto_reject_threshold {
        if score <= auto_reject {
            return "reject".to_string();
        }
    }

    if score >= model.excellent_threshold {
        "approve".to_string()
    } else if score >= model.good_threshold {
        "conditional_approve".to_string()
    } else if score >= model.fair_threshold {
        "review_required".to_string()
    } else {
        "reject".to_string()
    }
}

/// Get recommendation reason text.
fn get_recommendation_reason(score: i32, category: &AiRiskCategory) -> Option<String> {
    Some(match category {
        AiRiskCategory::Excellent => {
            format!("Applicant demonstrates excellent creditworthiness with a score of {}. All key indicators are positive.", score)
        }
        AiRiskCategory::Good => {
            format!("Applicant shows good overall profile with a score of {}. Minor concerns may exist but overall risk is low.", score)
        }
        AiRiskCategory::Fair => {
            format!("Applicant has a moderate risk profile with a score of {}. Additional review or conditions recommended.", score)
        }
        AiRiskCategory::Poor => {
            format!("Applicant presents elevated risk with a score of {}. Significant concerns identified.", score)
        }
        AiRiskCategory::VeryPoor => {
            format!("Applicant shows high risk indicators with a score of {}. Multiple red flags identified.", score)
        }
    })
}
