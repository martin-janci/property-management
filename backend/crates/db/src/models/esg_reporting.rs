// Epic 136: ESG Reporting Dashboard
// Models for ESG (Environmental, Social, Governance) reporting and compliance

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

// =============================================================================
// Enums
// =============================================================================

/// ESG metric category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "esg_metric_category", rename_all = "snake_case")]
pub enum EsgMetricCategory {
    Environmental,
    Social,
    Governance,
}

/// Emission scope type (GHG Protocol scopes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "esg_emission_scope")]
pub enum EsgEmissionScope {
    // Explicit renames: the PG enum values carry an underscore before the digit
    // (`scope_2_indirect`), which `rename_all = "snake_case"` (heck) would emit
    // as `scope2_indirect` — breaking both encode and decode round-trips.
    #[sqlx(rename = "scope_1_direct")]
    Scope1Direct,
    #[sqlx(rename = "scope_2_indirect")]
    Scope2Indirect,
    #[sqlx(rename = "scope_3_value_chain")]
    Scope3ValueChain,
}

/// Energy source type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "energy_source_type", rename_all = "snake_case")]
pub enum EnergySourceType {
    ElectricityGrid,
    NaturalGas,
    HeatingOil,
    DistrictHeating,
    SolarPv,
    Wind,
    Geothermal,
    Biomass,
}

/// Data entry method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "esg_data_entry_method", rename_all = "snake_case")]
pub enum EsgDataEntryMethod {
    Manual,
    CsvImport,
    ApiIntegration,
    SmartMeter,
    Calculated,
}

/// Compliance framework.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "esg_compliance_framework", rename_all = "snake_case")]
pub enum EsgComplianceFramework {
    EuTaxonomy,
    Sfdr,
    Csrd,
    Gresb,
    Leed,
    Breeam,
    Iso14001,
    GhgProtocol,
}

/// Benchmark category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "esg_benchmark_category", rename_all = "snake_case")]
pub enum EsgBenchmarkCategory {
    IndustryAverage,
    RegionalAverage,
    BestInClass,
    RegulatoryMinimum,
    InternalTarget,
}

/// ESG report status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "esg_report_status", rename_all = "snake_case")]
pub enum EsgReportStatus {
    Draft,
    PendingReview,
    Approved,
    Published,
    Archived,
}

// =============================================================================
// ESG Configuration
// =============================================================================

/// ESG configuration for an organization.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct EsgConfiguration {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub reporting_currency: String,
    pub default_unit_system: String,
    pub fiscal_year_start_month: i32,
    pub enabled_frameworks: serde_json::Value,
    pub grid_emission_factor: Option<Decimal>,
    pub natural_gas_emission_factor: Option<Decimal>,
    pub carbon_reduction_target_pct: Option<Decimal>,
    pub target_year: Option<i32>,
    pub baseline_year: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create ESG configuration request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateEsgConfiguration {
    pub reporting_currency: Option<String>,
    pub default_unit_system: Option<String>,
    pub fiscal_year_start_month: Option<i32>,
    pub enabled_frameworks: Option<Vec<EsgComplianceFramework>>,
    pub grid_emission_factor: Option<Decimal>,
    pub natural_gas_emission_factor: Option<Decimal>,
    pub carbon_reduction_target_pct: Option<Decimal>,
    pub target_year: Option<i32>,
    pub baseline_year: Option<i32>,
}

/// Update ESG configuration request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateEsgConfiguration {
    pub reporting_currency: Option<String>,
    pub default_unit_system: Option<String>,
    pub fiscal_year_start_month: Option<i32>,
    pub enabled_frameworks: Option<Vec<EsgComplianceFramework>>,
    pub grid_emission_factor: Option<Decimal>,
    pub natural_gas_emission_factor: Option<Decimal>,
    pub carbon_reduction_target_pct: Option<Decimal>,
    pub target_year: Option<i32>,
    pub baseline_year: Option<i32>,
}

// =============================================================================
// ESG Metrics
// =============================================================================

/// ESG metric data point.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct EsgMetric {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub building_id: Option<Uuid>,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub category: EsgMetricCategory,
    pub metric_type: String,
    pub metric_name: String,
    pub value: Decimal,
    pub unit: String,
    pub normalized_value: Option<Decimal>,
    pub data_source: EsgDataEntryMethod,
    pub confidence_level: Option<i32>,
    pub verification_status: Option<String>,
    pub verified_by: Option<Uuid>,
    pub verified_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
    pub supporting_documents: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Uuid,
}

/// Create ESG metric request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateEsgMetric {
    pub building_id: Option<Uuid>,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub category: EsgMetricCategory,
    pub metric_type: String,
    pub metric_name: String,
    pub value: Decimal,
    pub unit: String,
    pub normalized_value: Option<Decimal>,
    pub data_source: EsgDataEntryMethod,
    pub confidence_level: Option<i32>,
    pub notes: Option<String>,
}

/// Update ESG metric request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateEsgMetric {
    pub value: Option<Decimal>,
    pub unit: Option<String>,
    pub normalized_value: Option<Decimal>,
    pub confidence_level: Option<i32>,
    pub notes: Option<String>,
}

