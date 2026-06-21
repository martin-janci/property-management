//! Budget and financial planning models for Epic 24.
//!
//! Includes budgets, budget items, capital plans, reserve funds, and forecasts.

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

// ===========================================
// Status Constants
// ===========================================

/// Budget status values.
pub mod budget_status {
    pub const DRAFT: &str = "draft";
    pub const PENDING_APPROVAL: &str = "pending_approval";
    pub const APPROVED: &str = "approved";
    pub const ACTIVE: &str = "active";
    pub const CLOSED: &str = "closed";
}

/// Capital plan funding source values.
pub mod funding_source {
    pub const RESERVE_FUND: &str = "reserve_fund";
    pub const SPECIAL_ASSESSMENT: &str = "special_assessment";
    pub const LOAN: &str = "loan";
    pub const GRANT: &str = "grant";
    pub const OPERATING_BUDGET: &str = "operating_budget";
    pub const OTHER: &str = "other";
}

/// Capital plan priority values.
pub mod priority {
    pub const LOW: &str = "low";
    pub const MEDIUM: &str = "medium";
    pub const HIGH: &str = "high";
    pub const CRITICAL: &str = "critical";
}

/// Capital plan status values.
pub mod capital_plan_status {
    pub const PLANNED: &str = "planned";
    pub const APPROVED: &str = "approved";
    pub const IN_PROGRESS: &str = "in_progress";
    pub const COMPLETED: &str = "completed";
    pub const CANCELLED: &str = "cancelled";
    pub const DEFERRED: &str = "deferred";
}

/// Forecast type values.
pub mod forecast_type {
    pub const EXPENSE: &str = "expense";
    pub const REVENUE: &str = "revenue";
    pub const RESERVE: &str = "reserve";
    pub const COMBINED: &str = "combined";
}

/// Variance alert type values.
pub mod variance_alert_type {
    pub const WARNING: &str = "warning";
    pub const CRITICAL: &str = "critical";
    pub const EXCEEDED: &str = "exceeded";
}

// ===========================================
// Budget Models
// ===========================================

/// Budget entity.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Budget {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub building_id: Option<Uuid>,
    pub fiscal_year: i32,
    pub name: String,
    pub status: String,
    pub total_amount: Decimal,
    pub notes: Option<String>,
    pub approved_by: Option<Uuid>,
    pub approved_at: Option<DateTime<Utc>>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create budget request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateBudget {
    pub building_id: Option<Uuid>,
    pub fiscal_year: i32,
    pub name: String,
    pub notes: Option<String>,
}

/// Update budget request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateBudget {
    pub name: Option<String>,
    pub notes: Option<String>,
}

/// Budget query parameters.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BudgetQuery {
    pub building_id: Option<Uuid>,
    pub fiscal_year: Option<i32>,
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ===========================================
// Budget Category Models
// ===========================================

/// Budget category entity.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct BudgetCategory {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<Uuid>,
    pub sort_order: i32,
    pub is_system: bool,
    pub created_at: DateTime<Utc>,
}

/// Create budget category request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateBudgetCategory {
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<Uuid>,
    pub sort_order: Option<i32>,
}

/// Update budget category request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateBudgetCategory {
    pub name: Option<String>,
    pub description: Option<String>,
    pub sort_order: Option<i32>,
}

// ===========================================
// Budget Item Models
// ===========================================

/// Budget item entity.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct BudgetItem {
    pub id: Uuid,
    pub budget_id: Uuid,
    pub category_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub budgeted_amount: Decimal,
    pub actual_amount: Decimal,
    pub variance_amount: Decimal,
    pub variance_percent: Decimal,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create budget item request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateBudgetItem {
    pub category_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub budgeted_amount: Decimal,
    pub notes: Option<String>,
}

/// Update budget item request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateBudgetItem {
    pub name: Option<String>,
    pub description: Option<String>,
    pub budgeted_amount: Option<Decimal>,
    pub notes: Option<String>,
}

// ===========================================
// Budget Actual Models
// ===========================================

/// Budget actual expense entity.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct BudgetActual {
    pub id: Uuid,
    pub budget_item_id: Uuid,
    pub transaction_id: Option<Uuid>,
    pub amount: Decimal,
    pub description: Option<String>,
    pub transaction_date: NaiveDate,
    pub recorded_by: Uuid,
    pub created_at: DateTime<Utc>,
}

