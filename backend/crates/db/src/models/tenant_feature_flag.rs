//! Per-tenant feature flag model (Phase 3, defends leak #22).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// One per-(org, flag_key) row in `tenant_feature_flags`.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct TenantFeatureFlag {
    pub organization_id: Uuid,
    pub flag_key: String,
    pub enabled: bool,
    pub value: Option<Value>,
    pub updated_at: DateTime<Utc>,
    pub updated_by: Option<Uuid>,
}

/// Upsert payload for `PUT /admin/tenants/{org_id}/feature-flags`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpsertTenantFeatureFlag {
    pub flag_key: String,
    pub enabled: bool,
    #[serde(default)]
    pub value: Option<Value>,
}
