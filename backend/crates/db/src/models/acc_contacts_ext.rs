//! ACC contact CRM extension models (EPIC-ACC-03).
//!
//! The base `contact` row remains [`crate::models::accounting::Contact`]; these
//! types cover the CRM extensions in migration 00195 (`acc_contact_address`,
//! `acc_contact_tag`) plus an extended contact projection that includes the
//! per-contact default columns added by ALTER on `contact`.
//!
//! NOTE: the base `Contact` struct (models/accounting.rs) is owned by Foundation
//! and intentionally NOT changed; coders read the new columns via
//! [`AccContactExt`] which `SELECT`s the extended column set.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Billing/delivery address for a contact (UC-ACC-03.4).
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct AccContactAddress {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub contact_id: Uuid,
    pub kind: String,
    pub label: Option<String>,
    pub street: Option<String>,
    pub city: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Tag/segment on a contact (UC-ACC-03.6).
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct AccContactTag {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub contact_id: Uuid,
    pub tag: String,
    pub created_at: DateTime<Utc>,
}

/// Extended contact projection including the per-contact default columns
/// added to `contact` by migration 00195 (UC-ACC-03.1/.5/.9).
///
/// Field order matches a `SELECT` from `contact` after the base 00184 columns;
/// repositories that need the extended fields query this type.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct AccContactExt {
    // Base 00184 columns
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub ico: Option<String>,
    pub dic: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub iban: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // 00195 extension columns
    pub kind: String,
    pub is_active: bool,
    pub default_currency: Option<String>,
    pub default_price_level: Option<String>,
    pub default_payment_terms_days: Option<i32>,
    pub default_discount_pct: Option<Decimal>,
    pub default_language: Option<String>,
    pub vat_verified_at: Option<DateTime<Utc>>,
    pub vat_verified_valid: Option<bool>,
    pub credit_limit: Option<Decimal>,
    pub notes: Option<String>,
    pub merged_into_id: Option<Uuid>,
}
