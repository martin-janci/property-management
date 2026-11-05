//! IoT sensor models (Epic 14).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Sensor type enum.
#[allow(missing_docs)]
pub mod sensor_type {
    pub const TEMPERATURE: &str = "temperature";
    pub const HUMIDITY: &str = "humidity";
    pub const MOTION: &str = "motion";
    pub const CO2: &str = "co2";
    pub const WATER_LEAK: &str = "water_leak";
    pub const ENERGY: &str = "energy";
    pub const SMOKE: &str = "smoke";
    pub const DOOR: &str = "door";
    pub const WINDOW: &str = "window";
    pub const LIGHT: &str = "light";
    pub const PRESSURE: &str = "pressure";
    pub const NOISE: &str = "noise";
    pub const AIR_QUALITY: &str = "air_quality";
    pub const OCCUPANCY: &str = "occupancy";

    /// Every value the `sensors.sensor_type` CHECK constraint accepts.
    pub const ALL: &[&str] = &[
        TEMPERATURE,
        HUMIDITY,
        MOTION,
        CO2,
        WATER_LEAK,
        ENERGY,
        SMOKE,
        DOOR,
        WINDOW,
        LIGHT,
        PRESSURE,
        NOISE,
        AIR_QUALITY,
        OCCUPANCY,
    ];
}

/// Sensor connection type enum (`sensors.connection_type` CHECK constraint).
#[allow(missing_docs)]
pub mod sensor_connection_type {
    pub const API: &str = "api";
    pub const MQTT: &str = "mqtt";
    pub const WEBHOOK: &str = "webhook";
    pub const POLLING: &str = "polling";

    /// Every value the `sensors.connection_type` CHECK constraint accepts.
    pub const ALL: &[&str] = &[API, MQTT, WEBHOOK, POLLING];
}

/// Sensor status enum.
#[allow(missing_docs)]
pub mod sensor_status {
    pub const PENDING: &str = "pending";
    pub const ACTIVE: &str = "active";
    pub const OFFLINE: &str = "offline";
    pub const ERROR: &str = "error";
    pub const MAINTENANCE: &str = "maintenance";
    pub const DISABLED: &str = "disabled";

    /// Every value the `sensors.status` CHECK constraint accepts.
    pub const ALL: &[&str] = &[PENDING, ACTIVE, OFFLINE, ERROR, MAINTENANCE, DISABLED];
}

/// IoT sensor entity.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Sensor {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub building_id: Uuid,
    pub unit_id: Option<Uuid>,
    pub name: String,
    pub sensor_type: String,
    pub location: Option<String>,
    pub location_description: Option<String>,
    pub connection_type: String,
    #[sqlx(json)]
    pub connection_config: serde_json::Value,
    pub api_key_hash: Option<String>,
    pub unit_of_measurement: Option<String>,
    pub data_interval_seconds: Option<i32>,
    pub status: String,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub last_reading_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub error_count: Option<i32>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub firmware_version: Option<String>,
    pub serial_number: Option<String>,
    pub installed_at: Option<DateTime<Utc>>,
    #[sqlx(json)]
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Uuid,
}

/// Create sensor request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateSensor {
    /// Ignored on input: the route sets it from the validated tenant.
    #[serde(default)]
    pub organization_id: Uuid,
    pub building_id: Uuid,
    pub unit_id: Option<Uuid>,
    pub name: String,
    pub sensor_type: String,
    pub location: Option<String>,
    pub location_description: Option<String>,
    pub connection_type: Option<String>,
    pub connection_config: Option<serde_json::Value>,
    pub unit_of_measurement: Option<String>,
    pub data_interval_seconds: Option<i32>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub firmware_version: Option<String>,
    pub serial_number: Option<String>,
    pub installed_at: Option<DateTime<Utc>>,
    pub metadata: Option<serde_json::Value>,
    /// Ignored on input: the route sets it from the authenticated user.
    #[serde(default)]
    pub created_by: Uuid,
}

/// Update sensor request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateSensor {
    pub name: Option<String>,
    pub location: Option<String>,
    pub location_description: Option<String>,
    pub connection_type: Option<String>,
    pub connection_config: Option<serde_json::Value>,
    pub unit_of_measurement: Option<String>,
    pub data_interval_seconds: Option<i32>,
    pub status: Option<String>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub firmware_version: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// Sensor query parameters.
#[derive(Debug, Clone, Serialize, Deserialize, Default, utoipa::IntoParams)]
pub struct SensorQuery {
    pub building_id: Option<Uuid>,
    pub unit_id: Option<Uuid>,
    pub sensor_type: Option<String>,
    pub status: Option<String>,
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Sensor summary for listings.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct SensorSummary {
    pub id: Uuid,
    pub name: String,
    pub sensor_type: String,
    pub status: String,
    pub building_id: Uuid,
    pub last_reading_at: Option<DateTime<Utc>>,
    pub last_value: Option<f64>,
}

// ============================================================================
// Sensor Readings
// ============================================================================

/// Sensor reading entity.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct SensorReading {
    pub id: Uuid,
    pub sensor_id: Uuid,
    pub value: f64,
    pub unit: String,
    pub quality: Option<String>,
    // `raw_data` is a nullable JSONB column. Plain `#[sqlx(json)]` decodes via
    // `Json<T>`, which rejects a SQL NULL ("unexpected null"); `json(nullable)`
    // maps NULL -> None and decodes the JSON otherwise (same pattern as
    // `llm_document.rs`). A reading ingested without `raw_data` stores NULL and
    // must decode back through `RETURNING *`.
    #[sqlx(json(nullable))]
    pub raw_data: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
}