/// Record budget actual request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RecordBudgetActual {
    pub transaction_id: Option<Uuid>,
    pub amount: Decimal,
    pub description: Option<String>,
    pub transaction_date: NaiveDate,
}

// ===========================================
// Capital Plan Models
// ===========================================

/// Capital plan entity.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct CapitalPlan {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub building_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub estimated_cost: Decimal,
    pub actual_cost: Option<Decimal>,
    pub funding_source: String,
    pub target_year: i32,
    pub target_quarter: Option<i32>,
    pub priority: String,
    pub status: String,
    pub start_date: Option<NaiveDate>,
    pub completion_date: Option<NaiveDate>,
    pub notes: Option<String>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create capital plan request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateCapitalPlan {
    pub building_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub estimated_cost: Decimal,
    pub funding_source: String,
    pub target_year: i32,
    pub target_quarter: Option<i32>,
    pub priority: Option<String>,
    pub notes: Option<String>,
}

/// Update capital plan request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateCapitalPlan {
    pub name: Option<String>,
    pub description: Option<String>,
    pub estimated_cost: Option<Decimal>,
    pub actual_cost: Option<Decimal>,
    pub funding_source: Option<String>,
    pub target_year: Option<i32>,
    pub target_quarter: Option<i32>,
    pub priority: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub completion_date: Option<NaiveDate>,
    pub notes: Option<String>,
}

/// Capital plan query parameters.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CapitalPlanQuery {
    pub building_id: Option<Uuid>,
    pub target_year: Option<i32>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ===========================================
// Financial Forecast Models
// ===========================================

/// Financial forecast entity.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct FinancialForecast {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub building_id: Option<Uuid>,
    pub name: String,
    pub forecast_type: String,
    pub start_year: i32,
    pub end_year: i32,
    pub inflation_rate: Decimal,
    pub parameters: serde_json::Value,
    pub forecast_data: serde_json::Value,
    pub notes: Option<String>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create financial forecast request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateFinancialForecast {
    pub building_id: Option<Uuid>,
    pub name: String,
    pub forecast_type: Option<String>,
    pub start_year: i32,
    pub end_year: i32,
    pub inflation_rate: Option<Decimal>,
    pub parameters: Option<serde_json::Value>,
    pub notes: Option<String>,
}

/// Update financial forecast request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateFinancialForecast {
    pub name: Option<String>,
    pub inflation_rate: Option<Decimal>,
    pub parameters: Option<serde_json::Value>,
    pub forecast_data: Option<serde_json::Value>,
    pub notes: Option<String>,
}

/// Financial forecast query parameters.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ForecastQuery {
    pub building_id: Option<Uuid>,
    pub forecast_type: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ===========================================
// Budget Variance Alert Models
// ===========================================

/// Budget variance alert entity.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct BudgetVarianceAlert {
    pub id: Uuid,
    pub budget_item_id: Uuid,
    pub alert_type: String,
    pub threshold_percent: Decimal,
    pub current_variance_percent: Decimal,
    pub message: String,
    pub is_acknowledged: bool,
    pub acknowledged_by: Option<Uuid>,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Acknowledge variance alert request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AcknowledgeVarianceAlert {
    pub notes: Option<String>,
}

// ===========================================
// Statistics & Reporting Models
// ===========================================

/// Budget summary statistics.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BudgetSummary {
    pub total_budgeted: Decimal,
    pub total_actual: Decimal,
    pub total_variance: Decimal,
    pub variance_percent: Decimal,
    pub items_over_budget: i64,
    pub items_under_budget: i64,
}

/// Category variance summary.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct CategoryVariance {
    pub category_id: Uuid,
    pub category_name: String,
    pub budgeted_amount: Decimal,
    pub actual_amount: Decimal,
    pub variance_amount: Decimal,
    pub variance_percent: Decimal,
}

/// Capital plan summary by year.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct YearlyCapitalSummary {
    pub target_year: i32,
    pub total_estimated: Decimal,
    pub total_actual: Decimal,
    pub plan_count: i64,
}

/// Budget dashboard statistics.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BudgetDashboard {
    pub active_budget: Option<Budget>,
    pub summary: Option<BudgetSummary>,
    pub categories: Vec<CategoryVariance>,
    pub pending_alerts: i64,
    pub reserve_balance: Decimal,
}
