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

impl SupportedCurrency {
    /// Every `SupportedCurrency` variant, in declaration (discriminant) order.
    ///
    /// This is the Rust-side source of truth for *which* currencies exist and
    /// *how many* there are. `ALL`'s completeness is **compiler-enforced** by
    /// the two zero-dependency const guards directly below: adding a variant
    /// without extending this array is a const-eval compile error, not a
    /// silently-passing test. Keep the order here identical to the `enum`
    /// declaration above (the round-trip guard enforces this).
    ///
    /// When adding/removing a currency, update this array plus the Postgres
    /// enum (`00101_create_multi_currency.sql`) and the TypeSpec enum. Only the
    /// Postgres/TypeSpec mirrors still rely on the `enum_sync_guard` tests — the
    /// compiler now keeps `ALL` itself honest.
    pub const ALL: [SupportedCurrency; 13] = [
        SupportedCurrency::EUR,
        SupportedCurrency::CZK,
        SupportedCurrency::CHF,
        SupportedCurrency::GBP,
        SupportedCurrency::PLN,
        SupportedCurrency::USD,
        SupportedCurrency::HUF,
        SupportedCurrency::RON,
        SupportedCurrency::BGN,
        SupportedCurrency::HRK,
        SupportedCurrency::SEK,
        SupportedCurrency::DKK,
        SupportedCurrency::NOK,
    ];
}

// -----------------------------------------------------------------------------
// `SupportedCurrency::ALL` completeness — compiler-enforced (Issues #2104, #2124)
// -----------------------------------------------------------------------------
// Two zero-dependency const guards make `ALL` a faithful mirror of the enum at
// *compile time*, closing the #2124 gap where a variant added to the enum could
// be omitted from `ALL` while every `enum_sync_guard` test still passed. Each
// variant is a fieldless unit variant, so `v as usize` is its declaration-order
// discriminant (== its expected slot in `ALL`).
//
// 1. Completeness: the match is exhaustive, so adding a variant forces a new
//    arm here. Each arm returns `ALL[<variant> as usize]`; the new variant's
//    discriminant equals its expected slot, so forgetting to extend `ALL` makes
//    that index out of bounds — a hard compile error via the deny-by-default
//    `unconditional_panic` lint (see the `const _: ()` round-trip below for the
//    ordering half). When you fix it, also update the Postgres + TypeSpec enums.
// 2. Ordering: the round-trip asserts `ALL[i] as usize == i`, pinning `ALL` to
//    the enum's discriminant order, so reordering `ALL` — or inserting a variant
//    mid-enum without reordering `ALL` — is also a const-eval compile error.
//
// The guard is a never-called `const _: fn(..) -> ..` (not a free `const fn`,
// which would warn as dead code); it exists only so the compiler checks every
// arm's `ALL` index at build time.
const _: fn(SupportedCurrency) -> SupportedCurrency = |c| match c {
    SupportedCurrency::EUR => SupportedCurrency::ALL[SupportedCurrency::EUR as usize],
    SupportedCurrency::CZK => SupportedCurrency::ALL[SupportedCurrency::CZK as usize],
    SupportedCurrency::CHF => SupportedCurrency::ALL[SupportedCurrency::CHF as usize],
    SupportedCurrency::GBP => SupportedCurrency::ALL[SupportedCurrency::GBP as usize],
    SupportedCurrency::PLN => SupportedCurrency::ALL[SupportedCurrency::PLN as usize],
    SupportedCurrency::USD => SupportedCurrency::ALL[SupportedCurrency::USD as usize],
    SupportedCurrency::HUF => SupportedCurrency::ALL[SupportedCurrency::HUF as usize],
    SupportedCurrency::RON => SupportedCurrency::ALL[SupportedCurrency::RON as usize],
    SupportedCurrency::BGN => SupportedCurrency::ALL[SupportedCurrency::BGN as usize],
    SupportedCurrency::HRK => SupportedCurrency::ALL[SupportedCurrency::HRK as usize],
    SupportedCurrency::SEK => SupportedCurrency::ALL[SupportedCurrency::SEK as usize],
    SupportedCurrency::DKK => SupportedCurrency::ALL[SupportedCurrency::DKK as usize],
    SupportedCurrency::NOK => SupportedCurrency::ALL[SupportedCurrency::NOK as usize],
};
const _: () = {
    let mut i = 0;
    while i < SupportedCurrency::ALL.len() {
        assert!(
            SupportedCurrency::ALL[i] as usize == i,
            "SupportedCurrency::ALL is not in enum declaration (discriminant) order"
        );
        i += 1;
    }
};

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

