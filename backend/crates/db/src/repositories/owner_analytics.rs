//! Epic 74: Owner Investment Analytics repository.
//! Provides database operations for property valuations, ROI tracking, and expense approvals.

use crate::models::owner_analytics::*;
use crate::DbPool;
// Note: DateTime types are used from chrono
use common::errors::AppError;
use rust_decimal::Decimal;
use sqlx::Row;
use uuid::Uuid;

#[derive(Clone)]
pub struct OwnerAnalyticsRepository {
    pool: DbPool,
}

impl OwnerAnalyticsRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // ======================== Tenant-isolation guards ========================

    /// Verify that `unit_id` belongs to a building owned by `org_id`.
    /// Returns `AppError::NotFound` when the unit is absent or in another org —
    /// callers surface this as a 404 so cross-tenant probing cannot distinguish
    /// "wrong org" from "does not exist".
    async fn assert_unit_in_org(&self, unit_id: Uuid, org_id: Uuid) -> Result<(), AppError> {
        let exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM units u
                JOIN buildings b ON b.id = u.building_id
                WHERE u.id = $1 AND b.organization_id = $2
            )
            "#,
        )
        .bind(unit_id)
        .bind(org_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        if exists {
            Ok(())
        } else {
            Err(AppError::NotFound("Unit not found".to_string()))
        }
    }

    /// Verify that `valuation_id` belongs to a unit in a building owned by `org_id`.
    async fn assert_valuation_in_org(
        &self,
        valuation_id: Uuid,
        org_id: Uuid,
    ) -> Result<(), AppError> {
        let exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM property_valuations pv
                JOIN units u ON u.id = pv.unit_id
                JOIN buildings b ON b.id = u.building_id
                WHERE pv.id = $1 AND b.organization_id = $2
            )
            "#,
        )
        .bind(valuation_id)
        .bind(org_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        if exists {
            Ok(())
        } else {
            Err(AppError::NotFound("Valuation not found".to_string()))
        }
    }

    // ======================== Property Valuations (Story 74.1) ========================

    pub async fn create_valuation(
        &self,
        org_id: Uuid,
        req: CreatePropertyValuation,
    ) -> Result<PropertyValuation, AppError> {
        // Enforce tenant isolation: the unit must belong to the caller's org.
        self.assert_unit_in_org(req.unit_id, org_id).await?;

        // Calculate low and high values (±5%)
        let value_low = req.estimated_value * Decimal::new(95, 2);
        let value_high = req.estimated_value * Decimal::new(105, 2);

        let valuation = sqlx::query_as::<_, PropertyValuation>(
            r#"
            INSERT INTO property_valuations (unit_id, valuation_date, value_low, value_high,
                                             estimated_value, valuation_method, confidence_score, notes)
            VALUES ($1, CURRENT_DATE, $2, $3, $4, $5, 85, $6)
            RETURNING id, unit_id, valuation_date, value_low, value_high, estimated_value,
                      valuation_method, confidence_score, notes, created_at
            "#,
        )
        .bind(req.unit_id)
        .bind(value_low)
        .bind(value_high)
        .bind(req.estimated_value)
        .bind(&req.valuation_method)
        .bind(&req.notes)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        // Record in value history
        sqlx::query("INSERT INTO property_value_history (unit_id, value) VALUES ($1, $2)")
            .bind(req.unit_id)
            .bind(req.estimated_value)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(valuation)
    }

    pub async fn get_latest_valuation(
        &self,
        unit_id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<PropertyValuation>, AppError> {
        let valuation = sqlx::query_as::<_, PropertyValuation>(
            r#"
            SELECT pv.id, pv.unit_id, pv.valuation_date, pv.value_low, pv.value_high,
                   pv.estimated_value, pv.valuation_method, pv.confidence_score, pv.notes,
                   pv.created_at
            FROM property_valuations pv
            JOIN units u ON u.id = pv.unit_id
            JOIN buildings b ON b.id = u.building_id
            WHERE pv.unit_id = $1 AND b.organization_id = $2
            ORDER BY pv.valuation_date DESC
            LIMIT 1
            "#,
        )
        .bind(unit_id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(valuation)
    }

    pub async fn get_valuation_with_comparables(
        &self,
        valuation_id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<PropertyValuationWithComparables>, AppError> {
        let valuation = sqlx::query_as::<_, PropertyValuation>(
            r#"
            SELECT pv.id, pv.unit_id, pv.valuation_date, pv.value_low, pv.value_high,
                   pv.estimated_value, pv.valuation_method, pv.confidence_score, pv.notes,
                   pv.created_at
            FROM property_valuations pv
            JOIN units u ON u.id = pv.unit_id
            JOIN buildings b ON b.id = u.building_id
            WHERE pv.id = $1 AND b.organization_id = $2
            "#,
        )
        .bind(valuation_id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        match valuation {
            Some(v) => {
                let comparables = sqlx::query_as::<_, ComparableProperty>(
                    r#"
                    SELECT id, valuation_id, address, sale_price, sold_date, size_sqm,
                           rooms, distance_km, similarity_score, source, created_at
                    FROM comparable_properties
                    WHERE valuation_id = $1
                    ORDER BY similarity_score DESC
                    "#,
                )
                .bind(valuation_id)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;

                Ok(Some(PropertyValuationWithComparables {
                    valuation: v,
                    comparables,
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn add_comparable(
        &self,
        valuation_id: Uuid,
        org_id: Uuid,
        req: AddComparableProperty,
    ) -> Result<ComparableProperty, AppError> {
        // Enforce tenant isolation: the valuation must belong to the caller's org.
        self.assert_valuation_in_org(valuation_id, org_id).await?;

        let comparable = sqlx::query_as::<_, ComparableProperty>(
            r#"
            INSERT INTO comparable_properties (valuation_id, address, sale_price, size_sqm,
                                               rooms, distance_km, similarity_score, source)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, valuation_id, address, sale_price, sold_date, size_sqm,
                      rooms, distance_km, similarity_score, source, created_at
            "#,
        )
        .bind(valuation_id)
        .bind(&req.address)
        .bind(req.sale_price)
        .bind(req.size_sqm)
        .bind(req.rooms)
        .bind(req.distance_km)
        .bind(req.similarity_score)
        .bind(&req.source)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(comparable)
    }

    pub async fn get_value_history(
        &self,
        org_id: Uuid,
        q: ValueHistoryQuery,
    ) -> Result<Vec<PropertyValueHistory>, AppError> {
        let limit = q.limit.unwrap_or(12) as i64;

        let history = sqlx::query_as::<_, PropertyValueHistory>(
            r#"
            SELECT pvh.id, pvh.unit_id, pvh.value, pvh.recorded_at
            FROM property_value_history pvh
            JOIN units u ON u.id = pvh.unit_id
            JOIN buildings b ON b.id = u.building_id
            WHERE pvh.unit_id = $1 AND b.organization_id = $2
            ORDER BY pvh.recorded_at DESC
            LIMIT $3
            "#,
        )
        .bind(q.unit_id)
        .bind(org_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(history)
    }

    pub async fn get_value_trend(
        &self,
        unit_id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<ValueTrendAnalysis>, AppError> {
        let query = ValueHistoryQuery {
            unit_id,
            limit: Some(12),
        };
        let history = self.get_value_history(org_id, query).await?;

        if history.is_empty() {
            return Ok(None);
        }

        // Calculate trend percentage
        let trend_percent = if history.len() >= 2 {
            let latest = history.first().map(|h| h.value).unwrap_or(Decimal::ZERO);
            let oldest = history.last().map(|h| h.value).unwrap_or(Decimal::ZERO);
            if oldest.is_zero() {
                Decimal::ZERO
            } else {
                ((latest - oldest) / oldest * Decimal::from(100)).round_dp(1)
            }
        } else {
            Decimal::ZERO
        };

        Ok(Some(ValueTrendAnalysis {
            unit_id,
            history,
            trend_percent,
        }))
    }

    // ======================== ROI & Cash Flow (Story 74.2) ========================

    pub async fn calculate_roi(
        &self,
        org_id: Uuid,
        req: CalculateROIRequest,
    ) -> Result<PropertyROI, AppError> {
        // Enforce tenant isolation: the unit must belong to the caller's org.
        self.assert_unit_in_org(req.unit_id, org_id).await?;

        // Get total income for the period
        let total_income: Decimal = sqlx::query_scalar(
            r#"
            SELECT COALESCE(SUM(amount), 0)
            FROM invoices
            WHERE unit_id = $1 AND status = 'paid'
              AND issue_date >= $2 AND issue_date <= $3
            "#,
        )
        .bind(req.unit_id)
        .bind(req.from_date)
        .bind(req.to_date)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(Decimal::ZERO);

        // Get total expenses for the period
        let total_expenses: Decimal = sqlx::query_scalar(
            r#"
            SELECT COALESCE(SUM(amount), 0)
            FROM maintenance_requests mr
            WHERE mr.unit_id = $1 AND mr.status = 'completed'
              AND mr.created_at >= $2 AND mr.created_at <= $3
            "#,
        )
        .bind(req.unit_id)
        .bind(req.from_date)
        .bind(req.to_date)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(Decimal::ZERO);

        let total_returns = total_income - total_expenses;

        // Calculate ROI percentage
        let roi_pct = if req.total_investment.is_zero() {
            Decimal::ZERO
        } else {
            (total_returns / req.total_investment * Decimal::from(100)).round_dp(2)
        };

        // Calculate period in months
        let months = ((req.to_date - req.from_date).num_days() as i32 / 30).max(1);

        // Calculate annualized ROI
        let annualized = (roi_pct * Decimal::from(12) / Decimal::from(months)).round_dp(2);

        Ok(PropertyROI {
            unit_id: req.unit_id,
            total_investment: req.total_investment,
            total_returns,
            roi_percentage: roi_pct,
            annualized_roi: annualized,
            period_months: months,
        })
    }

    pub async fn get_cash_flow_breakdown(
        &self,
        unit_id: Uuid,
        org_id: Uuid,
        from: chrono::NaiveDate,
        to: chrono::NaiveDate,
    ) -> Result<CashFlowBreakdown, AppError> {
        // Enforce tenant isolation: the unit must belong to the caller's org.
        self.assert_unit_in_org(unit_id, org_id).await?;

        // Get rental income
        let rental_income: Decimal = sqlx::query_scalar(
            r#"
            SELECT COALESCE(SUM(amount), 0)
            FROM invoices
            WHERE unit_id = $1 AND status = 'paid'
              AND issue_date >= $2 AND issue_date <= $3
            "#,
        )
        .bind(unit_id)
        .bind(from)
        .bind(to)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(Decimal::ZERO);

        // Get maintenance expenses
        let maintenance: Decimal = sqlx::query_scalar(
            r#"
            SELECT COALESCE(SUM(COALESCE(actual_cost, estimated_cost)), 0)
            FROM maintenance_requests
            WHERE unit_id = $1 AND status = 'completed'
              AND created_at >= $2 AND created_at <= $3
            "#,
        )
        .bind(unit_id)
        .bind(from)
        .bind(to)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(Decimal::ZERO);

        let other_fees = Decimal::ZERO;
        let utilities = Decimal::ZERO;
        let insurance = Decimal::ZERO;
        let property_taxes = Decimal::ZERO;
        let management_fees = Decimal::ZERO;
        let other_expenses = Decimal::ZERO;

        let total_income = rental_income + other_fees;
        let total_expenses =
            maintenance + utilities + insurance + property_taxes + management_fees + other_expenses;
        let net_cash_flow = total_income - total_expenses;

        Ok(CashFlowBreakdown {
            unit_id,
            period_start: from,
            period_end: to,
            income: CashFlowIncome {
                rental_income,
                other_fees,
                total: total_income,
            },
            expenses: CashFlowExpenses {
                maintenance,
                utilities,
                insurance,
                property_taxes,
                management_fees,
                other: other_expenses,
                total: total_expenses,
            },
            net_cash_flow,
            cumulative_cash_flow: net_cash_flow,
        })
    }

    pub async fn get_roi_dashboard(
        &self,
        unit_id: Uuid,
        org_id: Uuid,
        from: chrono::NaiveDate,
        to: chrono::NaiveDate,
    ) -> Result<ROIDashboard, AppError> {
        // Get latest valuation for investment estimate
        let latest_valuation = self.get_latest_valuation(unit_id, org_id).await?;
        let investment = latest_valuation
            .as_ref()
            .map(|v| v.estimated_value)
            .unwrap_or(Decimal::new(150000, 0));

        // Calculate ROI
        let roi_request = CalculateROIRequest {
            unit_id,
            total_investment: investment,
            from_date: from,
            to_date: to,
        };
        let property_roi = self.calculate_roi(org_id, roi_request).await?;

        // Get cash flow breakdown
        let cash_flow_breakdown = self
            .get_cash_flow_breakdown(unit_id, org_id, from, to)
            .await?;

        // Get value trend
        let value_trend = self.get_value_trend(unit_id, org_id).await?;

        // Build dashboard
        Ok(ROIDashboard {
            property_roi,
            cash_flow: CashFlowSummary {
                total_income: cash_flow_breakdown.income.total,
                total_expenses: cash_flow_breakdown.expenses.total,
                net_cash_flow: cash_flow_breakdown.net_cash_flow,
            },
            value_trend: ValueTrendSummary {
                current_value: latest_valuation
                    .map(|v| v.estimated_value)
                    .unwrap_or(Decimal::ZERO),
                trend_percentage: value_trend.map(|t| t.trend_percent).unwrap_or_default(),
            },
        })
    }

    // ======================== Portfolio (Story 74.3) ========================

    pub async fn get_portfolio_summary(
        &self,
        q: OwnerPropertiesQuery,
    ) -> Result<PortfolioSummary, AppError> {
        // Get properties for the owner/organization
        let properties = sqlx::query(
            r#"
            SELECT u.id as unit_id,
                   COALESCE(u.name, b.address || ' #' || u.unit_number) as address,
                   COALESCE(
                       (SELECT estimated_value FROM property_valuations
                        WHERE unit_id = u.id ORDER BY valuation_date DESC LIMIT 1),
                       0
                   ) as current_value
            FROM units u
            JOIN buildings b ON u.building_id = b.id
            WHERE ($1::uuid IS NULL OR b.organization_id = $1)
            LIMIT 50
            "#,
        )
        .bind(q.organization_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let mut portfolio_properties = Vec::new();
        let mut total_value = Decimal::ZERO;
        let mut total_net_income = Decimal::ZERO;

        for row in properties {
            let unit_id: Uuid = row.get("unit_id");
            let address: String = row.get("address");
            let current_value: Decimal = row.get("current_value");

            // Simplified ROI calculation (would need actual data)
            let roi = Decimal::new(75, 1); // 7.5%
            let net_income = current_value * Decimal::new(75, 3) / Decimal::from(12); // Monthly

            portfolio_properties.push(PortfolioProperty {
                unit_id,
                address,
                current_value,
                roi,
                net_income,
            });

            total_value += current_value;
            total_net_income += net_income;
        }

        let total_roi = if total_value.is_zero() {
            Decimal::ZERO
        } else {
            (total_net_income * Decimal::from(12) / total_value * Decimal::from(100)).round_dp(1)
        };

        Ok(PortfolioSummary {
            properties: portfolio_properties,
            total_value,
            total_roi,
            total_net_income,
        })
    }

    pub async fn compare_properties(
        &self,
        org_id: Uuid,
        req: PortfolioComparisonRequest,
    ) -> Result<ComparisonMetrics, AppError> {
        let mut properties = Vec::new();
        let mut best_roi_unit = None;
        let mut best_roi = Decimal::MIN;
        let mut highest_value_unit = None;
        let mut highest_value = Decimal::MIN;

        for &unit_id in &req.unit_ids {
            // Enforce tenant isolation: every requested unit must belong to the
            // caller's org, otherwise a caller could probe cross-tenant values.
            self.assert_unit_in_org(unit_id, org_id).await?;

            // Get valuation
            let valuation = sqlx::query_scalar::<_, Decimal>(
                r#"
                SELECT estimated_value FROM property_valuations
                WHERE unit_id = $1
                ORDER BY valuation_date DESC
                LIMIT 1
                "#,
            )
            .bind(unit_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .unwrap_or(Decimal::ZERO);

            // Calculate simplified ROI and net income
            let roi = Decimal::new(75, 1); // 7.5%
            let net_income = valuation * Decimal::new(75, 3) / Decimal::from(12);

            if roi > best_roi {
                best_roi = roi;
                best_roi_unit = Some(unit_id);
            }

            if valuation > highest_value {
                highest_value = valuation;
                highest_value_unit = Some(unit_id);
            }

            properties.push(PropertyComparison {
                unit_id,
                value: valuation,
                roi,
                net_income,
            });
        }

        let default_unit = req.unit_ids.first().copied().unwrap_or_else(Uuid::new_v4);

        Ok(ComparisonMetrics {
            properties,
            best_roi_unit: best_roi_unit.unwrap_or(default_unit),
            highest_value_unit: highest_value_unit.unwrap_or(default_unit),
        })
    }

    // ======================== Expense Auto-Approval (Story 74.4) ========================

    pub async fn create_auto_approval_rule(
        &self,
        owner_id: Uuid,
        org_id: Uuid,
        req: CreateAutoApprovalRule,
    ) -> Result<ExpenseAutoApprovalRule, AppError> {
        let rule = sqlx::query_as::<_, ExpenseAutoApprovalRule>(
            r#"
            INSERT INTO expense_auto_approval_rules (organization_id, owner_id, unit_id,
                                                     max_amount_per_expense, max_monthly_total,
                                                     allowed_categories)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, organization_id, owner_id, unit_id, max_amount_per_expense,
                      max_monthly_total, allowed_categories, is_active, created_at, updated_at
            "#,
        )
        .bind(org_id)
        .bind(owner_id)
        .bind(req.unit_id)
        .bind(req.max_amount_per_expense.unwrap_or(Decimal::new(300, 0)))
        .bind(req.max_monthly_total.unwrap_or(Decimal::new(1000, 0)))
        .bind(
            req.allowed_categories
                .unwrap_or_else(|| vec!["maintenance".to_string()]),
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rule)
    }

    pub async fn get_auto_approval_rules(
        &self,
        owner_id: Uuid,
        org_id: Uuid,
    ) -> Result<Vec<ExpenseAutoApprovalRule>, AppError> {
        let rules = sqlx::query_as::<_, ExpenseAutoApprovalRule>(
            r#"
            SELECT id, organization_id, owner_id, unit_id, max_amount_per_expense,
                   max_monthly_total, allowed_categories, is_active, created_at, updated_at
            FROM expense_auto_approval_rules
            WHERE owner_id = $1 AND organization_id = $2
            ORDER BY created_at DESC
            "#,
        )
        .bind(owner_id)
        .bind(org_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rules)
    }

    pub async fn update_auto_approval_rule(
        &self,
        id: Uuid,
        owner_id: Uuid,
        req: UpdateAutoApprovalRule,
    ) -> Result<ExpenseAutoApprovalRule, AppError> {
        let rule = sqlx::query_as::<_, ExpenseAutoApprovalRule>(
            r#"
            UPDATE expense_auto_approval_rules
            SET unit_id = COALESCE($3, unit_id),
                max_amount_per_expense = COALESCE($4, max_amount_per_expense),
                max_monthly_total = COALESCE($5, max_monthly_total),
                allowed_categories = COALESCE($6, allowed_categories),
                is_active = COALESCE($7, is_active)
            WHERE id = $1 AND owner_id = $2
            RETURNING id, organization_id, owner_id, unit_id, max_amount_per_expense,
                      max_monthly_total, allowed_categories, is_active, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(owner_id)
        .bind(req.unit_id)
        .bind(req.max_amount_per_expense)
        .bind(req.max_monthly_total)
        .bind(&req.allowed_categories)
        .bind(req.is_active)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rule)
    }

    pub async fn delete_auto_approval_rule(
        &self,
        id: Uuid,
        owner_id: Uuid,
    ) -> Result<(), AppError> {
        sqlx::query("DELETE FROM expense_auto_approval_rules WHERE id = $1 AND owner_id = $2")
            .bind(id)
            .bind(owner_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    pub async fn submit_expense_for_approval(
        &self,
        submitted_by: Uuid,
        org_id: Uuid,
        req: SubmitExpenseForApproval,
    ) -> Result<ExpenseApprovalResponse, AppError> {
        // Enforce tenant isolation: the unit must belong to the caller's org.
        self.assert_unit_in_org(req.unit_id, org_id).await?;

        // Check for auto-approval rules (scoped to this org).
        let matching_rule = sqlx::query_as::<_, ExpenseAutoApprovalRule>(
            r#"
            SELECT id, organization_id, owner_id, unit_id, max_amount_per_expense,
                   max_monthly_total, allowed_categories, is_active, created_at, updated_at
            FROM expense_auto_approval_rules
            WHERE (unit_id = $1 OR unit_id IS NULL)
              AND organization_id = $4
              AND is_active = true
              AND max_amount_per_expense >= $2
              AND $3 = ANY(allowed_categories)
            ORDER BY unit_id DESC NULLS LAST
            LIMIT 1
            "#,
        )
        .bind(req.unit_id)
        .bind(req.amount)
        .bind(&req.category)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let (auto_approved, rule_id, status) = match &matching_rule {
            Some(rule) => {
                // Check monthly total
                let monthly_total: Decimal = sqlx::query_scalar(
                    r#"
                    SELECT COALESCE(SUM(amount), 0)
                    FROM expense_approval_requests
                    WHERE unit_id = $1
                      AND auto_approval_rule_id = $2
                      AND created_at >= date_trunc('month', CURRENT_DATE)
                    "#,
                )
                .bind(req.unit_id)
                .bind(rule.id)
                .fetch_one(&self.pool)
                .await
                .unwrap_or(Decimal::ZERO);

                if monthly_total + req.amount <= rule.max_monthly_total {
                    (true, Some(rule.id), "auto_approved".to_string())
                } else {
                    (false, None, "pending".to_string())
                }
            }
            None => (false, None, "pending".to_string()),
        };

        let expense = sqlx::query_as::<_, ExpenseApprovalRequest>(
            r#"
            INSERT INTO expense_approval_requests (unit_id, submitted_by, amount, category,
                                                   description, status, auto_approval_rule_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, unit_id, submitted_by, amount, category, description, status,
                      auto_approval_rule_id, reviewed_by, review_notes, created_at, updated_at
            "#,
        )
        .bind(req.unit_id)
        .bind(submitted_by)
        .bind(req.amount)
        .bind(&req.category)
        .bind(&req.description)
        .bind(&status)
        .bind(rule_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(ExpenseApprovalResponse {
            request: expense,
            auto_approved,
            message: if auto_approved {
                "Expense auto-approved based on your rules".to_string()
            } else {
                "Expense submitted for owner approval".to_string()
            },
        })
    }

    pub async fn list_expense_requests(
        &self,
        org_id: Uuid,
        q: ExpenseRequestsQuery,
    ) -> Result<ListExpenseRequestsResponse, AppError> {
        let limit = q.limit.unwrap_or(50) as i64;
        let offset = q.offset.unwrap_or(0) as i64;

        // Scope every expense request to units in buildings owned by `org_id`,
        // so a caller cannot read another tenant's expense history (or probe by
        // passing a foreign unit_id as `property_id`).
        let requests = sqlx::query_as::<_, ExpenseApprovalRequest>(
            r#"
            SELECT ear.id, ear.unit_id, ear.submitted_by, ear.amount, ear.category,
                   ear.description, ear.status, ear.auto_approval_rule_id, ear.reviewed_by,
                   ear.review_notes, ear.created_at, ear.updated_at
            FROM expense_approval_requests ear
            JOIN units u ON u.id = ear.unit_id
            JOIN buildings b ON b.id = u.building_id
            WHERE b.organization_id = $5
              AND ($1::uuid IS NULL OR ear.unit_id = $1)
              AND ($2::text IS NULL OR ear.status = $2)
            ORDER BY ear.created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(q.property_id)
        .bind(&q.status)
        .bind(limit)
        .bind(offset)
        .bind(org_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM expense_approval_requests ear
            JOIN units u ON u.id = ear.unit_id
            JOIN buildings b ON b.id = u.building_id
            WHERE b.organization_id = $3
              AND ($1::uuid IS NULL OR ear.unit_id = $1)
              AND ($2::text IS NULL OR ear.status = $2)
            "#,
        )
        .bind(q.property_id)
        .bind(&q.status)
        .bind(org_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(ListExpenseRequestsResponse { requests, total })
    }

    pub async fn review_expense(
        &self,
        id: Uuid,
        reviewed_by: Uuid,
        org_id: Uuid,
        req: ReviewExpenseRequest,
    ) -> Result<ExpenseApprovalRequest, AppError> {
        let status = match req.decision {
            ExpenseApprovalDecision::Approved => "approved",
            ExpenseApprovalDecision::Rejected => "rejected",
            ExpenseApprovalDecision::NeedsInfo => "needs_info",
        };

        // Scope the UPDATE to expense requests whose unit lives in a building
        // owned by `org_id`. A reviewer in another tenant therefore cannot
        // approve/reject this org's expenses (the row simply won't match).
        let expense = sqlx::query_as::<_, ExpenseApprovalRequest>(
            r#"
            UPDATE expense_approval_requests ear
            SET status = $2, reviewed_by = $3, review_notes = $4
            FROM units u, buildings b
            WHERE ear.id = $1
              AND u.id = ear.unit_id
              AND b.id = u.building_id
              AND b.organization_id = $5
            RETURNING ear.id, ear.unit_id, ear.submitted_by, ear.amount, ear.category,
                      ear.description, ear.status, ear.auto_approval_rule_id, ear.reviewed_by,
                      ear.review_notes, ear.created_at, ear.updated_at
            "#,
        )
        .bind(id)
        .bind(status)
        .bind(reviewed_by)
        .bind(&req.notes)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        expense.ok_or_else(|| AppError::NotFound("Expense request not found".to_string()))
    }
}
