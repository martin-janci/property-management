//! Investor Portal repository for Epic 139: Investor Portal & ROI Reporting.
//!
//! Provides database operations for investor profiles, portfolios, ROI calculations,
//! distributions, reports, and dashboard metrics.

use rust_decimal::Decimal;
use sqlx::{Error as SqlxError, Row};
use uuid::Uuid;

use crate::models::investor_portal::*;
use crate::DbPool;

/// Repository for investor portal operations.
#[derive(Clone)]
pub struct InvestorPortalRepository {
    pool: DbPool,
}

impl InvestorPortalRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // =========================================================================
    // INVESTOR PROFILES
    // =========================================================================

    /// Create a new investor profile.
    pub async fn create_investor_profile(
        &self,
        org_id: Uuid,
        data: &CreateInvestorProfile,
        created_by: Uuid,
    ) -> Result<InvestorProfile, SqlxError> {
        sqlx::query_as::<_, InvestorProfile>(
            r#"
            INSERT INTO investor_profiles (
                organization_id, user_id, display_name, investor_type, tax_id, tax_country,
                email, phone, address_line1, address_line2, city, state, postal_code, country,
                preferred_currency, distribution_preference, report_frequency, accredited_investor,
                created_by
            )
            VALUES ($1, $2, $3, $4::investor_type, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16::distribution_type, $17, $18, $19)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.user_id)
        .bind(&data.display_name)
        .bind(data.investor_type.as_ref().unwrap_or(&InvestorType::Individual))
        .bind(&data.tax_id)
        .bind(&data.tax_country)
        .bind(&data.email)
        .bind(&data.phone)
        .bind(&data.address_line1)
        .bind(&data.address_line2)
        .bind(&data.city)
        .bind(&data.state)
        .bind(&data.postal_code)
        .bind(&data.country)
        .bind(data.preferred_currency.as_deref().unwrap_or("EUR"))
        .bind(&data.distribution_preference)
        .bind(data.report_frequency.as_deref().unwrap_or("quarterly"))
        .bind(data.accredited_investor.unwrap_or(false))
        .bind(created_by)
        .fetch_one(&self.pool)
        .await
    }

    /// Get an investor profile by ID.
    pub async fn get_investor_profile(
        &self,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<InvestorProfile>, SqlxError> {
        sqlx::query_as::<_, InvestorProfile>(
            r#"
            SELECT * FROM investor_profiles
            WHERE id = $1 AND organization_id = $2
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// List all investor profiles for an organization.
    pub async fn list_investor_profiles(
        &self,
        org_id: Uuid,
    ) -> Result<Vec<InvestorProfile>, SqlxError> {
        sqlx::query_as::<_, InvestorProfile>(
            r#"
            SELECT * FROM investor_profiles
            WHERE organization_id = $1
            ORDER BY display_name
            "#,
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Update an investor profile.
    pub async fn update_investor_profile(
        &self,
        id: Uuid,
        org_id: Uuid,
        data: &UpdateInvestorProfile,
    ) -> Result<Option<InvestorProfile>, SqlxError> {
        sqlx::query_as::<_, InvestorProfile>(
            r#"
            UPDATE investor_profiles SET
                display_name = COALESCE($3, display_name),
                investor_type = COALESCE($4::investor_type, investor_type),
                tax_id = COALESCE($5, tax_id),
                tax_country = COALESCE($6, tax_country),
                email = COALESCE($7, email),
                phone = COALESCE($8, phone),
                address_line1 = COALESCE($9, address_line1),
                address_line2 = COALESCE($10, address_line2),
                city = COALESCE($11, city),
                state = COALESCE($12, state),
                postal_code = COALESCE($13, postal_code),
                country = COALESCE($14, country),
                preferred_currency = COALESCE($15, preferred_currency),
                distribution_preference = COALESCE($16::distribution_type, distribution_preference),
                report_frequency = COALESCE($17, report_frequency),
                kyc_verified = COALESCE($18, kyc_verified),
                accredited_investor = COALESCE($19, accredited_investor),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(org_id)
        .bind(&data.display_name)
        .bind(&data.investor_type)
        .bind(&data.tax_id)
        .bind(&data.tax_country)
        .bind(&data.email)
        .bind(&data.phone)
        .bind(&data.address_line1)
        .bind(&data.address_line2)
        .bind(&data.city)
        .bind(&data.state)
        .bind(&data.postal_code)
        .bind(&data.country)
        .bind(&data.preferred_currency)
        .bind(&data.distribution_preference)
        .bind(&data.report_frequency)
        .bind(data.kyc_verified)
        .bind(data.accredited_investor)
        .fetch_optional(&self.pool)
        .await
    }

    /// Delete an investor profile.
    pub async fn delete_investor_profile(&self, id: Uuid, org_id: Uuid) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            r#"
            DELETE FROM investor_profiles
            WHERE id = $1 AND organization_id = $2
            "#,
        )
        .bind(id)
        .bind(org_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    // =========================================================================
    // INVESTMENT PORTFOLIOS
    // =========================================================================

    /// Create a new investment portfolio.
    pub async fn create_portfolio(
        &self,
        org_id: Uuid,
        data: &CreateInvestmentPortfolio,
        created_by: Uuid,
    ) -> Result<InvestmentPortfolio, SqlxError> {
        sqlx::query_as::<_, InvestmentPortfolio>(
            r#"
            INSERT INTO investment_portfolios (
                organization_id, investor_id, name, description, initial_investment,
                ownership_percentage, currency, investment_date, target_exit_date, created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.investor_id)
        .bind(&data.name)
        .bind(&data.description)
        .bind(data.initial_investment)
        .bind(data.ownership_percentage)
        .bind(data.currency.as_deref().unwrap_or("EUR"))
        .bind(data.investment_date)
        .bind(data.target_exit_date)
        .bind(created_by)
        .fetch_one(&self.pool)
        .await
    }

    /// Get a portfolio by ID.
    pub async fn get_portfolio(
        &self,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<InvestmentPortfolio>, SqlxError> {
        sqlx::query_as::<_, InvestmentPortfolio>(
            r#"
            SELECT * FROM investment_portfolios
            WHERE id = $1 AND organization_id = $2
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// List portfolios for an investor.
    pub async fn list_portfolios_by_investor(
        &self,
        org_id: Uuid,
        investor_id: Uuid,
    ) -> Result<Vec<InvestmentPortfolio>, SqlxError> {
        sqlx::query_as::<_, InvestmentPortfolio>(
            r#"
            SELECT * FROM investment_portfolios
            WHERE organization_id = $1 AND investor_id = $2
            ORDER BY investment_date DESC
            "#,
        )
        .bind(org_id)
        .bind(investor_id)
        .fetch_all(&self.pool)
        .await
    }

    /// List all portfolios for an organization.
    pub async fn list_portfolios(
        &self,
        org_id: Uuid,
    ) -> Result<Vec<InvestmentPortfolio>, SqlxError> {
        sqlx::query_as::<_, InvestmentPortfolio>(
            r#"
            SELECT * FROM investment_portfolios
            WHERE organization_id = $1
            ORDER BY name
            "#,
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Update a portfolio.
    pub async fn update_portfolio(
        &self,
        id: Uuid,
        org_id: Uuid,
        data: &UpdateInvestmentPortfolio,
    ) -> Result<Option<InvestmentPortfolio>, SqlxError> {
        sqlx::query_as::<_, InvestmentPortfolio>(
            r#"
            UPDATE investment_portfolios SET
                name = COALESCE($3, name),
                description = COALESCE($4, description),
                status = COALESCE($5::investment_status, status),
                current_value = COALESCE($6, current_value),
                ownership_percentage = COALESCE($7, ownership_percentage),
                exit_date = COALESCE($8, exit_date),
                target_exit_date = COALESCE($9, target_exit_date),
                irr = COALESCE($10, irr),
                multiple = COALESCE($11, multiple),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(org_id)
        .bind(&data.name)
        .bind(&data.description)
        .bind(&data.status)
        .bind(data.current_value)
        .bind(data.ownership_percentage)
        .bind(data.exit_date)
        .bind(data.target_exit_date)
        .bind(data.irr)
        .bind(data.multiple)
        .fetch_optional(&self.pool)
        .await
    }

    /// Delete a portfolio.
    pub async fn delete_portfolio(&self, id: Uuid, org_id: Uuid) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            r#"
            DELETE FROM investment_portfolios
            WHERE id = $1 AND organization_id = $2
            "#,
        )
        .bind(id)
        .bind(org_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    // =========================================================================
    // PORTFOLIO PROPERTIES
    // =========================================================================

    /// Add a property to a portfolio.
    pub async fn add_portfolio_property(
        &self,
        portfolio_id: Uuid,
        data: &CreateInvestorPortfolioProperty,
    ) -> Result<InvestorPortfolioProperty, SqlxError> {
        sqlx::query_as::<_, InvestorPortfolioProperty>(
            r#"
            INSERT INTO portfolio_properties (
                portfolio_id, building_id, investment_amount, ownership_share,
                acquisition_date, acquisition_cost
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(portfolio_id)
        .bind(data.building_id)
        .bind(data.investment_amount)
        .bind(data.ownership_share)
        .bind(data.acquisition_date)
        .bind(data.acquisition_cost)
        .fetch_one(&self.pool)
        .await
    }

    /// List properties in a portfolio.
    pub async fn list_portfolio_properties(
        &self,
        portfolio_id: Uuid,
    ) -> Result<Vec<InvestorPortfolioProperty>, SqlxError> {
        sqlx::query_as::<_, InvestorPortfolioProperty>(
            r#"
            SELECT * FROM portfolio_properties
            WHERE portfolio_id = $1
            ORDER BY acquisition_date
            "#,
        )
        .bind(portfolio_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Update a portfolio property.
    pub async fn update_portfolio_property(
        &self,
        id: Uuid,
        data: &UpdateInvestorPortfolioProperty,
    ) -> Result<Option<InvestorPortfolioProperty>, SqlxError> {
        sqlx::query_as::<_, InvestorPortfolioProperty>(
            r#"
            UPDATE portfolio_properties SET
                investment_amount = COALESCE($2, investment_amount),
                ownership_share = COALESCE($3, ownership_share),
                current_value = COALESCE($4, current_value),
                appraised_value = COALESCE($5, appraised_value),
                appraised_at = COALESCE($6, appraised_at),
                rental_income_share = COALESCE($7, rental_income_share),
                operating_expenses_share = COALESCE($8, operating_expenses_share),
                net_income_share = COALESCE($9, net_income_share),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.investment_amount)
        .bind(data.ownership_share)
        .bind(data.current_value)
        .bind(data.appraised_value)
        .bind(data.appraised_at)
        .bind(data.rental_income_share)
        .bind(data.operating_expenses_share)
        .bind(data.net_income_share)
        .fetch_optional(&self.pool)
        .await
    }

    /// Remove a property from a portfolio.
    pub async fn remove_portfolio_property(&self, id: Uuid) -> Result<bool, SqlxError> {
        let result = sqlx::query("DELETE FROM portfolio_properties WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    // =========================================================================
    // ROI CALCULATIONS
    // =========================================================================

    /// Create a ROI calculation.
    pub async fn create_roi_calculation(
        &self,
        org_id: Uuid,
        data: &CreateRoiCalculation,
    ) -> Result<RoiCalculation, SqlxError> {
        // Calculate returns
        let contributions = data.contributions.unwrap_or(Decimal::ZERO);
        let distributions = data.distributions.unwrap_or(Decimal::ZERO);
        let gross_return = data.ending_value - data.beginning_value + distributions - contributions;
        let net_return = gross_return;
        let return_pct = if data.beginning_value > Decimal::ZERO {
            Some((gross_return / data.beginning_value) * Decimal::from(100))
        } else {
            None
        };

        sqlx::query_as::<_, RoiCalculation>(
            r#"
            INSERT INTO roi_calculations (
                organization_id, portfolio_id, period_type, period_start, period_end,
                beginning_value, ending_value, contributions, distributions,
                gross_return, net_return, return_percentage,
                rental_income, other_income, operating_expenses, capital_expenditures
            )
            VALUES ($1, $2, $3::roi_period, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.portfolio_id)
        .bind(&data.period_type)
        .bind(data.period_start)
        .bind(data.period_end)
        .bind(data.beginning_value)
        .bind(data.ending_value)
        .bind(contributions)
        .bind(distributions)
        .bind(gross_return)
        .bind(net_return)
        .bind(return_pct)
        .bind(data.rental_income)
        .bind(data.other_income)
        .bind(data.operating_expenses)
        .bind(data.capital_expenditures)
        .fetch_one(&self.pool)
        .await
    }

    /// Get ROI calculations for a portfolio.
    pub async fn list_roi_calculations(
        &self,
        org_id: Uuid,
        query: &RoiCalculationQuery,
    ) -> Result<Vec<RoiCalculation>, SqlxError> {
        let mut sql = String::from(
            r#"
            SELECT * FROM roi_calculations
            WHERE organization_id = $1
            "#,
        );
        let mut params: Vec<String> = vec![];

        if query.portfolio_id.is_some() {
            params.push(format!(" AND portfolio_id = ${}", params.len() + 2));
        }
        if query.period_type.is_some() {
            params.push(format!(
                " AND period_type = ${}::roi_period",
                params.len() + 2
            ));
        }
        if query.start_date.is_some() {
            params.push(format!(" AND period_start >= ${}", params.len() + 2));
        }
        if query.end_date.is_some() {
            params.push(format!(" AND period_end <= ${}", params.len() + 2));
        }

        sql.push_str(&params.join(""));
        sql.push_str(" ORDER BY period_end DESC");

        let mut query_builder =
            sqlx::query_as::<_, RoiCalculation>(sqlx::AssertSqlSafe(sql)).bind(org_id);

        if let Some(portfolio_id) = query.portfolio_id {
            query_builder = query_builder.bind(portfolio_id);
        }
        if let Some(ref period_type) = query.period_type {
            query_builder = query_builder.bind(period_type);
        }
        if let Some(start_date) = query.start_date {
            query_builder = query_builder.bind(start_date);
        }
        if let Some(end_date) = query.end_date {
            query_builder = query_builder.bind(end_date);
        }

        query_builder.fetch_all(&self.pool).await
    }

    /// Get the latest ROI calculation for a portfolio.
    pub async fn get_latest_roi(
        &self,
        org_id: Uuid,
        portfolio_id: Uuid,
    ) -> Result<Option<RoiCalculation>, SqlxError> {
        sqlx::query_as::<_, RoiCalculation>(
            r#"
            SELECT * FROM roi_calculations
            WHERE organization_id = $1 AND portfolio_id = $2
            ORDER BY period_end DESC
            LIMIT 1
            "#,
        )
        .bind(org_id)
        .bind(portfolio_id)
        .fetch_optional(&self.pool)
        .await
    }

    // =========================================================================
    // DISTRIBUTIONS
    // =========================================================================

    /// Create a distribution.
    pub async fn create_distribution(
        &self,
        org_id: Uuid,
        data: &CreateDistribution,
        created_by: Uuid,
    ) -> Result<InvestorDistribution, SqlxError> {
        let net_amount = data.gross_amount.unwrap_or(data.amount)
            - data.withholding_tax.unwrap_or(Decimal::ZERO);

        sqlx::query_as::<_, InvestorDistribution>(
            r#"
            INSERT INTO investor_distributions (
                organization_id, portfolio_id, investor_id, distribution_type,
                amount, currency, gross_amount, withholding_tax, net_amount,
                tax_year, scheduled_date, created_by
            )
            VALUES ($1, $2, $3, $4::distribution_type, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.portfolio_id)
        .bind(data.investor_id)
        .bind(&data.distribution_type)
        .bind(data.amount)
        .bind(data.currency.as_deref().unwrap_or("EUR"))
        .bind(data.gross_amount.unwrap_or(data.amount))
        .bind(data.withholding_tax)
        .bind(net_amount)
        .bind(data.tax_year)
        .bind(data.scheduled_date)
        .bind(created_by)
        .fetch_one(&self.pool)
        .await
    }

    /// List distributions for an investor.
    pub async fn list_distributions_by_investor(
        &self,
        org_id: Uuid,
        investor_id: Uuid,
    ) -> Result<Vec<InvestorDistribution>, SqlxError> {
        sqlx::query_as::<_, InvestorDistribution>(
            r#"
            SELECT * FROM investor_distributions
            WHERE organization_id = $1 AND investor_id = $2
            ORDER BY scheduled_date DESC
            "#,
        )
        .bind(org_id)
        .bind(investor_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Update a distribution.
    pub async fn update_distribution(
        &self,
        id: Uuid,
        org_id: Uuid,
        data: &UpdateDistribution,
    ) -> Result<Option<InvestorDistribution>, SqlxError> {
        sqlx::query_as::<_, InvestorDistribution>(
            r#"
            UPDATE investor_distributions SET
                amount = COALESCE($3, amount),
                gross_amount = COALESCE($4, gross_amount),
                withholding_tax = COALESCE($5, withholding_tax),
                net_amount = COALESCE($6, net_amount),
                scheduled_date = COALESCE($7, scheduled_date),
                paid_date = COALESCE($8, paid_date),
                payment_method = COALESCE($9, payment_method),
                payment_reference = COALESCE($10, payment_reference),
                status = COALESCE($11, status),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(org_id)
        .bind(data.amount)
        .bind(data.gross_amount)
        .bind(data.withholding_tax)
        .bind(data.net_amount)
        .bind(data.scheduled_date)
        .bind(data.paid_date)
        .bind(&data.payment_method)
        .bind(&data.payment_reference)
        .bind(&data.status)
        .fetch_optional(&self.pool)
        .await
    }

    // =========================================================================
    // INVESTOR REPORTS
    // =========================================================================

    /// Create an investor report.
    pub async fn create_report(
        &self,
        org_id: Uuid,
        data: &CreateInvestorReport,
        created_by: Uuid,
    ) -> Result<InvestorReport, SqlxError> {
        sqlx::query_as::<_, InvestorReport>(
            r#"
            INSERT INTO investor_reports (
                organization_id, investor_id, portfolio_id, report_type,
                title, description, period_start, period_end, report_data, created_by
            )
            VALUES ($1, $2, $3, $4::report_type, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.investor_id)
        .bind(data.portfolio_id)
        .bind(&data.report_type)
        .bind(&data.title)
        .bind(&data.description)
        .bind(data.period_start)
        .bind(data.period_end)
        .bind(&data.report_data)
        .bind(created_by)
        .fetch_one(&self.pool)
        .await
    }

    /// List reports for an investor.
    pub async fn list_reports_by_investor(
        &self,
        org_id: Uuid,
        investor_id: Uuid,
    ) -> Result<Vec<InvestorReport>, SqlxError> {
        sqlx::query_as::<_, InvestorReport>(
            r#"
            SELECT * FROM investor_reports
            WHERE organization_id = $1 AND investor_id = $2
            ORDER BY created_at DESC
            "#,
        )
        .bind(org_id)
        .bind(investor_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Get a report by ID.
    pub async fn get_report(
        &self,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<InvestorReport>, SqlxError> {
        sqlx::query_as::<_, InvestorReport>(
            r#"
            SELECT * FROM investor_reports
            WHERE id = $1 AND organization_id = $2
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
    }

    // =========================================================================
    // CAPITAL CALLS
    // =========================================================================

    /// Create a capital call.
    pub async fn create_capital_call(
        &self,
        org_id: Uuid,
        data: &CreateCapitalCall,
        created_by: Uuid,
    ) -> Result<CapitalCall, SqlxError> {
        sqlx::query_as::<_, CapitalCall>(
            r#"
            INSERT INTO capital_calls (
                organization_id, portfolio_id, investor_id, call_number,
                amount, currency, purpose, call_date, due_date, created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.portfolio_id)
        .bind(data.investor_id)
        .bind(data.call_number)
        .bind(data.amount)
        .bind(data.currency.as_deref().unwrap_or("EUR"))
        .bind(&data.purpose)
        .bind(data.call_date)
        .bind(data.due_date)
        .bind(created_by)
        .fetch_one(&self.pool)
        .await
    }

    /// List capital calls for an investor.
    pub async fn list_capital_calls_by_investor(
        &self,
        org_id: Uuid,
        investor_id: Uuid,
    ) -> Result<Vec<CapitalCall>, SqlxError> {
        sqlx::query_as::<_, CapitalCall>(
            r#"
            SELECT * FROM capital_calls
            WHERE organization_id = $1 AND investor_id = $2
            ORDER BY call_date DESC
            "#,
        )
        .bind(org_id)
        .bind(investor_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Update a capital call.
    pub async fn update_capital_call(
        &self,
        id: Uuid,
        org_id: Uuid,
        data: &UpdateCapitalCall,
    ) -> Result<Option<CapitalCall>, SqlxError> {
        sqlx::query_as::<_, CapitalCall>(
            r#"
            UPDATE capital_calls SET
                amount = COALESCE($3, amount),
                purpose = COALESCE($4, purpose),
                due_date = COALESCE($5, due_date),
                funded_date = COALESCE($6, funded_date),
                status = COALESCE($7, status),
                funded_amount = COALESCE($8, funded_amount),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(org_id)
        .bind(data.amount)
        .bind(&data.purpose)
        .bind(data.due_date)
        .bind(data.funded_date)
        .bind(&data.status)
        .bind(data.funded_amount)
        .fetch_optional(&self.pool)
        .await
    }

    // =========================================================================
    // DASHBOARD METRICS
    // =========================================================================

    /// Create or update dashboard metrics.
    pub async fn upsert_dashboard_metrics(
        &self,
        org_id: Uuid,
        data: &CreateDashboardMetrics,
    ) -> Result<InvestorDashboardMetrics, SqlxError> {
        let total_return = data.total_value - data.total_invested;

        sqlx::query_as::<_, InvestorDashboardMetrics>(
            r#"
            INSERT INTO investor_dashboard_metrics (
                organization_id, investor_id, metric_date,
                total_invested, total_value, total_distributions, total_return,
                ytd_return, itd_return, irr, cash_on_cash, equity_multiple,
                property_count, portfolio_count, monthly_income, annual_income
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            ON CONFLICT (investor_id, metric_date)
            DO UPDATE SET
                total_invested = EXCLUDED.total_invested,
                total_value = EXCLUDED.total_value,
                total_distributions = EXCLUDED.total_distributions,
                total_return = EXCLUDED.total_return,
                ytd_return = EXCLUDED.ytd_return,
                itd_return = EXCLUDED.itd_return,
                irr = EXCLUDED.irr,
                cash_on_cash = EXCLUDED.cash_on_cash,
                equity_multiple = EXCLUDED.equity_multiple,
                property_count = EXCLUDED.property_count,
                portfolio_count = EXCLUDED.portfolio_count,
                monthly_income = EXCLUDED.monthly_income,
                annual_income = EXCLUDED.annual_income,
                calculated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.investor_id)
        .bind(data.metric_date)
        .bind(data.total_invested)
        .bind(data.total_value)
        .bind(data.total_distributions)
        .bind(total_return)
        .bind(data.ytd_return)
        .bind(data.itd_return)
        .bind(data.irr)
        .bind(data.cash_on_cash)
        .bind(data.equity_multiple)
        .bind(data.property_count)
        .bind(data.portfolio_count)
        .bind(data.monthly_income)
        .bind(data.annual_income)
        .fetch_one(&self.pool)
        .await
    }

    /// Get the latest dashboard metrics for an investor.
    pub async fn get_latest_dashboard_metrics(
        &self,
        org_id: Uuid,
        investor_id: Uuid,
    ) -> Result<Option<InvestorDashboardMetrics>, SqlxError> {
        sqlx::query_as::<_, InvestorDashboardMetrics>(
            r#"
            SELECT * FROM investor_dashboard_metrics
            WHERE organization_id = $1 AND investor_id = $2
            ORDER BY metric_date DESC
            LIMIT 1
            "#,
        )
        .bind(org_id)
        .bind(investor_id)
        .fetch_optional(&self.pool)
        .await
    }

    // =========================================================================
    // SUMMARY & DASHBOARD
    // =========================================================================

    /// Get investor summary with portfolio statistics.
    pub async fn get_investor_summary(
        &self,
        org_id: Uuid,
        investor_id: Uuid,
    ) -> Result<Option<InvestorSummary>, SqlxError> {
        let row = sqlx::query(
            r#"
            SELECT
                ip.id,
                ip.display_name,
                ip.investor_type,
                COUNT(DISTINCT port.id) as portfolio_count,
                COALESCE(SUM(port.initial_investment), 0) as total_invested,
                COALESCE(SUM(port.current_value), 0) as total_value
            FROM investor_profiles ip
            LEFT JOIN investment_portfolios port ON port.investor_id = ip.id
            WHERE ip.organization_id = $1 AND ip.id = $2
            GROUP BY ip.id, ip.display_name, ip.investor_type
            "#,
        )
        .bind(org_id)
        .bind(investor_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let total_invested: Decimal = row.get("total_invested");
            let total_value: Decimal = row.get("total_value");
            let overall_return = if total_invested > Decimal::ZERO {
                Some(((total_value - total_invested) / total_invested) * Decimal::from(100))
            } else {
                None
            };

            Ok(Some(InvestorSummary {
                id: row.get("id"),
                display_name: row.get("display_name"),
                investor_type: row.get("investor_type"),
                portfolio_count: row.get("portfolio_count"),
                total_invested,
                total_value,
                overall_return,
            }))
        } else {
            Ok(None)
        }
    }

    /// Get pending capital calls count.
    pub async fn get_pending_capital_calls_count(
        &self,
        org_id: Uuid,
        investor_id: Uuid,
    ) -> Result<i64, SqlxError> {
        let row = sqlx::query(
            r#"
            SELECT COUNT(*) as count
            FROM capital_calls
            WHERE organization_id = $1 AND investor_id = $2 AND status = 'pending'
            "#,
        )
        .bind(org_id)
        .bind(investor_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.get("count"))
    }
}