impl std::fmt::Display for CountryCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CountryCode::SK => write!(f, "SK"),
            CountryCode::CZ => write!(f, "CZ"),
            CountryCode::AT => write!(f, "AT"),
            CountryCode::DE => write!(f, "DE"),
            CountryCode::PL => write!(f, "PL"),
            CountryCode::HU => write!(f, "HU"),
            CountryCode::CH => write!(f, "CH"),
            CountryCode::GB => write!(f, "GB"),
            CountryCode::FR => write!(f, "FR"),
            CountryCode::IT => write!(f, "IT"),
            CountryCode::ES => write!(f, "ES"),
            CountryCode::NL => write!(f, "NL"),
            CountryCode::BE => write!(f, "BE"),
            CountryCode::PT => write!(f, "PT"),
            CountryCode::IE => write!(f, "IE"),
            CountryCode::RO => write!(f, "RO"),
            CountryCode::BG => write!(f, "BG"),
            CountryCode::HR => write!(f, "HR"),
            CountryCode::SI => write!(f, "SI"),
            CountryCode::LU => write!(f, "LU"),
            CountryCode::SE => write!(f, "SE"),
            CountryCode::DK => write!(f, "DK"),
            CountryCode::NO => write!(f, "NO"),
            CountryCode::FI => write!(f, "FI"),
        }
    }
}

/// Error returned when a string cannot be parsed into a [`CountryCode`].
///
/// Mirrors [`ParseSupportedCurrencyError`]: it carries the offending
/// (uppercased) input and only signals "not a supported country code".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseCountryCodeError(pub String);

impl std::fmt::Display for ParseCountryCodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "'{}' is not a supported ISO 3166-1 alpha-2 country code",
            self.0
        )
    }
}

impl std::error::Error for ParseCountryCodeError {}

impl std::str::FromStr for CountryCode {
    type Err = ParseCountryCodeError;

    /// Parse a case-insensitive ISO 3166-1 alpha-2 code into a [`CountryCode`].
    ///
    /// Input is uppercased before matching so `"sk"` and `"SK"` both resolve.
    /// This mirrors the [`Display`](std::fmt::Display) impl above so the
    /// supported set stays defined in exactly one place.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let upper = s.trim().to_ascii_uppercase();
        match upper.as_str() {
            "SK" => Ok(CountryCode::SK),
            "CZ" => Ok(CountryCode::CZ),
            "AT" => Ok(CountryCode::AT),
            "DE" => Ok(CountryCode::DE),
            "PL" => Ok(CountryCode::PL),
            "HU" => Ok(CountryCode::HU),
            "CH" => Ok(CountryCode::CH),
            "GB" => Ok(CountryCode::GB),
            "FR" => Ok(CountryCode::FR),
            "IT" => Ok(CountryCode::IT),
            "ES" => Ok(CountryCode::ES),
            "NL" => Ok(CountryCode::NL),
            "BE" => Ok(CountryCode::BE),
            "PT" => Ok(CountryCode::PT),
            "IE" => Ok(CountryCode::IE),
            "RO" => Ok(CountryCode::RO),
            "BG" => Ok(CountryCode::BG),
            "HR" => Ok(CountryCode::HR),
            "SI" => Ok(CountryCode::SI),
            "LU" => Ok(CountryCode::LU),
            "SE" => Ok(CountryCode::SE),
            "DK" => Ok(CountryCode::DK),
            "NO" => Ok(CountryCode::NO),
            "FI" => Ok(CountryCode::FI),
            _ => Err(ParseCountryCodeError(upper)),
        }
    }
}