// =============================================================================
// Carbon Footprint
// =============================================================================

/// Carbon footprint record.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct CarbonFootprint {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub building_id: Option<Uuid>,
    pub year: i32,
    pub month: Option<i32>,
    pub source_type: EsgEmissionScope,
    pub energy_source: Option<EnergySourceType>,
    pub consumption_value: Decimal,
    pub consumption_unit: String,
    pub emission_factor: Decimal,
    pub co2_equivalent_kg: Decimal,
    pub area_sqm: Option<Decimal>,
    pub co2_per_sqm: Option<Decimal>,
    pub num_units: Option<i32>,
    pub co2_per_unit: Option<Decimal>,
    pub calculation_methodology: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create carbon footprint request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateCarbonFootprint {
    pub building_id: Option<Uuid>,
    pub year: i32,
    pub month: Option<i32>,
    pub source_type: EsgEmissionScope,
    pub energy_source: Option<EnergySourceType>,
    pub consumption_value: Decimal,
    pub consumption_unit: String,
    pub emission_factor: Decimal,
    pub area_sqm: Option<Decimal>,
    pub num_units: Option<i32>,
}

/// Carbon footprint summary.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct CarbonFootprintSummary {
    pub year: i32,
    pub total_co2_kg: Decimal,
    pub scope_1_kg: Option<Decimal>,
    pub scope_2_kg: Option<Decimal>,
    pub scope_3_kg: Option<Decimal>,
    pub avg_co2_per_sqm: Option<Decimal>,
}

// =============================================================================
// ESG Benchmarks
// =============================================================================

/// ESG benchmark.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct EsgBenchmark {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub category: EsgBenchmarkCategory,
    pub metric_type: String,
    pub benchmark_value: Decimal,
    pub unit: String,
    pub region: Option<String>,
    pub property_type: Option<String>,
    pub source: Option<String>,
    pub effective_date: NaiveDate,
    pub expiry_date: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
}

/// Create ESG benchmark request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateEsgBenchmark {
    pub name: String,
    pub category: EsgBenchmarkCategory,
    pub metric_type: String,
    pub benchmark_value: Decimal,
    pub unit: String,
    pub region: Option<String>,
    pub property_type: Option<String>,
    pub source: Option<String>,
    pub effective_date: NaiveDate,
    pub expiry_date: Option<NaiveDate>,
}

// =============================================================================
// ESG Targets
// =============================================================================

/// ESG target.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct EsgTarget {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub building_id: Option<Uuid>,
    pub name: String,
    pub category: EsgMetricCategory,
    pub metric_type: String,
    pub target_value: Decimal,
    pub unit: String,
    pub target_date: NaiveDate,
    pub baseline_value: Option<Decimal>,
    pub baseline_date: Option<NaiveDate>,
    pub current_value: Option<Decimal>,
    pub progress_pct: Option<Decimal>,
    pub status: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create ESG target request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateEsgTarget {
    pub building_id: Option<Uuid>,
    pub name: String,
    pub category: EsgMetricCategory,
    pub metric_type: String,
    pub target_value: Decimal,
    pub unit: String,
    pub target_date: NaiveDate,
    pub baseline_value: Option<Decimal>,
    pub baseline_date: Option<NaiveDate>,
}

/// Update ESG target request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateEsgTarget {
    pub target_value: Option<Decimal>,
    pub target_date: Option<NaiveDate>,
    pub current_value: Option<Decimal>,
    pub status: Option<String>,
}

// =============================================================================
// ESG Reports
// =============================================================================

/// ESG compliance report.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct EsgReport {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub report_type: String,
    pub title: String,
    pub description: Option<String>,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub frameworks: serde_json::Value,
    pub status: EsgReportStatus,
    pub submitted_at: Option<DateTime<Utc>>,
    pub approved_by: Option<Uuid>,
    pub approved_at: Option<DateTime<Utc>>,
    pub report_data: Option<serde_json::Value>,
    pub summary_scores: Option<serde_json::Value>,
    pub pdf_url: Option<String>,
    pub xml_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Uuid,
}

/// Create ESG report request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateEsgReport {
    pub report_type: String,
    pub title: String,
    pub description: Option<String>,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub frameworks: Vec<EsgComplianceFramework>,
}

/// Update ESG report request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateEsgReport {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<EsgReportStatus>,
    pub report_data: Option<serde_json::Value>,
    pub summary_scores: Option<serde_json::Value>,
}

// =============================================================================
// EU Taxonomy Assessment
// =============================================================================

