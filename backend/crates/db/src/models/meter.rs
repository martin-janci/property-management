//! Meter and utility management models (Epic 12).
//!
//! Provides types for meters, readings, utility bills, and consumption tracking.

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

// ============================================================================
// ENUMS
// ============================================================================

/// Type of meter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "meter_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum MeterType {
    Electricity,
    Gas,
    Water,
    Heat,
    ColdWater,
    HotWater,
    Solar,
    Other,
}

/// Source of meter reading.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, sqlx::Type, ToSchema,
)]
#[sqlx(type_name = "reading_source", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ReadingSource {
    #[default]
    Manual,
    Photo,
    Automatic,
    Estimated,
}

/// Status of meter reading.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, sqlx::Type, ToSchema,
)]
#[sqlx(type_name = "reading_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ReadingStatus {
    #[default]
    Pending,
    Approved,
    Rejected,
    Estimated,
}

/// Method for distributing utility costs.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, sqlx::Type, ToSchema,
)]
#[sqlx(type_name = "distribution_method", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum DistributionMethod {
    #[default]
    Consumption,
    Area,
    Equal,
    Occupants,
    Hybrid,
}

// ============================================================================
// METERS (Story 12.1)
// ============================================================================

/// Meter entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct Meter {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub building_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_id: Option<Uuid>,
    pub meter_number: String,
    pub meter_type: MeterType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub initial_reading: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_reading: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_reading_date: Option<NaiveDate>,
    pub unit_of_measure: String,
    pub is_smart_meter: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smart_meter_provider: Option<String>,
    pub is_active: bool,
    pub is_shared: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed_at: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decommissioned_at: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replaced_meter_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement_reading: Option<Decimal>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to register a meter.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RegisterMeter {
    pub building_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_id: Option<Uuid>,
    pub meter_number: String,
    pub meter_type: MeterType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub initial_reading: Decimal,
    #[serde(default = "default_unit_of_measure")]
    pub unit_of_measure: String,
    #[serde(default)]
    pub is_shared: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed_at: Option<NaiveDate>,
}

fn default_unit_of_measure() -> String {
    "kWh".to_string()
}

/// Request to replace a meter.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ReplaceMeter {
    pub final_reading: Decimal,
    pub new_meter_number: String,
    pub new_initial_reading: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_location: Option<String>,
}

// ============================================================================
// METER READINGS (Story 12.2)
// ============================================================================

/// Meter reading entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct MeterReading {
    pub id: Uuid,
    pub meter_id: Uuid,
    pub reading: Decimal,
    pub reading_date: NaiveDate,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reading_time: Option<NaiveTime>,
    pub source: ReadingSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub photo_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ocr_reading: Option<Decimal>,
    pub status: ReadingStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validated_by: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validated_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consumption: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_reading_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub submission_window_id: Option<Uuid>,
    pub is_anomaly: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anomaly_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub submitted_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to submit a meter reading.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SubmitReading {
    pub meter_id: Uuid,
    pub reading: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reading_date: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reading_time: Option<NaiveTime>,
    #[serde(default)]
    pub source: ReadingSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub photo_url: Option<String>,
}

/// Request to approve or reject a reading.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ValidateReading {
    pub status: ReadingStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub corrected_reading: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// Submission window entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct ReadingSubmissionWindow {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub building_id: Uuid,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub billing_period_start: NaiveDate,
    pub billing_period_end: NaiveDate,
    pub submission_start: NaiveDate,
    pub submission_end: NaiveDate,
    pub is_open: bool,
    pub is_finalized: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finalized_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finalized_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to create a submission window.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateSubmissionWindow {
    pub building_id: Uuid,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub billing_period_start: NaiveDate,
    pub billing_period_end: NaiveDate,
    pub submission_start: NaiveDate,
    pub submission_end: NaiveDate,
}

// ============================================================================
// VALIDATION RULES (Story 12.3)
// ============================================================================

/// Reading validation rule.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct ReadingValidationRule {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub meter_type: MeterType,
    pub rule_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_consumption_threshold: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_consumption_threshold: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_increase_percentage: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_decrease_percentage: Option<Decimal>,
    pub comparison_months: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ============================================================================
// UTILITY BILLS (Story 12.4)
// ============================================================================

/// Utility bill entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct UtilityBill {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub building_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bill_number: Option<String>,
    pub meter_type: MeterType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_name: Option<String>,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub total_amount: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_consumption: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_price: Option<Decimal>,
    pub currency: String,
    pub distribution_method: DistributionMethod,
    pub is_distributed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distributed_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distributed_by: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bill_file_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to create a utility bill.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateUtilityBill {
    pub building_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bill_number: Option<String>,
    pub meter_type: MeterType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_name: Option<String>,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub total_amount: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_consumption: Option<Decimal>,
    #[serde(default = "default_currency")]
    pub currency: String,
    #[serde(default)]
    pub distribution_method: DistributionMethod,
}

fn default_currency() -> String {
    "EUR".to_string()
}

/// Utility bill distribution to unit.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct UtilityBillDistribution {
    pub id: Uuid,
    pub utility_bill_id: Uuid,
    pub unit_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consumption: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consumption_percentage: Option<Decimal>,
    pub amount: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub area_factor: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub occupant_factor: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoice_item_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