impl CountryCode {
    /// Every `CountryCode` variant, in declaration (discriminant) order.
    ///
    /// This is the Rust-side source of truth for *which* countries exist and
    /// *how many* there are. `ALL`'s completeness is **compiler-enforced** by
    /// the two zero-dependency const guards directly below: adding a variant
    /// without extending this array is a const-eval compile error, not a
    /// silently-passing test. Keep the order here identical to the `enum`
    /// declaration above (the round-trip guard enforces this).
    ///
    /// When adding/removing a country, update this array plus the Postgres enum
    /// (`00101_create_multi_currency.sql`) and the TypeSpec enum. Only the
    /// Postgres/TypeSpec mirrors still rely on the `enum_sync_guard` tests — the
    /// compiler now keeps `ALL` itself honest.
    pub const ALL: [CountryCode; 24] = [
        CountryCode::SK,
        CountryCode::CZ,
        CountryCode::AT,
        CountryCode::DE,
        CountryCode::PL,
        CountryCode::HU,
        CountryCode::CH,
        CountryCode::GB,
        CountryCode::FR,
        CountryCode::IT,
        CountryCode::ES,
        CountryCode::NL,
        CountryCode::BE,
        CountryCode::PT,
        CountryCode::IE,
        CountryCode::RO,
        CountryCode::BG,
        CountryCode::HR,
        CountryCode::SI,
        CountryCode::LU,
        CountryCode::SE,
        CountryCode::DK,
        CountryCode::NO,
        CountryCode::FI,
    ];
}

// -----------------------------------------------------------------------------
// `CountryCode::ALL` completeness — compiler-enforced (Issues #2104, #2124)
// -----------------------------------------------------------------------------
// Same two zero-dependency const guards as `SupportedCurrency` above (see that
// block for the full rationale): the exhaustive completeness match reads
// `ALL[<variant> as usize]` per arm, so a variant added to the enum but omitted
// from `ALL` is an out-of-bounds const index (hard compile error), and the
// round-trip pins `ALL` to the enum's discriminant order. Like the currency
// guard, this is a never-called `const _: fn(..) -> ..` checked at build time.
const _: fn(CountryCode) -> CountryCode = |c| match c {
    CountryCode::SK => CountryCode::ALL[CountryCode::SK as usize],
    CountryCode::CZ => CountryCode::ALL[CountryCode::CZ as usize],
    CountryCode::AT => CountryCode::ALL[CountryCode::AT as usize],
    CountryCode::DE => CountryCode::ALL[CountryCode::DE as usize],
    CountryCode::PL => CountryCode::ALL[CountryCode::PL as usize],
    CountryCode::HU => CountryCode::ALL[CountryCode::HU as usize],
    CountryCode::CH => CountryCode::ALL[CountryCode::CH as usize],
    CountryCode::GB => CountryCode::ALL[CountryCode::GB as usize],
    CountryCode::FR => CountryCode::ALL[CountryCode::FR as usize],
    CountryCode::IT => CountryCode::ALL[CountryCode::IT as usize],
    CountryCode::ES => CountryCode::ALL[CountryCode::ES as usize],
    CountryCode::NL => CountryCode::ALL[CountryCode::NL as usize],
    CountryCode::BE => CountryCode::ALL[CountryCode::BE as usize],
    CountryCode::PT => CountryCode::ALL[CountryCode::PT as usize],
    CountryCode::IE => CountryCode::ALL[CountryCode::IE as usize],
    CountryCode::RO => CountryCode::ALL[CountryCode::RO as usize],
    CountryCode::BG => CountryCode::ALL[CountryCode::BG as usize],
    CountryCode::HR => CountryCode::ALL[CountryCode::HR as usize],
    CountryCode::SI => CountryCode::ALL[CountryCode::SI as usize],
    CountryCode::LU => CountryCode::ALL[CountryCode::LU as usize],
    CountryCode::SE => CountryCode::ALL[CountryCode::SE as usize],
    CountryCode::DK => CountryCode::ALL[CountryCode::DK as usize],
    CountryCode::NO => CountryCode::ALL[CountryCode::NO as usize],
    CountryCode::FI => CountryCode::ALL[CountryCode::FI as usize],
};
const _: () = {
    let mut i = 0;
    while i < CountryCode::ALL.len() {
        assert!(
            CountryCode::ALL[i] as usize == i,
            "CountryCode::ALL is not in enum declaration (discriminant) order"
        );
        i += 1;
    }
};

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

