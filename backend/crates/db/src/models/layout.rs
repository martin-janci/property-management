use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct LayoutConfigRow {
    pub id: Uuid,
    pub screen: String,
    pub draft: serde_json::Value,
    pub published: Option<serde_json::Value>,
    pub published_version: i32,
    pub rails: serde_json::Value,
    pub updated_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct LayoutConfigVersionRow {
    pub id: Uuid,
    pub screen: String,
    pub version: i32,
    pub config: serde_json::Value,
    pub published_by: Option<Uuid>,
    pub published_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct LayoutTenantOverrideRow {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub screen: String,
    pub override_config: serde_json::Value,
    pub updated_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct LayoutKillFlagRow {
    pub screen: String,
    pub section_type: String,
    pub killed_by: Option<Uuid>,
    pub killed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct LayoutRegistryManifestRow {
    pub platform: String,
    pub manifest: serde_json::Value,
    pub updated_by: Option<Uuid>,
    pub updated_at: DateTime<Utc>,
}