/// EU Taxonomy alignment assessment.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct EuTaxonomyAssessment {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub building_id: Option<Uuid>,
    pub year: i32,
    pub climate_mitigation_eligible: Option<bool>,
    pub climate_mitigation_aligned: Option<bool>,
    pub climate_mitigation_revenue_pct: Option<Decimal>,
    pub climate_adaptation_eligible: Option<bool>,
    pub climate_adaptation_aligned: Option<bool>,
    pub climate_adaptation_revenue_pct: Option<Decimal>,
    pub energy_performance_class: Option<String>,
    pub primary_energy_demand: Option<Decimal>,
    pub meets_nzeb_standard: Option<bool>,
    pub dnsh_water: Option<bool>,
    pub dnsh_circular_economy: Option<bool>,
    pub dnsh_pollution: Option<bool>,
    pub dnsh_biodiversity: Option<bool>,
    pub oecd_guidelines_compliance: Option<bool>,
    pub un_guiding_principles: Option<bool>,
    pub overall_alignment_pct: Option<Decimal>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create EU Taxonomy assessment request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateEuTaxonomyAssessment {
    pub building_id: Option<Uuid>,
    pub year: i32,
    pub climate_mitigation_eligible: Option<bool>,
    pub climate_mitigation_aligned: Option<bool>,
    pub climate_adaptation_eligible: Option<bool>,
    pub climate_adaptation_aligned: Option<bool>,
    pub energy_performance_class: Option<String>,
    pub primary_energy_demand: Option<Decimal>,
    pub meets_nzeb_standard: Option<bool>,
    pub notes: Option<String>,
}

/// Update EU Taxonomy assessment request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateEuTaxonomyAssessment {
    pub climate_mitigation_eligible: Option<bool>,
    pub climate_mitigation_aligned: Option<bool>,
    pub climate_adaptation_eligible: Option<bool>,
    pub climate_adaptation_aligned: Option<bool>,
    pub energy_performance_class: Option<String>,
    pub primary_energy_demand: Option<Decimal>,
    pub meets_nzeb_standard: Option<bool>,
    pub dnsh_water: Option<bool>,
    pub dnsh_circular_economy: Option<bool>,
    pub dnsh_pollution: Option<bool>,
    pub dnsh_biodiversity: Option<bool>,
    pub oecd_guidelines_compliance: Option<bool>,
    pub un_guiding_principles: Option<bool>,
    pub notes: Option<String>,
}

// =============================================================================
// ESG Dashboard
// =============================================================================

/// ESG dashboard metrics (cached aggregations).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct EsgDashboardMetrics {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub building_id: Option<Uuid>,
    pub year: i32,
    pub month: Option<i32>,
    pub environmental_score: Option<Decimal>,
    pub social_score: Option<Decimal>,
    pub governance_score: Option<Decimal>,
    pub overall_esg_score: Option<Decimal>,
    pub total_co2_kg: Option<Decimal>,
    pub co2_per_sqm: Option<Decimal>,
    pub energy_intensity: Option<Decimal>,
    pub water_intensity: Option<Decimal>,
    pub waste_diversion_rate: Option<Decimal>,
    pub renewable_energy_pct: Option<Decimal>,
    pub yoy_co2_change_pct: Option<Decimal>,
    pub benchmark_comparison: Option<serde_json::Value>,
    pub compliance_alerts: Option<serde_json::Value>,
    pub calculated_at: DateTime<Utc>,
}

/// ESG summary scores.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EsgSummaryScores {
    pub environmental_score: Decimal,
    pub social_score: Decimal,
    pub governance_score: Decimal,
    pub overall_score: Decimal,
}

/// ESG benchmark comparison.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EsgBenchmarkComparison {
    pub metric_name: String,
    pub current_value: Decimal,
    pub benchmark_value: Decimal,
    pub benchmark_category: EsgBenchmarkCategory,
    pub percentile_rank: Option<i32>,
    pub status: String, // above, at, below
}

// =============================================================================
// Import Jobs
// =============================================================================

/// ESG data import job.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct EsgImportJob {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub file_name: String,
    pub file_url: Option<String>,
    pub data_type: String,
    pub status: String,
    pub rows_total: Option<i32>,
    pub rows_processed: Option<i32>,
    pub rows_failed: Option<i32>,
    pub error_log: Option<serde_json::Value>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub created_by: Uuid,
}

/// Create import job request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateEsgImportJob {
    pub file_name: String,
    pub file_url: Option<String>,
    pub data_type: String,
    pub rows_total: Option<i32>,
}

// =============================================================================
// Query Types
// =============================================================================

/// ESG metrics query parameters.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EsgMetricsQuery {
    pub building_id: Option<Uuid>,
    pub category: Option<EsgMetricCategory>,
    pub metric_type: Option<String>,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

/// Carbon footprint query parameters.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CarbonFootprintQuery {
    pub building_id: Option<Uuid>,
    pub year: Option<i32>,
    pub source_type: Option<EsgEmissionScope>,
}

/// ESG statistics.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct EsgStatistics {
    pub total_metrics: i64,
    pub total_buildings_tracked: i64,
    pub latest_esg_score: Option<Decimal>,
    pub total_co2_current_year: Option<Decimal>,
    pub yoy_co2_change: Option<Decimal>,
    pub reports_published: i64,
}

/// Calculate carbon footprint request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CalculateCarbonFootprintRequest {
    pub building_id: Option<Uuid>,
    pub year: i32,
    pub month: Option<i32>,
}

/// Generate ESG report request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GenerateEsgReportRequest {
    pub report_id: Uuid,
}