/// Request to distribute a utility bill.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct DistributeUtilityBill {
    pub distribution_method: DistributionMethod,
    /// Override distribution for specific units
    #[serde(default)]
    pub unit_overrides: Vec<UnitDistributionOverride>,
}

/// Override for unit distribution.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UnitDistributionOverride {
    pub unit_id: Uuid,
    pub amount: Decimal,
}

// ============================================================================
// CONSUMPTION ANALYTICS (Story 12.5)
// ============================================================================

/// Pre-calculated consumption aggregate.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct ConsumptionAggregate {
    pub id: Uuid,
    pub meter_id: Uuid,
    pub year: i32,
    pub month: i32,
    pub total_consumption: Decimal,
    pub reading_count: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_daily_consumption: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_reading: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_reading: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub building_avg_consumption: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percentile_rank: Option<Decimal>,
    pub calculated_at: DateTime<Utc>,
}

/// Consumption history response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ConsumptionHistoryResponse {
    pub meter_id: Uuid,
    pub meter_type: MeterType,
    pub unit_of_measure: String,
    pub history: Vec<ConsumptionDataPoint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub building_average: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comparison: Option<ConsumptionComparison>,
}

/// Single data point for consumption history.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ConsumptionDataPoint {
    pub date: NaiveDate,
    pub reading: Decimal,
    pub consumption: Decimal,
}

/// Consumption comparison data.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ConsumptionComparison {
    pub period: String,
    pub current_consumption: Decimal,
    pub previous_consumption: Decimal,
    pub change_percentage: Decimal,
    pub vs_building_average: Decimal,
}

// ============================================================================
// SMART METERS (Story 12.6)
// ============================================================================

/// Smart meter provider configuration.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct SmartMeterProvider {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub provider_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub polling_interval_minutes: Option<i32>,
    pub is_active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_sync_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    pub error_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Smart meter reading log.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct SmartMeterReadingLog {
    pub id: Uuid,
    pub meter_id: Uuid,
    pub provider_id: Uuid,
    pub reading: Decimal,
    pub reading_timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_data: Option<serde_json::Value>,
    pub processed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meter_reading_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    pub received_at: DateTime<Utc>,
}

/// API request to ingest smart meter reading.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct IngestSmartMeterReading {
    pub meter_number: String,
    pub reading: Decimal,
    pub reading_timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_data: Option<serde_json::Value>,
}

/// Missing reading alert.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct MissingReadingAlert {
    pub id: Uuid,
    pub meter_id: Uuid,
    pub expected_date: NaiveDate,
    pub alert_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub is_resolved: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_by: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution_notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// RESPONSES
// ============================================================================

/// Response with meter details.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MeterResponse {
    pub meter: Meter,
    pub recent_readings: Vec<MeterReading>,
}

/// Response listing meters.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ListMetersResponse {
    pub meters: Vec<Meter>,
    pub total: i64,
}

/// Response listing readings.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ListReadingsResponse {
    pub readings: Vec<MeterReading>,
    pub total: i64,
}

/// Reading approval queue entry.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ReadingApprovalEntry {
    pub reading: MeterReading,
    pub meter: Meter,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_number: Option<String>,
    pub building_name: String,
}

/// Row type for reading approval query.
#[derive(Debug, Clone, FromRow)]
pub struct ReadingApprovalRow {
    // Reading fields
    pub reading_id: Uuid,
    pub reading_meter_id: Uuid,
    pub reading_value: Decimal,
    pub reading_date: NaiveDate,
    pub reading_time: Option<NaiveTime>,
    pub reading_source: ReadingSource,
    pub photo_url: Option<String>,
    pub ocr_reading: Option<Decimal>,
    pub reading_status: ReadingStatus,
    pub validated_by: Option<Uuid>,
    pub validated_at: Option<chrono::DateTime<Utc>>,
    pub validation_notes: Option<String>,
    pub consumption: Option<Decimal>,
    pub previous_reading_id: Option<Uuid>,
    pub submission_window_id: Option<Uuid>,
    pub is_anomaly: bool,
    pub anomaly_reason: Option<String>,
    pub submitted_by: Option<Uuid>,
    pub reading_created_at: chrono::DateTime<Utc>,
    pub reading_updated_at: chrono::DateTime<Utc>,
    // Meter fields
    pub meter_id: Uuid,
    pub meter_number: String,
    pub meter_type: MeterType,
    pub unit_of_measure: String,
    // Context fields
    pub unit_number: Option<String>,
    pub building_name: String,
}

/// Response with utility bill distribution.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UtilityBillResponse {
    pub bill: UtilityBill,
    pub distributions: Vec<UtilityBillDistribution>,
}

// ============================================================================
// OCR METER CORRECTIONS (Story 128.1)
// ============================================================================

/// Persisted OCR correction feedback record.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct OcrMeterCorrection {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub submitted_by: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meter_reading_id: Option<Uuid>,
    pub original_value: Decimal,
    pub corrected_value: Decimal,
    pub image_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bounding_box: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Request to persist an OCR correction.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateOcrCorrection {
    pub organization_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meter_reading_id: Option<Uuid>,
    pub original_value: Decimal,
    pub corrected_value: Decimal,
    pub image_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bounding_box: Option<serde_json::Value>,
}
