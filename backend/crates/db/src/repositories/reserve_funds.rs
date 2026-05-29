//! Reserve Fund Management repository for Epic 141.

use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::reserve_funds::{
    CreateContributionSchedule, CreateFundComponent, CreateFundProjection, CreateInvestmentPolicy,
    CreateProjectionItem, CreateReserveFund, FundAlert, FundComponent, FundContributionSchedule,
    FundDashboard, FundHealthReport, FundInvestmentPolicy, FundProjection, FundProjectionItem,
    FundSummary, FundTransaction, FundTransactionType, FundTransferRequest, FundType,
    RecordFundTransaction, ReserveFund, TransactionQuery, UpdateContributionSchedule,
    UpdateFundComponent, UpdateReserveFund,
};

/// Repository for reserve fund operations.
#[derive(Clone)]
pub struct ReserveFundRepository {
    pool: PgPool,
}

impl ReserveFundRepository {
    /// Create a new repository instance.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ========================================================================
    // Reserve Funds CRUD
    // ========================================================================

    /// Create a new reserve fund.
    pub async fn create_fund(
        &self,
        org_id: Uuid,
        req: CreateReserveFund,
        created_by: Uuid,
    ) -> Result<ReserveFund, sqlx::Error> {
        sqlx::query_as::<_, ReserveFund>(
            r#"
            INSERT INTO reserve_funds (
                organization_id, building_id, name, description, fund_type,
                target_balance, minimum_balance, currency, created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(req.building_id)
        .bind(&req.name)
        .bind(&req.description)
        .bind(req.fund_type)
        .bind(req.target_balance)
        .bind(req.minimum_balance)
        .bind(req.currency.unwrap_or_else(|| "EUR".to_string()))
        .bind(created_by)
        .fetch_one(&self.pool)
        .await
    }

    /// Get a reserve fund by ID.
    pub async fn get_fund(&self, fund_id: Uuid) -> Result<Option<ReserveFund>, sqlx::Error> {
        sqlx::query_as::<_, ReserveFund>("SELECT * FROM reserve_funds WHERE id = $1")
            .bind(fund_id)
            .fetch_optional(&self.pool)
            .await
    }

    /// List all funds for an organization.
    pub async fn list_funds(
        &self,
        org_id: Uuid,
        fund_type: Option<FundType>,
        building_id: Option<Uuid>,
        active_only: bool,
    ) -> Result<Vec<ReserveFund>, sqlx::Error> {
        let mut query = String::from("SELECT * FROM reserve_funds WHERE organization_id = $1");
        let mut param_count = 1;

        if fund_type.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND fund_type = ${}", param_count));
        }

        if building_id.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND building_id = ${}", param_count));
        }

        if active_only {
            query.push_str(" AND is_active = true");
        }

        query.push_str(" ORDER BY name");

        let mut q = sqlx::query_as::<_, ReserveFund>(sqlx::AssertSqlSafe(query)).bind(org_id);

        if let Some(ft) = fund_type {
            q = q.bind(ft);
        }

        if let Some(bid) = building_id {
            q = q.bind(bid);
        }

        q.fetch_all(&self.pool).await
    }