// =============================================================================
// ENUM SYNC GUARD (Issue #2083, hardened in #2104)
// =============================================================================
//
// `SupportedCurrency` and `CountryCode` are hand-maintained in THREE places:
//   1. Postgres enums  → backend/crates/db/migrations/00101_create_multi_currency.sql
//   2. Rust enums      → this file (the canonical source of truth)
//   3. TypeSpec enums  → docs/api/typespec/shared/models.tsp (drives OpenAPI)
//
// These tests turn "three sources of truth" into "one source + checked
// mirrors": they hard-code the canonical ordered wire values and assert the
// Rust enums serialize to exactly that list, in that order. The `match` inside
// each canonical helper is EXHAUSTIVE — adding a variant without updating the
// canonical list is a compile error, which is the whole point of the guard.
//
// #2104: the count/serialization assertions used to be driven purely off the
// hand-written `list` vecs, so an enum that grew *past* its canonical list
// (variant added, exhaustive `_wire`/`Display` matches updated, but the `list`
// left untouched) was undetectable — the count tests just counted the stale
// list. #2104 tied the assertions to `Self::ALL` and added per-variant
// membership checks in both directions.
//
// #2124: `Self::ALL`'s *completeness* is now compiler-enforced next to each
// enum — the completeness guard indexes `ALL[<variant> as usize]` for every
// (exhaustively matched) variant, and the round-trip asserts `ALL[i] as usize
// == i`, so a variant added to the enum but omitted from (or misordered in)
// `ALL` is a const-eval compile error, not a silently-passing build. Because
// `ALL` is compiler-guaranteed complete, the tests below are genuine enum-vs-
// mirror checks: they compare the (now-trustworthy) `ALL` against the canonical
// wire list, catching Postgres/TypeSpec drift that the compiler cannot see.
//
// If a currency/country is added or removed, update ALL THREE files above plus
// `Self::ALL` and the canonical arrays here, and these tests keep the mirrors
// honest.
#[cfg(test)]
mod enum_sync_guard {
    use super::*;
    use std::str::FromStr;

