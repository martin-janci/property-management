//! Multi-Currency & Cross-Border Support models (Epic 145).
//! Provides multi-currency configuration, exchange rate management,
//! cross-currency transactions, cross-border lease management, and consolidated reporting.

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

/// Supported currencies for multi-currency operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, sqlx::Type)]
#[sqlx(type_name = "supported_currency", rename_all = "UPPERCASE")]
#[serde(rename_all = "UPPERCASE")]
#[derive(Default)]
pub enum SupportedCurrency {
    #[default]
    EUR, // Euro (base for most EU)
    CZK, // Czech Koruna
    CHF, // Swiss Franc
    GBP, // British Pound
    PLN, // Polish Zloty
    USD, // US Dollar
    HUF, // Hungarian Forint
    RON, // Romanian Leu
    BGN, // Bulgarian Lev
    HRK, // Croatian Kuna (legacy)
    SEK, // Swedish Krona
    DKK, // Danish Krone
    NOK, // Norwegian Krone
}

impl std::fmt::Display for SupportedCurrency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SupportedCurrency::EUR => write!(f, "EUR"),
            SupportedCurrency::CZK => write!(f, "CZK"),
            SupportedCurrency::CHF => write!(f, "CHF"),
            SupportedCurrency::GBP => write!(f, "GBP"),
            SupportedCurrency::PLN => write!(f, "PLN"),
            SupportedCurrency::USD => write!(f, "USD"),
            SupportedCurrency::HUF => write!(f, "HUF"),
            SupportedCurrency::RON => write!(f, "RON"),
            SupportedCurrency::BGN => write!(f, "BGN"),
            SupportedCurrency::HRK => write!(f, "HRK"),
            SupportedCurrency::SEK => write!(f, "SEK"),
            SupportedCurrency::DKK => write!(f, "DKK"),
            SupportedCurrency::NOK => write!(f, "NOK"),
        }
    }
}

/// Error returned when a string cannot be parsed into a [`SupportedCurrency`].
///
/// Carries the offending (uppercased) input so callers can build a helpful
/// message. Kept intentionally small — it only signals "not a supported code".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseSupportedCurrencyError(pub String);

impl std::fmt::Display for ParseSupportedCurrencyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "'{}' is not a supported ISO-4217 currency code", self.0)
    }
}

impl std::error::Error for ParseSupportedCurrencyError {}

impl std::str::FromStr for SupportedCurrency {
    type Err = ParseSupportedCurrencyError;

    /// Parse a case-insensitive ISO-4217 code into a [`SupportedCurrency`].
    ///
    /// Input is uppercased before matching so `"eur"` and `"EUR"` both resolve.
    /// This mirrors the [`Display`](std::fmt::Display) impl above so the
    /// supported set stays defined in exactly one place.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let upper = s.trim().to_ascii_uppercase();
        match upper.as_str() {
            "EUR" => Ok(SupportedCurrency::EUR),
            "CZK" => Ok(SupportedCurrency::CZK),
            "CHF" => Ok(SupportedCurrency::CHF),
            "GBP" => Ok(SupportedCurrency::GBP),
            "PLN" => Ok(SupportedCurrency::PLN),
            "USD" => Ok(SupportedCurrency::USD),
            "HUF" => Ok(SupportedCurrency::HUF),
            "RON" => Ok(SupportedCurrency::RON),
            "BGN" => Ok(SupportedCurrency::BGN),
            "HRK" => Ok(SupportedCurrency::HRK),
            "SEK" => Ok(SupportedCurrency::SEK),
            "DKK" => Ok(SupportedCurrency::DKK),
            "NOK" => Ok(SupportedCurrency::NOK),
            _ => Err(ParseSupportedCurrencyError(upper)),
        }
    }
}

/// Exchange rate source
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, sqlx::Type)]
#[sqlx(type_name = "exchange_rate_source", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum ExchangeRateSource {
    #[default]
    Ecb, // European Central Bank
    Xe,     // XE.com
    Manual, // Manual override
    Api,    // Custom API integration
}

