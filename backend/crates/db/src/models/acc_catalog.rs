//! ACC product & price-list catalog models (EPIC-ACC-04).
//!
//! Structs match migration 00195 (`acc_item_category`, `acc_catalog_item`,
//! `acc_price_level`). Column names verified against 00195.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Catalog category (UC-ACC-04.4).
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct AccItemCategory {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Reusable catalog item (UC-ACC-04.1/.3/.7).
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct AccCatalogItem {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub code: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub kind: String,
    pub unit_id: Option<Uuid>,
    pub vat_rate: Decimal,
    pub unit_price: Decimal,
    pub price_is_gross: bool,
    pub category_id: Option<Uuid>,
    pub stock_tracked: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Named price level per catalog item (UC-ACC-04.2).
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct AccPriceLevel {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub item_id: Uuid,
    pub name: String,
    pub price: Decimal,
    pub price_is_gross: bool,
    pub currency: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