/// Create sensor reading request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateSensorReading {
    pub sensor_id: Uuid,
    pub value: f64,
    pub unit: String,
    pub quality: Option<String>,
    pub raw_data: Option<serde_json::Value>,
    pub timestamp: Option<DateTime<Utc>>,
}

/// Batch reading submission.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BatchSensorReadings {
    pub sensor_id: Uuid,
    pub readings: Vec<SingleReading>,
}

/// Single reading in a batch.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SingleReading {
    pub value: f64,
    pub unit: String,
    pub quality: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// Reading query parameters.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReadingQuery {
    pub from_time: Option<DateTime<Utc>>,
    pub to_time: Option<DateTime<Utc>>,
    pub aggregation: Option<String>,
    pub limit: Option<i64>,
}

/// Aggregated sensor data.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct AggregatedReading {
    pub period: DateTime<Utc>,
    pub min_value: f64,
    pub max_value: f64,
    pub avg_value: f64,
    pub count: i64,
}

// ============================================================================
// Sensor Thresholds
// ============================================================================

/// Sensor threshold entity.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct SensorThreshold {
    pub id: Uuid,
    pub sensor_id: Uuid,
    pub metric: String,
    pub comparison: String,
    pub warning_value: Option<f64>,
    pub warning_high: Option<f64>,
    pub critical_value: Option<f64>,
    pub critical_high: Option<f64>,
    pub enabled: bool,
    pub alert_cooldown_minutes: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create threshold request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateSensorThreshold {
    pub sensor_id: Uuid,
    pub metric: Option<String>,
    pub comparison: String,
    pub warning_value: Option<f64>,
    pub warning_high: Option<f64>,
    pub critical_value: Option<f64>,
    pub critical_high: Option<f64>,
    pub alert_cooldown_minutes: Option<i32>,
}

/// Update threshold request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateSensorThreshold {
    pub comparison: Option<String>,
    pub warning_value: Option<f64>,
    pub warning_high: Option<f64>,
    pub critical_value: Option<f64>,
    pub critical_high: Option<f64>,
    pub enabled: Option<bool>,
    pub alert_cooldown_minutes: Option<i32>,
}

// ============================================================================
// Sensor Alerts
// ============================================================================

/// Sensor alert entity.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct SensorAlert {
    pub id: Uuid,
    pub sensor_id: Uuid,
    pub threshold_id: Uuid,
    pub severity: String,
    pub triggered_value: f64,
    pub threshold_value: f64,
    pub message: Option<String>,
    pub triggered_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolved_value: Option<f64>,
    pub acknowledged_by: Option<Uuid>,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub notification_sent: bool,
}

/// Create alert request (typically system-generated).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSensorAlert {
    pub sensor_id: Uuid,
    pub threshold_id: Uuid,
    pub severity: String,
    pub triggered_value: f64,
    pub threshold_value: f64,
    pub message: Option<String>,
}

/// Alert query parameters.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AlertQuery {
    pub sensor_id: Option<Uuid>,
    pub building_id: Option<Uuid>,
    pub severity: Option<String>,
    pub resolved: Option<bool>,
    pub acknowledged: Option<bool>,
    pub from_time: Option<DateTime<Utc>>,
    pub to_time: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ============================================================================
// Threshold Templates
// ============================================================================

/// Threshold template entity.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct SensorThresholdTemplate {
    pub id: Uuid,
    pub organization_id: Option<Uuid>,
    pub sensor_type: String,
    pub name: String,
    pub description: Option<String>,
    pub comparison: String,
    pub warning_value: Option<f64>,
    pub warning_high: Option<f64>,
    pub critical_value: Option<f64>,
    pub critical_high: Option<f64>,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// Sensor-Fault Correlations
// ============================================================================

/// Sensor-fault correlation entity.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct SensorFaultCorrelation {
    pub id: Uuid,
    pub sensor_id: Uuid,
    pub fault_id: Uuid,
    pub correlation_type: String,
    pub confidence: Option<f64>,
    pub sensor_data_start: Option<DateTime<Utc>>,
    pub sensor_data_end: Option<DateTime<Utc>>,
    pub summary: Option<String>,
    pub created_at: DateTime<Utc>,
    pub created_by: Option<Uuid>,
}

/// Create correlation request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateSensorFaultCorrelation {
    pub sensor_id: Uuid,
    pub fault_id: Uuid,
    pub correlation_type: Option<String>,
    pub confidence: Option<f64>,
    pub sensor_data_start: Option<DateTime<Utc>>,
    pub sensor_data_end: Option<DateTime<Utc>>,
    pub summary: Option<String>,
    pub created_by: Option<Uuid>,
}

// ============================================================================
// Dashboard Types
// ============================================================================

/// Sensor dashboard data.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SensorDashboard {
    pub total_sensors: i64,
    pub active_sensors: i64,
    pub offline_sensors: i64,
    pub unresolved_alerts: i64,
    pub sensors_by_type: Vec<SensorTypeCount>,
    pub recent_alerts: Vec<SensorAlert>,
}

/// Sensor count by type.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct SensorTypeCount {
    pub sensor_type: String,
    pub count: i64,
}