    /// Update a reserve fund.
    pub async fn update_fund(
        &self,
        fund_id: Uuid,
        req: UpdateReserveFund,
    ) -> Result<ReserveFund, sqlx::Error> {
        sqlx::query_as::<_, ReserveFund>(
            r#"
            UPDATE reserve_funds SET
                name = COALESCE($2, name),
                description = COALESCE($3, description),
                fund_type = COALESCE($4, fund_type),
                target_balance = COALESCE($5, target_balance),
                minimum_balance = COALESCE($6, minimum_balance),
                is_active = COALESCE($7, is_active),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(fund_id)
        .bind(req.name)
        .bind(req.description)
        .bind(req.fund_type)
        .bind(req.target_balance)
        .bind(req.minimum_balance)
        .bind(req.is_active)
        .fetch_one(&self.pool)
        .await
    }

    /// Delete a reserve fund.
    pub async fn delete_fund(&self, fund_id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM reserve_funds WHERE id = $1")
            .bind(fund_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    // ========================================================================
    // Contribution Schedules
    // ========================================================================

    /// Create a contribution schedule.
    pub async fn create_contribution_schedule(
        &self,
        fund_id: Uuid,
        req: CreateContributionSchedule,
    ) -> Result<FundContributionSchedule, sqlx::Error> {
        sqlx::query_as::<_, FundContributionSchedule>(
            r#"
            INSERT INTO fund_contribution_schedules (
                fund_id, name, description, amount, frequency,
                start_date, end_date, next_due_date, auto_collect
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $6, $8)
            RETURNING *
            "#,
        )
        .bind(fund_id)
        .bind(&req.name)
        .bind(&req.description)
        .bind(req.amount)
        .bind(req.frequency)
        .bind(req.start_date)
        .bind(req.end_date)
        .bind(req.auto_collect.unwrap_or(false))
        .fetch_one(&self.pool)
        .await
    }

    /// List contribution schedules for a fund.
    pub async fn list_contribution_schedules(
        &self,
        fund_id: Uuid,
        active_only: bool,
    ) -> Result<Vec<FundContributionSchedule>, sqlx::Error> {
        let query = if active_only {
            "SELECT * FROM fund_contribution_schedules WHERE fund_id = $1 AND is_active = true ORDER BY next_due_date"
        } else {
            "SELECT * FROM fund_contribution_schedules WHERE fund_id = $1 ORDER BY next_due_date"
        };

        sqlx::query_as::<_, FundContributionSchedule>(query)
            .bind(fund_id)
            .fetch_all(&self.pool)
            .await
    }

    /// Update a contribution schedule.
    pub async fn update_contribution_schedule(
        &self,
        schedule_id: Uuid,
        req: UpdateContributionSchedule,
    ) -> Result<FundContributionSchedule, sqlx::Error> {
        sqlx::query_as::<_, FundContributionSchedule>(
            r#"
            UPDATE fund_contribution_schedules SET
                name = COALESCE($2, name),
                description = COALESCE($3, description),
                amount = COALESCE($4, amount),
                frequency = COALESCE($5, frequency),
                end_date = COALESCE($6, end_date),
                is_active = COALESCE($7, is_active),
                auto_collect = COALESCE($8, auto_collect),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(schedule_id)
        .bind(req.name)
        .bind(req.description)
        .bind(req.amount)
        .bind(req.frequency)
        .bind(req.end_date)
        .bind(req.is_active)
        .bind(req.auto_collect)
        .fetch_one(&self.pool)
        .await
    }

    /// Delete a contribution schedule.
    pub async fn delete_contribution_schedule(
        &self,
        schedule_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM fund_contribution_schedules WHERE id = $1")
            .bind(schedule_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    // ========================================================================
    // Transactions
    // ========================================================================

    /// Record a fund transaction.
    pub async fn record_transaction(
        &self,
        fund_id: Uuid,
        req: RecordFundTransaction,
        created_by: Uuid,
    ) -> Result<FundTransaction, sqlx::Error> {
        // Get current balance
        let fund = self
            .get_fund(fund_id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)?;

        // Calculate new balance
        let amount = req.amount;
        let new_balance = match req.transaction_type {
            FundTransactionType::Contribution
            | FundTransactionType::Interest
            | FundTransactionType::Dividend
            | FundTransactionType::OpeningBalance => fund.current_balance + amount,
            FundTransactionType::Withdrawal | FundTransactionType::Fee => {
                fund.current_balance - amount
            }
            FundTransactionType::Transfer => {
                if req.transfer_to_fund_id.is_some() {
                    fund.current_balance - amount
                } else {
                    fund.current_balance + amount
                }
            }
            FundTransactionType::Adjustment => fund.current_balance + amount, // Can be positive or negative
        };

        // Create transaction
        let transaction = sqlx::query_as::<_, FundTransaction>(
            r#"
            INSERT INTO fund_transactions (
                fund_id, transaction_type, amount, balance_after,
                description, reference_number, contribution_schedule_id,
                transfer_to_fund_id, requires_approval, created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#,
        )
        .bind(fund_id)
        .bind(req.transaction_type)
        .bind(amount)
        .bind(new_balance)
        .bind(&req.description)
        .bind(&req.reference_number)
        .bind(req.contribution_schedule_id)
        .bind(req.transfer_to_fund_id)
        .bind(req.requires_approval.unwrap_or(false))
        .bind(created_by)
        .fetch_one(&self.pool)
        .await?;

        // Update fund balance
        sqlx::query(
            "UPDATE reserve_funds SET current_balance = $2, updated_at = NOW() WHERE id = $1",
        )
        .bind(fund_id)
        .bind(new_balance)
        .execute(&self.pool)
        .await?;

        Ok(transaction)
    }

    /// List transactions for a fund.
    pub async fn list_transactions(
        &self,
        query: TransactionQuery,
    ) -> Result<Vec<FundTransaction>, sqlx::Error> {
        let limit = query.limit.unwrap_or(100);
        let offset = query.offset.unwrap_or(0);

        sqlx::query_as::<_, FundTransaction>(
            r#"
            SELECT * FROM fund_transactions
            WHERE ($1::UUID IS NULL OR fund_id = $1)
              AND ($2::fund_transaction_type IS NULL OR transaction_type = $2)
              AND ($3::TIMESTAMPTZ IS NULL OR transaction_date >= $3)
              AND ($4::TIMESTAMPTZ IS NULL OR transaction_date <= $4)
            ORDER BY transaction_date DESC
            LIMIT $5 OFFSET $6
            "#,
        )
        .bind(query.fund_id)
        .bind(query.transaction_type)
        .bind(query.from_date)
        .bind(query.to_date)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    /// Transfer between funds.
    pub async fn transfer_funds(
        &self,
        req: FundTransferRequest,
        created_by: Uuid,
    ) -> Result<(FundTransaction, FundTransaction), sqlx::Error> {
        // Record withdrawal from source
        let withdrawal = self
            .record_transaction(
                req.from_fund_id,
                RecordFundTransaction {
                    transaction_type: FundTransactionType::Transfer,
                    amount: req.amount,
                    description: req.description.clone(),
                    reference_number: None,
                    contribution_schedule_id: None,
                    transfer_to_fund_id: Some(req.to_fund_id),
                    requires_approval: None,
                },
                created_by,
            )
            .await?;

        // Record deposit to destination
        let deposit = self
            .record_transaction(
                req.to_fund_id,
                RecordFundTransaction {
                    transaction_type: FundTransactionType::Transfer,
                    amount: req.amount,
                    description: req.description,
                    reference_number: None,
                    contribution_schedule_id: None,
                    transfer_to_fund_id: None,
                    requires_approval: None,
                },
                created_by,
            )
            .await?;

        Ok((withdrawal, deposit))
    }

    // ========================================================================
    // Investment Policies
    // ========================================================================

    /// Create an investment policy.
    pub async fn create_investment_policy(
        &self,
        fund_id: Uuid,
        req: CreateInvestmentPolicy,
    ) -> Result<FundInvestmentPolicy, sqlx::Error> {
        sqlx::query_as::<_, FundInvestmentPolicy>(
            r#"
            INSERT INTO fund_investment_policies (
                fund_id, name, description, risk_level,
                cash_allocation_pct, bonds_allocation_pct,
                money_market_allocation_pct, other_allocation_pct,
                max_single_investment, min_liquidity_pct, effective_date
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#,
        )
        .bind(fund_id)
        .bind(&req.name)
        .bind(&req.description)
        .bind(req.risk_level)
        .bind(req.cash_allocation_pct)
        .bind(req.bonds_allocation_pct)
        .bind(req.money_market_allocation_pct)
        .bind(req.other_allocation_pct)
        .bind(req.max_single_investment)
        .bind(req.min_liquidity_pct)
        .bind(req.effective_date)
        .fetch_one(&self.pool)
        .await
    }

    /// Get active investment policy for a fund.
    pub async fn get_active_investment_policy(
        &self,
        fund_id: Uuid,
    ) -> Result<Option<FundInvestmentPolicy>, sqlx::Error> {
        sqlx::query_as::<_, FundInvestmentPolicy>(
            r#"
            SELECT * FROM fund_investment_policies
            WHERE fund_id = $1 AND is_active = true
              AND effective_date <= CURRENT_DATE
              AND (end_date IS NULL OR end_date > CURRENT_DATE)
            ORDER BY effective_date DESC
            LIMIT 1
            "#,
        )
        .bind(fund_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// List investment policies for a fund.
    pub async fn list_investment_policies(
        &self,
        fund_id: Uuid,
    ) -> Result<Vec<FundInvestmentPolicy>, sqlx::Error> {
        sqlx::query_as::<_, FundInvestmentPolicy>(
            "SELECT * FROM fund_investment_policies WHERE fund_id = $1 ORDER BY effective_date DESC",
        )
        .bind(fund_id)
        .fetch_all(&self.pool)
        .await
    }

    // ========================================================================
    // Projections (Reserve Studies)
    // ========================================================================

    /// Create a fund projection.
    pub async fn create_projection(
        &self,
        fund_id: Uuid,
        req: CreateFundProjection,
    ) -> Result<FundProjection, sqlx::Error> {
        // Get current balance for starting balance
        let fund = self
            .get_fund(fund_id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)?;

        // Mark existing projections as not current
        sqlx::query("UPDATE fund_projections SET is_current = false WHERE fund_id = $1")
            .bind(fund_id)
            .execute(&self.pool)
            .await?;

        sqlx::query_as::<_, FundProjection>(
            r#"
            INSERT INTO fund_projections (
                fund_id, study_name, study_date, projection_years,
                annual_inflation_rate, annual_interest_rate,
                starting_balance, recommended_annual_contribution, prepared_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(fund_id)
        .bind(&req.study_name)
        .bind(req.study_date)
        .bind(req.projection_years)
        .bind(req.annual_inflation_rate.unwrap_or(Decimal::new(25, 1)))
        .bind(req.annual_interest_rate.unwrap_or(Decimal::new(15, 1)))
        .bind(fund.current_balance)
        .bind(req.recommended_annual_contribution)
        .bind(&req.prepared_by)
        .fetch_one(&self.pool)
        .await
    }

    /// Get current projection for a fund.
    pub async fn get_current_projection(
        &self,
        fund_id: Uuid,
    ) -> Result<Option<FundProjection>, sqlx::Error> {
        sqlx::query_as::<_, FundProjection>(
            "SELECT * FROM fund_projections WHERE fund_id = $1 AND is_current = true LIMIT 1",
        )
        .bind(fund_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Add projection line items.
    pub async fn add_projection_items(
        &self,
        projection_id: Uuid,
        items: Vec<CreateProjectionItem>,
    ) -> Result<Vec<FundProjectionItem>, sqlx::Error> {
        let mut results = Vec::new();

        for item in items {
            let result = sqlx::query_as::<_, FundProjectionItem>(
                r#"
                INSERT INTO fund_projection_items (
                    projection_id, projection_year, fiscal_year,
                    contributions, interest_income, planned_expenditures,
                    beginning_balance, ending_balance, expenditure_details
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                RETURNING *
                "#,
            )
            .bind(projection_id)
            .bind(item.projection_year)
            .bind(item.fiscal_year)
            .bind(item.contributions)
            .bind(item.interest_income)
            .bind(item.planned_expenditures)
            .bind(item.beginning_balance)
            .bind(item.ending_balance)
            .bind(&item.expenditure_details)
            .fetch_one(&self.pool)
            .await?;

            results.push(result);
        }

        Ok(results)
    }

    /// Get projection items.
    pub async fn get_projection_items(
        &self,
        projection_id: Uuid,
    ) -> Result<Vec<FundProjectionItem>, sqlx::Error> {
        sqlx::query_as::<_, FundProjectionItem>(
            "SELECT * FROM fund_projection_items WHERE projection_id = $1 ORDER BY projection_year",
        )
        .bind(projection_id)
        .fetch_all(&self.pool)
        .await
    }

    // ========================================================================
    // Components
    // ========================================================================

    /// Create a fund component.
    pub async fn create_component(
        &self,
        fund_id: Uuid,
        req: CreateFundComponent,
    ) -> Result<FundComponent, sqlx::Error> {
        sqlx::query_as::<_, FundComponent>(
            r#"
            INSERT INTO fund_components (
                fund_id, name, description, category,
                current_replacement_cost, useful_life_years,
                remaining_life_years, condition_rating,
                last_inspection_date, next_replacement_date
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#,
        )
        .bind(fund_id)
        .bind(&req.name)
        .bind(&req.description)
        .bind(&req.category)
        .bind(req.current_replacement_cost)
        .bind(req.useful_life_years)
        .bind(req.remaining_life_years)
        .bind(req.condition_rating)
        .bind(req.last_inspection_date)
        .bind(req.next_replacement_date)
        .fetch_one(&self.pool)
        .await
    }

    /// List components for a fund.
    pub async fn list_components(&self, fund_id: Uuid) -> Result<Vec<FundComponent>, sqlx::Error> {
        sqlx::query_as::<_, FundComponent>(
            "SELECT * FROM fund_components WHERE fund_id = $1 ORDER BY next_replacement_date NULLS LAST",
        )
        .bind(fund_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Update a component.
    pub async fn update_component(
        &self,
        component_id: Uuid,
        req: UpdateFundComponent,
    ) -> Result<FundComponent, sqlx::Error> {
        sqlx::query_as::<_, FundComponent>(
            r#"
            UPDATE fund_components SET
                name = COALESCE($2, name),
                description = COALESCE($3, description),
                category = COALESCE($4, category),
                current_replacement_cost = COALESCE($5, current_replacement_cost),
                useful_life_years = COALESCE($6, useful_life_years),
                remaining_life_years = COALESCE($7, remaining_life_years),
                condition_rating = COALESCE($8, condition_rating),
                last_inspection_date = COALESCE($9, last_inspection_date),
                next_replacement_date = COALESCE($10, next_replacement_date),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(component_id)
        .bind(req.name)
        .bind(req.description)
        .bind(req.category)
        .bind(req.current_replacement_cost)
        .bind(req.useful_life_years)
        .bind(req.remaining_life_years)
        .bind(req.condition_rating)
        .bind(req.last_inspection_date)
        .bind(req.next_replacement_date)
        .fetch_one(&self.pool)
        .await
    }

    /// Delete a component.
    pub async fn delete_component(&self, component_id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM fund_components WHERE id = $1")
            .bind(component_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    // ========================================================================
    // Alerts
    // ========================================================================

    /// Create a fund alert.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_alert(
        &self,
        fund_id: Uuid,
        alert_type: &str,
        severity: &str,
        title: &str,
        message: &str,
        threshold_value: Option<Decimal>,
        current_value: Option<Decimal>,
    ) -> Result<FundAlert, sqlx::Error> {
        sqlx::query_as::<_, FundAlert>(
            r#"
            INSERT INTO fund_alerts (
                fund_id, alert_type, severity, title, message,
                threshold_value, current_value
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(fund_id)
        .bind(alert_type)
        .bind(severity)
        .bind(title)
        .bind(message)
        .bind(threshold_value)
        .bind(current_value)
        .fetch_one(&self.pool)
        .await
    }

    /// List active alerts for an organization.
    pub async fn list_active_alerts(&self, org_id: Uuid) -> Result<Vec<FundAlert>, sqlx::Error> {
        sqlx::query_as::<_, FundAlert>(
            r#"
            SELECT fa.* FROM fund_alerts fa
            JOIN reserve_funds rf ON rf.id = fa.fund_id
            WHERE rf.organization_id = $1 AND fa.is_active = true
            ORDER BY fa.created_at DESC
            "#,
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Acknowledge an alert.
    pub async fn acknowledge_alert(
        &self,
        alert_id: Uuid,
        user_id: Uuid,
    ) -> Result<FundAlert, sqlx::Error> {
        sqlx::query_as::<_, FundAlert>(
            r#"
            UPDATE fund_alerts SET
                acknowledged_at = NOW(),
                acknowledged_by = $2
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(alert_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
    }

    /// Resolve an alert.
    pub async fn resolve_alert(
        &self,
        alert_id: Uuid,
        user_id: Uuid,
    ) -> Result<FundAlert, sqlx::Error> {
        sqlx::query_as::<_, FundAlert>(
            r#"
            UPDATE fund_alerts SET
                resolved_at = NOW(),
                resolved_by = $2,
                is_active = false
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(alert_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
    }

    // ========================================================================
    // Dashboard & Reports
    // ========================================================================

    /// Get fund dashboard for an organization.
    pub async fn get_fund_dashboard(&self, org_id: Uuid) -> Result<FundDashboard, sqlx::Error> {
        // Get all funds
        let funds = self.list_funds(org_id, None, None, true).await?;

        let mut total_balance = Decimal::ZERO;
        let mut total_target = Decimal::ZERO;
        let mut funds_below_min = 0i64;
        let mut fund_summaries = Vec::new();

        for fund in &funds {
            total_balance += fund.current_balance;

            if let Some(target) = fund.target_balance {
                total_target += target;
            }

            let funding_pct = fund.target_balance.map(|target| {
                if target > Decimal::ZERO {
                    (fund.current_balance / target) * Decimal::from(100)
                } else {
                    Decimal::from(100)
                }
            });

            let is_below_min = fund
                .minimum_balance
                .map(|min| fund.current_balance < min)
                .unwrap_or(false);

            if is_below_min {
                funds_below_min += 1;
            }

            // Get upcoming contributions
            let schedules = self.list_contribution_schedules(fund.id, true).await?;
            let upcoming = schedules.iter().map(|s| s.amount).sum();

            // Get active alerts count
            let alerts: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM fund_alerts WHERE fund_id = $1 AND is_active = true",
            )
            .bind(fund.id)
            .fetch_one(&self.pool)
            .await?;

            fund_summaries.push(FundSummary {
                id: fund.id,
                name: fund.name.clone(),
                fund_type: fund.fund_type,
                current_balance: fund.current_balance,
                target_balance: fund.target_balance,
                funding_percentage: funding_pct,
                is_below_minimum: is_below_min,
                upcoming_contributions: upcoming,
                active_alerts_count: alerts.0,
            });
        }

        let overall_pct = if total_target > Decimal::ZERO {
            (total_balance / total_target) * Decimal::from(100)
        } else {
            Decimal::from(100)
        };

        // Get total active alerts
        let total_alerts: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM fund_alerts fa
            JOIN reserve_funds rf ON rf.id = fa.fund_id
            WHERE rf.organization_id = $1 AND fa.is_active = true
            "#,
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(FundDashboard {
            total_fund_balance: total_balance,
            total_target_balance: total_target,
            overall_funding_percentage: overall_pct,
            funds_below_minimum: funds_below_min,
            active_alerts: total_alerts.0,
            funds: fund_summaries,
        })
    }

    /// Get fund health report.
    pub async fn get_fund_health_report(
        &self,
        fund_id: Uuid,
    ) -> Result<FundHealthReport, sqlx::Error> {
        let fund = self
            .get_fund(fund_id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)?;

        let mut issues = Vec::new();
        let mut recommendations = Vec::new();
        let mut health_score = 100;

        // Check minimum balance
        if let Some(min) = fund.minimum_balance {
            if fund.current_balance < min {
                issues.push(format!(
                    "Balance below minimum: {} < {}",
                    fund.current_balance, min
                ));
                health_score -= 20;
                recommendations.push("Increase contributions to reach minimum balance".to_string());
            }
        }

        // Check funding percentage
        let funding_pct = fund.target_balance.map(|target| {
            if target > Decimal::ZERO {
                (fund.current_balance / target) * Decimal::from(100)
            } else {
                Decimal::from(100)
            }
        });

        if let Some(pct) = funding_pct {
            if pct < Decimal::from(50) {
                issues.push(format!("Severely underfunded: {}%", pct));
                health_score -= 30;
                recommendations.push("Consider special assessment or increased dues".to_string());
            } else if pct < Decimal::from(70) {
                issues.push(format!("Underfunded: {}%", pct));
                health_score -= 15;
                recommendations.push("Review contribution schedule".to_string());
            }
        }

        // Check for current reserve study
        let projection = self.get_current_projection(fund_id).await?;
        if projection.is_none() {
            issues.push("No current reserve study".to_string());
            health_score -= 10;
            recommendations.push("Commission a reserve study".to_string());
        }

        // Check upcoming component replacements
        let components = self.list_components(fund_id).await?;
        let urgent_components: Vec<_> = components
            .iter()
            .filter(|c| c.remaining_life_years.map(|y| y <= 2).unwrap_or(false))
            .collect();

        if !urgent_components.is_empty() {
            issues.push(format!(
                "{} components need replacement within 2 years",
                urgent_components.len()
            ));
            health_score -= 10;
        }

        Ok(FundHealthReport {
            fund_id: fund.id,
            fund_name: fund.name,
            current_balance: fund.current_balance,
            target_balance: fund.target_balance,
            funding_status_pct: funding_pct,
            health_score: health_score.max(0),
            issues,
            recommendations,
        })
    }
}