/// Transaction conversion status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, sqlx::Type)]
#[sqlx(type_name = "conversion_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum ConversionStatus {
    #[default]
    Pending,
    Converted,
    Failed,
    Manual,
}

/// Cross-border compliance status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, sqlx::Type)]
#[sqlx(type_name = "compliance_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum CrossBorderComplianceStatus {
    Compliant,
    #[default]
    PendingReview,
    NonCompliant,
    Exempt,
}

/// Country codes (ISO 3166-1 alpha-2)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, sqlx::Type)]
#[sqlx(type_name = "country_code", rename_all = "UPPERCASE")]
#[serde(rename_all = "UPPERCASE")]
pub enum CountryCode {
    SK, // Slovakia
    CZ, // Czech Republic
    AT, // Austria
    DE, // Germany
    PL, // Poland
    HU, // Hungary
    CH, // Switzerland
    GB, // United Kingdom
    FR, // France
    IT, // Italy
    ES, // Spain
    NL, // Netherlands
    BE, // Belgium
    PT, // Portugal
    IE, // Ireland
    RO, // Romania
    BG, // Bulgaria
    HR, // Croatia
    SI, // Slovenia
    LU, // Luxembourg
    SE, // Sweden
    DK, // Denmark
    NO, // Norway
    FI, // Finland
}

// =============================================================================
// STORY 145.1: MULTI-CURRENCY CONFIGURATION
// =============================================================================

