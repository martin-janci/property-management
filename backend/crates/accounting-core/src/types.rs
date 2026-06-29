//! Shared value types used across accounting-core modules.
//!
//! Foundation-owned and stable: money/vat/rounding (BE-Invoicing) and numbering
//! (BE-Config) consume these. If you need a new shared type, request it via
//! `contract-notes/<role>.md` rather than editing this file; add per-epic types
//! in your own module meanwhile.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// How totals are rounded. Maps to `acc_company_settings.rounding_mode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoundingMode {
    /// Standard half-up arithmetic rounding.
    #[default]
    Arithmetic,
    /// Banker's rounding (round half to even).
    Bankers,
    /// No rounding (keep full precision).
    None,
}

/// A single computed invoice line, the input to VAT grouping and totals.
///
/// `qty * unit_price` (less discount) gives the net base; VAT is `base * rate%`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineInput {
    /// Quantity (up to 4 dp).
    pub qty: Decimal,
    /// Net unit price (exclusive of VAT).
    pub unit_price: Decimal,
    /// VAT rate as a percentage, e.g. `20` for 20%.
    pub vat_rate: Decimal,
    /// Line-level discount percentage applied before VAT (0..=100), or zero.
    pub discount_pct: Decimal,
}

/// Computed monetary breakdown of a line or document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Money {
    /// Net (taxable base) amount.
    pub base: Decimal,
    /// VAT amount.
    pub vat: Decimal,
    /// Gross (base + vat) amount.
    pub total: Decimal,
}

/// VAT subtotal for one rate, used to render the "VAT breakdown by rate" panel
/// and to feed VAT records (UC-ACC-05.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VatGroup {
    /// VAT rate as a percentage, e.g. `20`.
    pub rate: Decimal,
    /// Net base summed across lines at this rate.
    pub base: Decimal,
    /// VAT summed across lines at this rate.
    pub vat: Decimal,
}