    /// Canonical ordered wire values for `SupportedCurrency`, matching the
    /// `supported_currency` Postgres enum and the TypeSpec `SupportedCurrency`
    /// enum (migration `00101_create_multi_currency.sql`).
    ///
    /// The exhaustive `match` guarantees every variant appears exactly once —
    /// adding a variant fails to compile until this list is updated.
    fn currency_canonical() -> Vec<(SupportedCurrency, &'static str)> {
        use SupportedCurrency::*;
        // Exhaustiveness guard: a new variant breaks this match at compile time.
        fn _wire(c: SupportedCurrency) -> &'static str {
            match c {
                EUR => "EUR",
                CZK => "CZK",
                CHF => "CHF",
                GBP => "GBP",
                PLN => "PLN",
                USD => "USD",
                HUF => "HUF",
                RON => "RON",
                BGN => "BGN",
                HRK => "HRK",
                SEK => "SEK",
                DKK => "DKK",
                NOK => "NOK",
            }
        }
        let list = vec![
            (EUR, "EUR"),
            (CZK, "CZK"),
            (CHF, "CHF"),
            (GBP, "GBP"),
            (PLN, "PLN"),
            (USD, "USD"),
            (HUF, "HUF"),
            (RON, "RON"),
            (BGN, "BGN"),
            (HRK, "HRK"),
            (SEK, "SEK"),
            (DKK, "DKK"),
            (NOK, "NOK"),
        ];
        // Every listed value must round-trip through the exhaustive `_wire`.
        for (variant, wire) in &list {
            assert_eq!(_wire(*variant), *wire);
        }
        list
    }

    /// Canonical ordered wire values for `CountryCode`, matching the
    /// `country_code` Postgres enum and the TypeSpec `CountryCode` enum.
    fn country_canonical() -> Vec<(CountryCode, &'static str)> {
        use CountryCode::*;
        // Exhaustiveness guard: a new variant breaks this match at compile time.
        fn _wire(c: CountryCode) -> &'static str {
            match c {
                SK => "SK",
                CZ => "CZ",
                AT => "AT",
                DE => "DE",
                PL => "PL",
                HU => "HU",
                CH => "CH",
                GB => "GB",
                FR => "FR",
                IT => "IT",
                ES => "ES",
                NL => "NL",
                BE => "BE",
                PT => "PT",
                IE => "IE",
                RO => "RO",
                BG => "BG",
                HR => "HR",
                SI => "SI",
                LU => "LU",
                SE => "SE",
                DK => "DK",
                NO => "NO",
                FI => "FI",
            }
        }
        let list = vec![
            (SK, "SK"),
            (CZ, "CZ"),
            (AT, "AT"),
            (DE, "DE"),
            (PL, "PL"),
            (HU, "HU"),
            (CH, "CH"),
            (GB, "GB"),
            (FR, "FR"),
            (IT, "IT"),
            (ES, "ES"),
            (NL, "NL"),
            (BE, "BE"),
            (PT, "PT"),
            (IE, "IE"),
            (RO, "RO"),
            (BG, "BG"),
            (HR, "HR"),
            (SI, "SI"),
            (LU, "LU"),
            (SE, "SE"),
            (DK, "DK"),
            (NO, "NO"),
            (FI, "FI"),
        ];
        for (variant, wire) in &list {
            assert_eq!(_wire(*variant), *wire);
        }
        list
    }

    #[test]
    fn supported_currency_count_is_stable() {
        // The Postgres enum and TypeSpec enum both declare exactly 13 values.
        assert_eq!(
            currency_canonical().len(),
            13,
            "SupportedCurrency count drifted from the 13-value canonical list"
        );
        // The *enum* (via `ALL`, which the #2124 completeness + round-trip const
        // guards force to cover every variant, in order) must agree with the
        // canonical wire list. This catches the enum and the canonical/Postgres/
        // TypeSpec list drifting apart — the count above alone cannot.
        assert_eq!(
            SupportedCurrency::ALL.len(),
            currency_canonical().len(),
            "SupportedCurrency::ALL and the canonical wire list disagree — a \
             variant was added/removed without updating both"
        );
    }

    #[test]
    fn country_code_count_is_stable() {
        // The Postgres enum and TypeSpec enum both declare exactly 24 values.
        assert_eq!(
            country_canonical().len(),
            24,
            "CountryCode count drifted from the 24-value canonical list"
        );
        // The *enum* (via `ALL`, which the #2124 completeness + round-trip const
        // guards force to cover every variant, in order) must agree with the
        // canonical wire list. This catches the enum and the canonical/Postgres/
        // TypeSpec list drifting apart — the count above alone cannot.
        assert_eq!(
            CountryCode::ALL.len(),
            country_canonical().len(),
            "CountryCode::ALL and the canonical wire list disagree — a variant \
             was added/removed without updating both"
        );
    }

    #[test]
    fn supported_currency_all_matches_canonical_list() {
        let list = currency_canonical();
        // Every enum variant (from `ALL`) must appear in the canonical list…
        for variant in SupportedCurrency::ALL {
            assert!(
                list.iter().any(|(v, _)| *v == variant),
                "{variant:?} is a SupportedCurrency variant missing from the \
                 canonical wire list"
            );
        }
        // …and every canonical entry must be a real variant present in `ALL`.
        for (variant, _) in &list {
            assert!(
                SupportedCurrency::ALL.contains(variant),
                "{variant:?} is in the canonical list but not in \
                 SupportedCurrency::ALL"
            );
        }
    }

    #[test]
    fn country_code_all_matches_canonical_list() {
        let list = country_canonical();
        // Every enum variant (from `ALL`) must appear in the canonical list…
        for variant in CountryCode::ALL {
            assert!(
                list.iter().any(|(v, _)| *v == variant),
                "{variant:?} is a CountryCode variant missing from the \
                 canonical wire list"
            );
        }
        // …and every canonical entry must be a real variant present in `ALL`.
        for (variant, _) in &list {
            assert!(
                CountryCode::ALL.contains(variant),
                "{variant:?} is in the canonical list but not in CountryCode::ALL"
            );
        }
    }

    #[test]
    fn supported_currency_serializes_to_canonical_wire_values() {
        for (variant, wire) in currency_canonical() {
            // serde (rename_all = "UPPERCASE") must emit the canonical wire value.
            assert_eq!(
                serde_json::to_string(&variant).unwrap(),
                format!("\"{wire}\""),
                "serde serialization for {variant:?} drifted"
            );
            // Display must agree with the wire value.
            assert_eq!(variant.to_string(), wire, "Display for {variant:?} drifted");
        }
    }

    #[test]
    fn country_code_serializes_to_canonical_wire_values() {
        for (variant, wire) in country_canonical() {
            assert_eq!(
                serde_json::to_string(&variant).unwrap(),
                format!("\"{wire}\""),
                "serde serialization for {variant:?} drifted"
            );
            assert_eq!(variant.to_string(), wire, "Display for {variant:?} drifted");
        }
    }

    #[test]
    fn supported_currency_deserializes_from_canonical_wire_values() {
        for (variant, wire) in currency_canonical() {
            let json = format!("\"{wire}\"");
            let parsed: SupportedCurrency = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, variant, "serde deserialization for {wire} drifted");
        }
    }

    #[test]
    fn country_code_deserializes_from_canonical_wire_values() {
        for (variant, wire) in country_canonical() {
            let json = format!("\"{wire}\"");
            let parsed: CountryCode = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, variant, "serde deserialization for {wire} drifted");
        }
    }

    #[test]
    fn supported_currency_from_str_round_trips() {
        for (variant, wire) in currency_canonical() {
            // FromStr(Display) == identity.
            assert_eq!(SupportedCurrency::from_str(wire).unwrap(), variant);
            // Case-insensitive parse resolves to the same variant.
            assert_eq!(
                SupportedCurrency::from_str(&wire.to_ascii_lowercase()).unwrap(),
                variant
            );
        }
    }

    #[test]
    fn country_code_from_str_round_trips() {
        for (variant, wire) in country_canonical() {
            assert_eq!(CountryCode::from_str(wire).unwrap(), variant);
            assert_eq!(
                CountryCode::from_str(&wire.to_ascii_lowercase()).unwrap(),
                variant
            );
        }
    }

    #[test]
    fn supported_currency_rejects_unknown_code() {
        // Unknown ISO-4217-shaped input must error, not silently map.
        let err = SupportedCurrency::from_str("XXX").unwrap_err();
        assert_eq!(err, ParseSupportedCurrencyError("XXX".to_string()));
        assert!(SupportedCurrency::from_str("").is_err());
        assert!(SupportedCurrency::from_str("EURO").is_err());
    }

    #[test]
    fn country_code_rejects_unknown_code() {
        // Unknown ISO-3166-shaped input must error, not silently map.
        let err = CountryCode::from_str("XX").unwrap_err();
        assert_eq!(err, ParseCountryCodeError("XX".to_string()));
        assert!(CountryCode::from_str("").is_err());
        assert!(CountryCode::from_str("USA").is_err());
    }
}