/// Organization-level currency configuration
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct OrganizationCurrencyConfig {
    pub id: Uuid,
    pub organization_id: Uuid,

    /// Base currency for the organization
    pub base_currency: SupportedCurrency,

    /// Enabled additional currencies
    pub enabled_currencies: Vec<SupportedCurrency>,

    /// Default display currency (null = use base currency)
    pub display_currency: Option<SupportedCurrency>,
    pub show_original_amount: bool,
    pub decimal_places: i32,

    /// Exchange rate settings
    pub exchange_rate_source: ExchangeRateSource,
    pub auto_update_rates: bool,
    pub update_frequency_hours: i32,
    pub last_rate_update: Option<DateTime<Utc>>,

    /// Rounding mode
    pub rounding_mode: String,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateCurrencyConfig {
    #[serde(default)]
    pub base_currency: SupportedCurrency,
    #[serde(default = "default_enabled_currencies")]
    pub enabled_currencies: Vec<SupportedCurrency>,
    pub display_currency: Option<SupportedCurrency>,
    #[serde(default = "default_true")]
    pub show_original_amount: bool,
    #[serde(default = "default_decimal_places")]
    pub decimal_places: i32,
    #[serde(default)]
    pub exchange_rate_source: ExchangeRateSource,
    #[serde(default = "default_true")]
    pub auto_update_rates: bool,
    #[serde(default = "default_update_frequency")]
    pub update_frequency_hours: i32,
    #[serde(default = "default_rounding_mode")]
    pub rounding_mode: String,
}

fn default_enabled_currencies() -> Vec<SupportedCurrency> {
    vec![SupportedCurrency::EUR]
}

fn default_true() -> bool {
    true
}

fn default_decimal_places() -> i32 {
    2
}

fn default_update_frequency() -> i32 {
    24
}

fn default_rounding_mode() -> String {
    "half_up".to_string()
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateCurrencyConfig {
    pub base_currency: Option<SupportedCurrency>,
    pub enabled_currencies: Option<Vec<SupportedCurrency>>,
    pub display_currency: Option<SupportedCurrency>,
    pub show_original_amount: Option<bool>,
    pub decimal_places: Option<i32>,
    pub exchange_rate_source: Option<ExchangeRateSource>,
    pub auto_update_rates: Option<bool>,
    pub update_frequency_hours: Option<i32>,
    pub rounding_mode: Option<String>,
}

/// Property-level currency configuration
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct PropertyCurrencyConfig {
    pub id: Uuid,
    pub building_id: Uuid,
    pub organization_id: Uuid,

    pub default_currency: SupportedCurrency,
    pub country: CountryCode,

    /// Tax settings
    pub vat_rate: Option<Decimal>,
    pub vat_registration_number: Option<String>,
    pub local_tax_id: Option<String>,

    /// Compliance flags
    pub requires_local_reporting: bool,
    pub local_accounting_format: Option<String>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreatePropertyCurrencyConfig {
    pub building_id: Uuid,
    pub default_currency: SupportedCurrency,
    pub country: CountryCode,
    pub vat_rate: Option<Decimal>,
    pub vat_registration_number: Option<String>,
    pub local_tax_id: Option<String>,
    #[serde(default)]
    pub requires_local_reporting: bool,
    pub local_accounting_format: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdatePropertyCurrencyConfig {
    pub default_currency: Option<SupportedCurrency>,
    pub country: Option<CountryCode>,
    pub vat_rate: Option<Decimal>,
    pub vat_registration_number: Option<String>,
    pub local_tax_id: Option<String>,
    pub requires_local_reporting: Option<bool>,
    pub local_accounting_format: Option<String>,
}

// =============================================================================
// STORY 145.2: EXCHANGE RATE MANAGEMENT
// =============================================================================

/// Historical exchange rate record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct ExchangeRate {
    pub id: Uuid,
    pub from_currency: SupportedCurrency,
    pub to_currency: SupportedCurrency,
    pub rate: Decimal,
    pub inverse_rate: Decimal,
    pub rate_date: NaiveDate,
    pub source: ExchangeRateSource,
    pub source_reference: Option<String>,
    pub is_override: bool,
    pub override_reason: Option<String>,
    pub overridden_by: Option<Uuid>,
    pub valid_from: DateTime<Utc>,
    pub valid_until: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateExchangeRate {
    pub from_currency: SupportedCurrency,
    pub to_currency: SupportedCurrency,
    pub rate: Decimal,
    pub rate_date: NaiveDate,
    #[serde(default)]
    pub source: ExchangeRateSource,
    pub source_reference: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct OverrideExchangeRate {
    pub from_currency: SupportedCurrency,
    pub to_currency: SupportedCurrency,
    pub rate: Decimal,
    pub rate_date: NaiveDate,
    pub reason: String,
}

/// Exchange rate fetch log
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct ExchangeRateFetchLog {
    pub id: Uuid,
    pub organization_id: Option<Uuid>,
    pub source: ExchangeRateSource,
    pub fetch_time: DateTime<Utc>,
    pub success: bool,
    pub rates_fetched: Option<i32>,
    pub error_message: Option<String>,
    pub response_data: Option<JsonValue>,
    pub created_at: DateTime<Utc>,
}

/// Exchange rate query parameters
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ExchangeRateQuery {
    pub from_currency: Option<SupportedCurrency>,
    pub to_currency: Option<SupportedCurrency>,
    pub date: Option<NaiveDate>,
    pub date_from: Option<NaiveDate>,
    pub date_to: Option<NaiveDate>,
    pub source: Option<ExchangeRateSource>,
}

/// Exchange rate summary for a currency pair
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ExchangeRateSummary {
    pub from_currency: SupportedCurrency,
    pub to_currency: SupportedCurrency,
    pub current_rate: Decimal,
    pub rate_date: NaiveDate,
    pub source: ExchangeRateSource,
    pub change_24h: Option<Decimal>,
    pub change_7d: Option<Decimal>,
    pub change_30d: Option<Decimal>,
}

// =============================================================================
// STORY 145.3: CROSS-CURRENCY TRANSACTIONS
// =============================================================================

/// Multi-currency transaction record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct MultiCurrencyTransaction {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub building_id: Option<Uuid>,

    /// Reference to original transaction
    pub source_type: String,
    pub source_id: Uuid,

    /// Original amount
    pub original_currency: SupportedCurrency,
    pub original_amount: Decimal,

    /// Converted amount
    pub base_currency: SupportedCurrency,
    pub converted_amount: Decimal,

    /// Exchange rate used
    pub exchange_rate: Decimal,
    pub exchange_rate_id: Option<Uuid>,
    pub rate_date: NaiveDate,

    /// Conversion details
    pub conversion_status: ConversionStatus,
    pub conversion_timestamp: DateTime<Utc>,

    /// Manual override
    pub is_rate_override: bool,
    pub override_rate: Option<Decimal>,
    pub override_reason: Option<String>,
    pub overridden_by: Option<Uuid>,

    /// Realized gain/loss
    pub realized_gain_loss: Decimal,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateMultiCurrencyTransaction {
    pub building_id: Option<Uuid>,
    pub source_type: String,
    pub source_id: Uuid,
    pub original_currency: SupportedCurrency,
    pub original_amount: Decimal,
    pub rate_date: Option<NaiveDate>,
    pub override_rate: Option<Decimal>,
    pub override_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateTransactionRate {
    pub new_rate: Decimal,
    pub reason: String,
}

/// Currency conversion audit log
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct CurrencyConversionAudit {
    pub id: Uuid,
    pub transaction_id: Uuid,
    pub action: String,
    pub previous_rate: Option<Decimal>,
    pub new_rate: Option<Decimal>,
    pub previous_amount: Option<Decimal>,
    pub new_amount: Option<Decimal>,
    pub performed_by: Option<Uuid>,
    pub performed_at: DateTime<Utc>,
    pub notes: Option<String>,
}

/// Transaction query parameters
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct TransactionQuery {
    pub building_id: Option<Uuid>,
    pub source_type: Option<String>,
    pub currency: Option<SupportedCurrency>,
    pub date_from: Option<NaiveDate>,
    pub date_to: Option<NaiveDate>,
    pub status: Option<ConversionStatus>,
}

// =============================================================================
// STORY 145.4: CROSS-BORDER LEASE MANAGEMENT
// =============================================================================

/// Cross-border lease configuration
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct CrossBorderLease {
    pub id: Uuid,
    pub lease_id: Uuid,
    pub organization_id: Uuid,

    /// Property location
    pub property_country: CountryCode,
    pub property_currency: SupportedCurrency,

    /// Tenant information
    pub tenant_country: Option<CountryCode>,
    pub tenant_tax_id: Option<String>,
    pub tenant_vat_number: Option<String>,

    /// Lease currency settings
    pub lease_currency: SupportedCurrency,
    pub payment_currency: SupportedCurrency,

    /// Conversion rules
    pub convert_at_invoice_date: bool,
    pub convert_at_payment_date: bool,
    pub fixed_exchange_rate: Option<Decimal>,
    pub rate_lock_date: Option<NaiveDate>,

    /// Tax handling
    pub local_vat_applicable: bool,
    pub vat_rate: Option<Decimal>,
    pub reverse_charge_vat: bool,
    pub withholding_tax_rate: Option<Decimal>,

    /// Compliance
    pub compliance_status: CrossBorderComplianceStatus,
    pub compliance_notes: Option<String>,
    pub last_compliance_check: Option<DateTime<Utc>>,

    /// Country-specific clauses
    pub local_clauses: Option<JsonValue>,
    pub governing_law: Option<CountryCode>,
    pub jurisdiction: Option<CountryCode>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateCrossBorderLease {
    pub lease_id: Uuid,
    pub property_country: CountryCode,
    pub property_currency: SupportedCurrency,
    pub tenant_country: Option<CountryCode>,
    pub tenant_tax_id: Option<String>,
    pub tenant_vat_number: Option<String>,
    pub lease_currency: SupportedCurrency,
    pub payment_currency: SupportedCurrency,
    #[serde(default = "default_true")]
    pub convert_at_invoice_date: bool,
    #[serde(default)]
    pub convert_at_payment_date: bool,
    pub fixed_exchange_rate: Option<Decimal>,
    pub rate_lock_date: Option<NaiveDate>,
    #[serde(default = "default_true")]
    pub local_vat_applicable: bool,
    pub vat_rate: Option<Decimal>,
    #[serde(default)]
    pub reverse_charge_vat: bool,
    pub withholding_tax_rate: Option<Decimal>,
    pub local_clauses: Option<JsonValue>,
    pub governing_law: Option<CountryCode>,
    pub jurisdiction: Option<CountryCode>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateCrossBorderLease {
    pub tenant_country: Option<CountryCode>,
    pub tenant_tax_id: Option<String>,
    pub tenant_vat_number: Option<String>,
    pub payment_currency: Option<SupportedCurrency>,
    pub convert_at_invoice_date: Option<bool>,
    pub convert_at_payment_date: Option<bool>,
    pub fixed_exchange_rate: Option<Decimal>,
    pub rate_lock_date: Option<NaiveDate>,
    pub local_vat_applicable: Option<bool>,
    pub vat_rate: Option<Decimal>,
    pub reverse_charge_vat: Option<bool>,
    pub withholding_tax_rate: Option<Decimal>,
    pub compliance_status: Option<CrossBorderComplianceStatus>,
    pub compliance_notes: Option<String>,
    pub local_clauses: Option<JsonValue>,
    pub governing_law: Option<CountryCode>,
    pub jurisdiction: Option<CountryCode>,
}

/// Cross-border compliance requirements
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct CrossBorderComplianceRequirement {
    pub id: Uuid,
    pub country: CountryCode,
    pub requirement_type: String,
    pub requirement_name: String,
    pub description: Option<String>,
    pub threshold_amount: Option<Decimal>,
    pub threshold_currency: Option<SupportedCurrency>,
    pub reporting_frequency: Option<String>,
    pub reporting_deadline_days: Option<i32>,
    pub required_documents: Option<JsonValue>,
    pub is_active: bool,
    pub effective_from: NaiveDate,
    pub effective_until: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Cross-border lease query parameters
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CrossBorderLeaseQuery {
    pub property_country: Option<CountryCode>,
    pub lease_currency: Option<SupportedCurrency>,
    pub compliance_status: Option<CrossBorderComplianceStatus>,
}

// =============================================================================
// STORY 145.5: CONSOLIDATED MULTI-CURRENCY REPORTING
// =============================================================================

/// Multi-currency report configuration
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct MultiCurrencyReportConfig {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub description: Option<String>,

    /// Currency display
    pub report_currency: SupportedCurrency,
    pub show_original_currencies: bool,
    pub show_conversion_details: bool,

    /// Exchange rate for report
    pub rate_date_type: String,
    pub specific_rate_date: Option<NaiveDate>,

    /// Grouping
    pub group_by_currency: bool,
    pub group_by_country: bool,
    pub group_by_property: bool,

    /// Saved report
    pub is_saved: bool,
    pub is_default: bool,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateReportConfig {
    pub name: String,
    pub description: Option<String>,
    pub report_currency: SupportedCurrency,
    #[serde(default = "default_true")]
    pub show_original_currencies: bool,
    #[serde(default = "default_true")]
    pub show_conversion_details: bool,
    #[serde(default = "default_rate_date_type")]
    pub rate_date_type: String,
    pub specific_rate_date: Option<NaiveDate>,
    #[serde(default = "default_true")]
    pub group_by_currency: bool,
    #[serde(default)]
    pub group_by_country: bool,
    #[serde(default = "default_true")]
    pub group_by_property: bool,
    #[serde(default)]
    pub is_saved: bool,
    #[serde(default)]
    pub is_default: bool,
}

fn default_rate_date_type() -> String {
    "end_of_period".to_string()
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateReportConfig {
    pub name: Option<String>,
    pub description: Option<String>,
    pub report_currency: Option<SupportedCurrency>,
    pub show_original_currencies: Option<bool>,
    pub show_conversion_details: Option<bool>,
    pub rate_date_type: Option<String>,
    pub specific_rate_date: Option<NaiveDate>,
    pub group_by_currency: Option<bool>,
    pub group_by_country: Option<bool>,
    pub group_by_property: Option<bool>,
    pub is_saved: Option<bool>,
    pub is_default: Option<bool>,
}

/// Multi-currency report snapshot
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct MultiCurrencyReportSnapshot {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub config_id: Option<Uuid>,

    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub report_currency: SupportedCurrency,

    pub total_revenue: Decimal,
    pub total_expenses: Decimal,
    pub net_income: Decimal,

    /// Breakdowns stored as JSON
    pub currency_breakdown: JsonValue,
    pub exchange_rate_impact: Option<Decimal>,
    pub unrealized_fx_gain_loss: Option<Decimal>,
    pub realized_fx_gain_loss: Option<Decimal>,
    pub country_breakdown: Option<JsonValue>,
    pub property_breakdown: Option<JsonValue>,
    pub rates_used: JsonValue,
    pub rate_date: NaiveDate,

    pub generated_at: DateTime<Utc>,
    pub generated_by: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct GenerateReportRequest {
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub report_currency: SupportedCurrency,
    pub config_id: Option<Uuid>,
    pub rate_date: Option<NaiveDate>,
}

/// Currency exposure analysis
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct CurrencyExposureAnalysis {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub analysis_date: NaiveDate,
    pub currency: SupportedCurrency,

    pub receivables_amount: Decimal,
    pub payables_amount: Decimal,
    pub net_exposure: Decimal,
    pub asset_value: Decimal,
    pub projected_revenue: Decimal,
    pub projected_expenses: Decimal,

    pub value_at_risk: Option<Decimal>,
    pub expected_shortfall: Option<Decimal>,
    pub hedged_amount: Option<Decimal>,
    pub hedge_effectiveness: Option<Decimal>,

    pub created_at: DateTime<Utc>,
}

/// Currency breakdown entry for reports
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CurrencyBreakdown {
    pub currency: SupportedCurrency,
    pub revenue: Decimal,
    pub expenses: Decimal,
    pub net: Decimal,
    pub exchange_rate: Decimal,
    pub converted_revenue: Decimal,
    pub converted_expenses: Decimal,
    pub converted_net: Decimal,
}

/// Country breakdown entry for reports
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CountryBreakdown {
    pub country: CountryCode,
    pub currency: SupportedCurrency,
    pub revenue: Decimal,
    pub expenses: Decimal,
    pub property_count: i32,
}

/// Property breakdown entry for reports
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PropertyBreakdown {
    pub building_id: Uuid,
    pub building_name: String,
    pub currency: SupportedCurrency,
    pub revenue: Decimal,
    pub expenses: Decimal,
    pub net: Decimal,
}

// =============================================================================
// DASHBOARD & SUMMARY TYPES
// =============================================================================

/// Multi-currency dashboard summary
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MultiCurrencyDashboard {
    pub base_currency: SupportedCurrency,
    pub enabled_currencies: Vec<SupportedCurrency>,

    /// Currency summary
    pub currency_summaries: Vec<CurrencySummary>,

    /// Exchange rate info
    pub rate_last_updated: Option<DateTime<Utc>>,
    pub rate_source: ExchangeRateSource,

    /// Recent transactions
    pub recent_transactions: Vec<MultiCurrencyTransaction>,

    /// Exposure analysis
    pub exposure_by_currency: Vec<CurrencyExposureAnalysis>,

    /// Alerts
    pub rate_alerts: Vec<ExchangeRateAlert>,
}

/// Summary for a single currency
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CurrencySummary {
    pub currency: SupportedCurrency,
    pub total_receivables: Decimal,
    pub total_payables: Decimal,
    pub net_position: Decimal,
    pub property_count: i32,
    pub current_rate_to_base: Decimal,
    pub base_currency_value: Decimal,
}

/// Exchange rate alert
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ExchangeRateAlert {
    pub currency_pair: String,
    pub alert_type: String, // 'significant_change', 'rate_not_updated', 'override_expiring'
    pub message: String,
    pub severity: String,
    pub created_at: DateTime<Utc>,
}

/// Statistics for multi-currency operations
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MultiCurrencyStatistics {
    pub total_currencies_used: i32,
    pub total_transactions: i64,
    pub total_cross_border_leases: i64,
    pub total_fx_gain_loss: Decimal,
    pub currency_distribution: Vec<CurrencyDistribution>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CurrencyDistribution {
    pub currency: SupportedCurrency,
    pub transaction_count: i64,
    pub total_amount: Decimal,
    pub percentage: Decimal,
}
