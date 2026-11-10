//! Portfolio Analytics models (Epic 140: Multi-Property Portfolio Analytics).
//! Provides portfolio benchmarking, KPIs, trend analysis, and property comparison.

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

// =============================================================================
// ENUMS
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, sqlx::Type)]
#[sqlx(type_name = "benchmark_category", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum BenchmarkCategory {
    Occupancy,
    Revenue,
    Expense,
    Noi,
    CapRate,
    Maintenance,
    TenantSatisfaction,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, sqlx::Type)]
#[sqlx(type_name = "aggregation_period", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AggregationPeriod {
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Yearly,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, sqlx::Type)]
#[sqlx(type_name = "comparison_scope", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ComparisonScope {
    Portfolio,
    Building,
    UnitType,
    Region,
    Market,
}

// =============================================================================
// PORTFOLIO BENCHMARKS (Story 140.1)
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct PortfolioBenchmark {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub category: BenchmarkCategory,

    pub target_value: Decimal,
    pub min_acceptable: Option<Decimal>,
    pub max_acceptable: Option<Decimal>,

    pub scope: ComparisonScope,
    pub property_type: Option<String>,
    pub region: Option<String>,

    pub is_industry_standard: bool,
    pub source_name: Option<String>,
    pub source_year: Option<i32>,

    pub is_active: bool,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreatePortfolioBenchmark {
    pub name: String,
    pub description: Option<String>,
    pub category: BenchmarkCategory,
    pub target_value: Decimal,
    pub min_acceptable: Option<Decimal>,
    pub max_acceptable: Option<Decimal>,
    #[serde(default = "default_scope")]
    pub scope: ComparisonScope,
    pub property_type: Option<String>,
    pub region: Option<String>,
    #[serde(default)]
    pub is_industry_standard: bool,
    pub source_name: Option<String>,
    pub source_year: Option<i32>,
}

fn default_scope() -> ComparisonScope {
    ComparisonScope::Portfolio
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdatePortfolioBenchmark {
    pub name: Option<String>,
    pub description: Option<String>,
    pub target_value: Option<Decimal>,
    pub min_acceptable: Option<Decimal>,
    pub max_acceptable: Option<Decimal>,
    pub is_active: Option<bool>,
}

// =============================================================================
// PROPERTY PERFORMANCE METRICS (Story 140.2)
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct PropertyPerformanceMetrics {
    pub id: Uuid,
    pub building_id: Uuid,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub period_type: AggregationPeriod,

    // Occupancy
    pub total_units: i32,
    pub occupied_units: i32,
    pub occupancy_rate: Option<Decimal>,
    pub average_lease_term_months: Option<Decimal>,
    pub tenant_turnover_rate: Option<Decimal>,

    // Financial
    pub gross_rental_income: Decimal,
    pub other_income: Option<Decimal>,
    pub total_revenue: Option<Decimal>,
    pub operating_expenses: Option<Decimal>,
    pub net_operating_income: Option<Decimal>,
    pub currency: String,

    // Per unit
    pub revenue_per_unit: Option<Decimal>,
    pub expense_per_unit: Option<Decimal>,

    // Efficiency
    pub expense_ratio: Option<Decimal>,
    pub collection_rate: Option<Decimal>,

    // Maintenance
    pub maintenance_requests: Option<i32>,
    pub avg_resolution_time_hours: Option<Decimal>,
    pub maintenance_cost: Option<Decimal>,

    // Tenant satisfaction
    pub tenant_satisfaction_score: Option<Decimal>,
    pub complaints_count: Option<i32>,

    // Property value
    pub estimated_value: Option<Decimal>,
    pub cap_rate: Option<Decimal>,

    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreatePropertyMetrics {
    pub building_id: Uuid,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    #[serde(default = "default_period")]
    pub period_type: AggregationPeriod,

    pub total_units: i32,
    pub occupied_units: i32,
    pub average_lease_term_months: Option<Decimal>,
    pub tenant_turnover_rate: Option<Decimal>,

    pub gross_rental_income: Decimal,
    pub other_income: Option<Decimal>,
    pub operating_expenses: Option<Decimal>,
    #[serde(default = "default_currency")]
    pub currency: String,

    pub revenue_per_unit: Option<Decimal>,
    pub expense_per_unit: Option<Decimal>,

    pub expense_ratio: Option<Decimal>,
    pub collection_rate: Option<Decimal>,

    pub maintenance_requests: Option<i32>,
    pub avg_resolution_time_hours: Option<Decimal>,
    pub maintenance_cost: Option<Decimal>,

    pub tenant_satisfaction_score: Option<Decimal>,
    pub complaints_count: Option<i32>,

    pub estimated_value: Option<Decimal>,
    pub cap_rate: Option<Decimal>,
}

fn default_period() -> AggregationPeriod {
    AggregationPeriod::Monthly
}

fn default_currency() -> String {
    "EUR".to_string()
}

// =============================================================================
// PORTFOLIO AGGREGATED METRICS (Story 140.3)
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct PortfolioAggregatedMetrics {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub period_type: AggregationPeriod,

    pub total_buildings: i32,
    pub total_units: i32,
    pub total_sqm: Option<Decimal>,

    pub occupied_units: i32,
    pub portfolio_occupancy_rate: Option<Decimal>,

    pub total_revenue: Decimal,
    pub total_expenses: Option<Decimal>,
    pub total_noi: Option<Decimal>,
    pub currency: String,

    pub avg_rent_per_unit: Option<Decimal>,
    pub avg_rent_per_sqm: Option<Decimal>,
    pub avg_expense_per_unit: Option<Decimal>,
    pub avg_cap_rate: Option<Decimal>,

    pub revenue_growth_pct: Option<Decimal>,
    pub expense_growth_pct: Option<Decimal>,
    pub noi_growth_pct: Option<Decimal>,

    pub estimated_portfolio_value: Option<Decimal>,

    pub buildings_by_type: Option<JsonValue>,
    pub revenue_by_region: Option<JsonValue>,

    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PortfolioSummary {
    pub total_buildings: i32,
    pub total_units: i32,
    pub occupied_units: i32,
    pub occupancy_rate: Decimal,
    pub total_revenue: Decimal,
    pub total_noi: Decimal,
    pub estimated_value: Decimal,
    pub currency: String,
}

// =============================================================================
// PROPERTY COMPARISONS (Story 140.4)
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct PortfolioPropertyComparison {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub description: Option<String>,

    pub building_ids: Vec<Uuid>,

    pub comparison_period_start: NaiveDate,
    pub comparison_period_end: NaiveDate,
    pub metrics_to_compare: Vec<String>,

    pub comparison_results: Option<JsonValue>,
    pub rankings: Option<JsonValue>,

    pub is_saved: bool,
    pub created_by: Option<Uuid>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreatePropertyComparison {
    pub name: String,
    pub description: Option<String>,
    pub building_ids: Vec<Uuid>,
    pub comparison_period_start: NaiveDate,
    pub comparison_period_end: NaiveDate,
    #[serde(default)]
    pub metrics_to_compare: Vec<String>,
    #[serde(default)]
    pub is_saved: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PropertyRanking {
    pub building_id: Uuid,
    pub building_name: String,
    pub metric_name: String,
    pub value: Decimal,
    pub rank: i32,
    pub vs_portfolio_avg: Decimal,
}

// =============================================================================
// PORTFOLIO TRENDS (Story 140.5)
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct PortfolioTrend {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub building_id: Option<Uuid>,
    pub metric_name: String,

    pub recorded_at: NaiveDate,
    pub value: Decimal,
    pub previous_value: Option<Decimal>,
    pub change_pct: Option<Decimal>,

    pub period_type: AggregationPeriod,
    pub currency: Option<String>,
    pub notes: Option<String>,

    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RecordTrend {
    pub building_id: Option<Uuid>,
    pub metric_name: String,
    pub recorded_at: NaiveDate,
    pub value: Decimal,
    #[serde(default = "default_period")]
    pub period_type: AggregationPeriod,
    pub currency: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TrendDataPoint {
    pub date: NaiveDate,
    pub value: Decimal,
    pub change_pct: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TrendAnalysis {
    pub metric_name: String,
    pub data_points: Vec<TrendDataPoint>,
    pub avg_value: Decimal,
    pub min_value: Decimal,
    pub max_value: Decimal,
    pub trend_direction: String, // up, down, stable
    pub growth_rate: Option<Decimal>,
}

// =============================================================================
// ALERT RULES (Story 140.6)
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct PortfolioAlertRule {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub description: Option<String>,

    pub metric_name: String,
    pub category: BenchmarkCategory,

    pub operator: String,
    pub threshold_value: Decimal,

    pub scope: ComparisonScope,
    pub building_id: Option<Uuid>,

    pub notify_roles: Vec<String>,
    pub notify_users: Vec<Uuid>,
    pub notification_frequency: Option<String>,

    pub is_active: bool,
    pub last_triggered_at: Option<DateTime<Utc>>,
    pub trigger_count: i32,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateAlertRule {
    pub name: String,
    pub description: Option<String>,
    pub metric_name: String,
    pub category: BenchmarkCategory,
    #[serde(default = "default_operator")]
    pub operator: String,
    pub threshold_value: Decimal,
    #[serde(default = "default_scope")]
    pub scope: ComparisonScope,
    pub building_id: Option<Uuid>,
    #[serde(default)]
    pub notify_roles: Vec<String>,
    #[serde(default)]
    pub notify_users: Vec<Uuid>,
    #[serde(default = "default_frequency")]
    pub notification_frequency: String,
}

fn default_operator() -> String {
    "<".to_string()
}

fn default_frequency() -> String {
    "immediate".to_string()
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateAlertRule {
    pub name: Option<String>,
    pub description: Option<String>,
    pub threshold_value: Option<Decimal>,
    pub operator: Option<String>,
    pub notify_roles: Option<Vec<String>>,
    pub notify_users: Option<Vec<Uuid>>,
    pub notification_frequency: Option<String>,
    pub is_active: Option<bool>,
}

// =============================================================================
// ALERTS (Story 140.6)
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct PortfolioAlert {
    pub id: Uuid,
    /// NULL once the rule that raised the alert is deleted
    /// (`portfolio_alerts.rule_id ... ON DELETE SET NULL`).
    pub rule_id: Option<Uuid>,
    pub organization_id: Uuid,
    pub building_id: Option<Uuid>,

    pub metric_name: Option<String>,
    pub current_value: Decimal,
    pub threshold_value: Option<Decimal>,
    pub deviation_pct: Option<Decimal>,

    pub severity: String,

    pub title: String,
    pub message: Option<String>,

    pub is_read: bool,
    pub is_resolved: bool,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolved_by: Option<Uuid>,
    pub resolution_notes: Option<String>,

    pub notifications_sent: Option<JsonValue>,

    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AcknowledgeAlert {
    pub is_read: bool,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ResolveAlert {
    pub resolution_notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AlertStats {
    pub total_alerts: i64,
    pub unread_alerts: i64,
    pub unresolved_alerts: i64,
    pub critical_alerts: i64,
}

// =============================================================================
// DASHBOARD (Story 140.7)
// =============================================================================

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PortfolioAnalyticsDashboard {
    pub summary: PortfolioSummary,
    pub benchmark_performance: Vec<BenchmarkPerformance>,
    pub top_performers: Vec<PropertyRanking>,
    pub underperformers: Vec<PropertyRanking>,
    pub recent_alerts: Vec<PortfolioAlert>,
    pub revenue_trend: Vec<TrendDataPoint>,
    pub occupancy_trend: Vec<TrendDataPoint>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BenchmarkPerformance {
    pub benchmark_name: String,
    pub category: BenchmarkCategory,
    pub target_value: Decimal,
    pub actual_value: Decimal,
    pub variance_pct: Decimal,
    pub status: String, // above_target, on_target, below_target
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct PortfolioAnalyticsQuery {
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub building_ids: Option<Vec<Uuid>>,
    pub period_type: Option<AggregationPeriod>,
}
