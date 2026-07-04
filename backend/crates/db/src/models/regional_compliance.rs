//! Regional Legal Compliance models (Epic 72).
//!
//! Implements Slovak and Czech regional compliance features:
//! - Slovak voting quorum (zakon 182/1993 Z.z.)
//! - Slovak accounting export (POHODA, Money S3)
//! - Slovak GDPR (zakon 18/2018 Z.z.)
//! - Czech SVJ support (zakon 89/2012 Sb.)

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

// ============================================================================
// Jurisdiction Enum
// ============================================================================

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema, Default,
)]
#[sqlx(type_name = "jurisdiction", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Jurisdiction {
    #[default]
    Slovakia,
    Czechia,
}

// ============================================================================
// Slovak Decision Types (zakon 182/1993 Z.z.)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "sk_decision_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum SlovakDecisionType {
    /// Simple majority (50.01% of present votes)
    SimpleMajority,
    /// Two-thirds majority (66.67% of all owners)
    TwoThirdsMajority,
    /// Three-quarters majority (75% of all owners)
    ThreeQuartersMajority,
    /// Unanimous consent (100% of all owners)
    Unanimous,
}

impl SlovakDecisionType {
    /// Get the required quorum percentage for this decision type.
    pub fn required_quorum_percentage(&self) -> Decimal {
        match self {
            SlovakDecisionType::SimpleMajority => Decimal::new(5001, 2),
            SlovakDecisionType::TwoThirdsMajority => Decimal::new(6667, 2),
            SlovakDecisionType::ThreeQuartersMajority => Decimal::new(7500, 2),
            SlovakDecisionType::Unanimous => Decimal::new(10000, 2),
        }
    }

    /// Get the legal reference for this decision type.
    pub fn legal_reference(&self) -> &'static str {
        match self {
            SlovakDecisionType::SimpleMajority => "SS 14 ods. 1 zakona 182/1993 Z.z.",
            SlovakDecisionType::TwoThirdsMajority => "SS 14 ods. 2 zakona 182/1993 Z.z.",
            SlovakDecisionType::ThreeQuartersMajority => "SS 14 ods. 3 zakona 182/1993 Z.z.",
            SlovakDecisionType::Unanimous => "SS 14 ods. 4 zakona 182/1993 Z.z.",
        }
    }

    /// The snake_case key as stored in `jurisdiction_rules.decision_type`
    /// (migration 00197). This is the value to pass to
    /// `RegionalComplianceRepository::get_quorum_rule` — NOT `legal_reference()`,
    /// which the validate handlers previously used and which never matched the
    /// seeded keys, silently falling back to the in-code defaults (#1906
    /// finding-1). Matches the enum's `serde(rename_all = "snake_case")`.
    pub fn decision_type_key(&self) -> &'static str {
        match self {
            SlovakDecisionType::SimpleMajority => "simple_majority",
            SlovakDecisionType::TwoThirdsMajority => "two_thirds_majority",
            SlovakDecisionType::ThreeQuartersMajority => "three_quarters_majority",
            SlovakDecisionType::Unanimous => "unanimous",
        }
    }
}

// ============================================================================
// Czech Decision Types (zakon 89/2012 Sb.)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "cz_decision_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum CzechDecisionType {
    SimpleMajority,
    QualifiedMajority,
    ThreeQuartersMajority,
    AllOwners,
}

impl CzechDecisionType {
    pub fn required_quorum_percentage(&self) -> Decimal {
        match self {
            CzechDecisionType::SimpleMajority => Decimal::new(5001, 2),
            CzechDecisionType::QualifiedMajority => Decimal::new(6667, 2),
            CzechDecisionType::ThreeQuartersMajority => Decimal::new(7500, 2),
            CzechDecisionType::AllOwners => Decimal::new(10000, 2),
        }
    }

    pub fn legal_reference(&self) -> &'static str {
        match self {
            CzechDecisionType::SimpleMajority => "SS 1206 zakona 89/2012 Sb.",
            CzechDecisionType::QualifiedMajority => "SS 1208 zakona 89/2012 Sb.",
            CzechDecisionType::ThreeQuartersMajority => "SS 1209 zakona 89/2012 Sb.",
            CzechDecisionType::AllOwners => "SS 1214 zakona 89/2012 Sb.",
        }
    }

    /// The snake_case key as stored in `jurisdiction_rules.decision_type`
    /// (migration 00197) — pass this to `get_quorum_rule`, not `legal_reference()`
    /// (#1906 finding-1). Matches the enum's `serde(rename_all = "snake_case")`.
    pub fn decision_type_key(&self) -> &'static str {
        match self {
            CzechDecisionType::SimpleMajority => "simple_majority",
            CzechDecisionType::QualifiedMajority => "qualified_majority",
            CzechDecisionType::ThreeQuartersMajority => "three_quarters_majority",
            CzechDecisionType::AllOwners => "all_owners",
        }
    }
}

// ============================================================================
// Other Enums
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "sk_accounting_format", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum SlovakAccountingFormat {
    Pohoda,
    MoneyS3,
    GenericCsv,
}

impl SlovakAccountingFormat {
    /// Stable snake_case name. Use instead of `{:?}` in audit/log records.
    pub fn as_str(&self) -> &'static str {
        match self {
            SlovakAccountingFormat::Pohoda => "pohoda",
            SlovakAccountingFormat::MoneyS3 => "money_s3",
            SlovakAccountingFormat::GenericCsv => "generic_csv",
        }
    }
}

impl std::fmt::Display for SlovakAccountingFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "gdpr_consent_category", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum GdprConsentCategory {
    Essential,
    Communication,
    Marketing,
    Analytics,
    ThirdParty,
    Profiling,
}

impl GdprConsentCategory {
    /// Stable snake_case name. Use instead of `{:?}` in audit/log records.
    pub fn as_str(&self) -> &'static str {
        match self {
            GdprConsentCategory::Essential => "essential",
            GdprConsentCategory::Communication => "communication",
            GdprConsentCategory::Marketing => "marketing",
            GdprConsentCategory::Analytics => "analytics",
            GdprConsentCategory::ThirdParty => "third_party",
            GdprConsentCategory::Profiling => "profiling",
        }
    }
}

impl std::fmt::Display for GdprConsentCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "cz_org_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum CzechOrgType {
    Svj,
    Druzstvo,
    ObecniByt,
}

impl CzechOrgType {
    /// Stable snake_case name. Use instead of `{:?}` in audit/log records.
    pub fn as_str(&self) -> &'static str {
        match self {
            CzechOrgType::Svj => "svj",
            CzechOrgType::Druzstvo => "druzstvo",
            CzechOrgType::ObecniByt => "obecni_byt",
        }
    }
}

impl std::fmt::Display for CzechOrgType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Standard Slovak account codes for property management.
pub mod slovak_accounts {
    pub const CASH: &str = "211";
    pub const BANK_ACCOUNTS: &str = "221";
    pub const RECEIVABLES_OWNERS: &str = "311";
    pub const REPAIR_FUND: &str = "384";
}

fn default_true() -> bool {
    true
}

fn default_notice_days() -> i32 {
    14
}

// ============================================================================
// Slovak Voting Configuration
// ============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct SlovakVotingConfig {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub building_id: Uuid,
    pub enabled: bool,
    pub default_decision_type: String,
    pub use_ownership_weight: bool,
    pub min_notice_days: i32,
    pub allow_proxy_voting: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ConfigureSlovakVoting {
    pub building_id: Uuid,
    #[serde(default)]
    pub enabled: bool,
    pub default_decision_type: Option<SlovakDecisionType>,
    #[serde(default = "default_true")]
    pub use_ownership_weight: bool,
    #[serde(default = "default_notice_days")]
    pub min_notice_days: i32,
    #[serde(default = "default_true")]
    pub allow_proxy_voting: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SlovakVoteValidation {
    pub vote_id: Uuid,
    pub decision_type: SlovakDecisionType,
    pub required_quorum_percentage: Decimal,
    pub actual_participation_percentage: Decimal,
    pub quorum_met: bool,
    pub approval_percentage: Decimal,
    pub approval_required_percentage: Decimal,
    pub is_valid: bool,
    pub legal_reference: String,
    pub validation_notes: Vec<String>,
    pub validated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ValidateSlovakVote {
    pub vote_id: Uuid,
    pub decision_type: SlovakDecisionType,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SlovakVoteMinutes {
    pub vote_id: Uuid,
    pub building_id: Uuid,
    pub meeting_date: NaiveDate,
    pub meeting_location: String,
    pub title: String,
    pub decision_type: SlovakDecisionType,
    pub legal_reference: String,
    pub total_ownership_shares: Decimal,
    pub participating_shares: Decimal,
    pub participation_percentage: Decimal,
    pub quorum_required: Decimal,
    pub quorum_met: bool,
    pub votes_for: Decimal,
    pub votes_against: Decimal,
    pub abstentions: Decimal,
    pub result_approved: bool,
    pub participants: Vec<VoteParticipantMinutes>,
    pub questions: Vec<QuestionMinutes>,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VoteParticipantMinutes {
    pub unit_designation: String,
    pub owner_name: String,
    pub ownership_share: Decimal,
    pub is_proxy: bool,
    pub proxy_holder_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QuestionMinutes {
    pub question_text: String,
    pub votes_for: Decimal,
    pub votes_against: Decimal,
    pub abstentions: Decimal,
    pub result: String,
}

// ============================================================================
// Slovak Accounting
// ============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct SlovakAccountingConfig {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub export_format: String,
    pub ico: Option<String>,
    pub dic: Option<String>,
    pub ic_dph: Option<String>,
    pub default_iban: Option<String>,
    pub account_mapping: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ConfigureSlovakAccounting {
    pub export_format: SlovakAccountingFormat,
    pub ico: Option<String>,
    pub dic: Option<String>,
    pub ic_dph: Option<String>,
    pub default_iban: Option<String>,
    pub account_mapping: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ExportSlovakAccounting {
    pub organization_id: Uuid,
    pub building_id: Option<Uuid>,
    pub from_date: NaiveDate,
    pub to_date: NaiveDate,
    pub format: SlovakAccountingFormat,
    #[serde(default = "default_true")]
    pub include_invoices: bool,
    #[serde(default = "default_true")]
    pub include_payments: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SlovakAccountingExport {
    pub export_id: Uuid,
    pub organization_id: Uuid,
    pub from_date: NaiveDate,
    pub to_date: NaiveDate,
    pub format: SlovakAccountingFormat,
    pub invoice_count: i32,
    pub payment_count: i32,
    pub journal_entry_count: i32,
    pub total_revenue: Decimal,
    /// Total expenses for the period. `None` = **not available**, not zero.
    /// There is no expense source wired for this export yet (#1906 finding-3,
    /// tracked by #2030); a `null` here means "not computed" and a consumer
    /// MUST NOT treat it as genuine zero expenses. See `unsupported_fields`.
    pub total_expenses: Option<Decimal>,
    pub total_receivables: Decimal,
    /// Total accounts-payable for the period. `None` = **not available**, not
    /// zero. No vendor/accounts-payable source is wired yet (#1906 finding-3,
    /// tracked by #2030). See `unsupported_fields`.
    pub total_payables: Option<Decimal>,
    /// `true` when one or more monetary fields could not be computed and are
    /// returned as `null`. A partial export is NOT a complete accounting
    /// statement — see `unsupported_fields` for which figures are missing.
    pub partial: bool,
    /// Names of the fields that are not yet computed and are returned as
    /// `null` (e.g. `["total_expenses", "total_payables"]`). Empty when the
    /// export is complete.
    pub unsupported_fields: Vec<String>,
    pub download_url: Option<String>,
    pub export_data: Option<serde_json::Value>,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PohodaExport {
    pub header: PohodaHeader,
    pub invoices: Vec<PohodaInvoiceExport>,
    pub payments: Vec<PohodaPaymentExport>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PohodaHeader {
    pub ico: String,
    pub dic: Option<String>,
    pub company_name: String,
    pub export_date: NaiveDate,
    pub period_from: NaiveDate,
    pub period_to: NaiveDate,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PohodaInvoiceExport {
    pub invoice_number: String,
    pub issue_date: NaiveDate,
    pub due_date: NaiveDate,
    pub customer_name: String,
    pub customer_address: String,
    pub items: Vec<PohodaInvoiceItemExport>,
    pub total: Decimal,
    pub currency: String,
    pub account_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PohodaInvoiceItemExport {
    pub description: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub amount: Decimal,
    pub account_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PohodaPaymentExport {
    pub payment_date: NaiveDate,
    pub reference: Option<String>,
    pub payer_name: String,
    pub amount: Decimal,
    pub currency: String,
    pub account_code: String,
    pub linked_invoice: Option<String>,
}

// ============================================================================
// Slovak GDPR
// ============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct SlovakGdprConsent {
    pub id: Uuid,
    pub user_id: Uuid,
    pub organization_id: Option<Uuid>,
    pub category: String,
    pub granted: bool,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub consent_version: String,
    pub consented_at: DateTime<Utc>,
    pub withdrawn_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RecordGdprConsent {
    pub category: GdprConsentCategory,
    pub granted: bool,
    pub consent_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GdprConsentStatus {
    pub user_id: Uuid,
    pub consents: Vec<ConsentCategoryStatus>,
    pub dpo_contact: DpoContact,
    pub processing_purposes: Vec<ProcessingPurpose>,
    pub last_updated: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConsentCategoryStatus {
    pub category: GdprConsentCategory,
    pub name: String,
    pub description: String,
    pub granted: bool,
    pub required: bool,
    pub consented_at: Option<DateTime<Utc>>,
    pub consent_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DpoContact {
    pub name: String,
    pub email: String,
    pub phone: Option<String>,
    pub address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProcessingPurpose {
    pub category: GdprConsentCategory,
    pub purpose: String,
    pub legal_basis: String,
    pub retention_period: String,
    pub recipients: Vec<String>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct SlovakGdprConfig {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub dpo_name: String,
    pub dpo_email: String,
    pub dpo_phone: Option<String>,
    pub org_address: Option<String>,
    pub processing_purposes: serde_json::Value,
    pub consent_texts: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ConfigureSlovakGdpr {
    pub dpo_name: String,
    pub dpo_email: String,
    pub dpo_phone: Option<String>,
    pub org_address: Option<String>,
    pub processing_purposes: Option<serde_json::Value>,
    pub consent_texts: Option<serde_json::Value>,
}

// ============================================================================
// Czech SVJ
// ============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct CzechSvjConfig {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub building_id: Uuid,
    pub org_type: String,
    pub ico: String,
    pub has_stanovy: bool,
    pub stanovy_document_id: Option<Uuid>,
    pub stanovy_effective_date: Option<NaiveDate>,
    pub default_decision_type: String,
    pub use_ownership_weight: bool,
    pub notary_threshold_czk: Option<Decimal>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ConfigureCzechSvj {
    pub building_id: Uuid,
    pub org_type: CzechOrgType,
    pub ico: String,
    pub stanovy_document_id: Option<Uuid>,
    pub stanovy_effective_date: Option<NaiveDate>,
    pub default_decision_type: Option<CzechDecisionType>,
    #[serde(default = "default_true")]
    pub use_ownership_weight: bool,
    pub notary_threshold_czk: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CzechVoteValidation {
    pub vote_id: Uuid,
    pub decision_type: CzechDecisionType,
    pub required_quorum_percentage: Decimal,
    pub actual_participation_percentage: Decimal,
    pub quorum_met: bool,
    pub approval_percentage: Decimal,
    pub approval_required_percentage: Decimal,
    pub is_valid: bool,
    pub legal_reference: String,
    pub requires_notary: bool,
    pub validation_notes: Vec<String>,
    pub validated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ValidateCzechVote {
    pub vote_id: Uuid,
    pub decision_type: CzechDecisionType,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CzechSvjUsneseni {
    pub vote_id: Uuid,
    pub building_id: Uuid,
    pub svj_name: String,
    pub ico: String,
    pub meeting_date: NaiveDate,
    pub meeting_location: String,
    pub title: String,
    pub decision_type: CzechDecisionType,
    pub legal_reference: String,
    pub total_ownership_shares: Decimal,
    pub participating_shares: Decimal,
    pub participation_percentage: Decimal,
    pub quorum_required: Decimal,
    pub quorum_met: bool,
    pub votes_for: Decimal,
    pub votes_against: Decimal,
    pub abstentions: Decimal,
    pub result_approved: bool,
    pub requires_notary: bool,
    pub participants: Vec<VoteParticipantMinutes>,
    pub questions: Vec<QuestionMinutes>,
    pub generated_at: DateTime<Utc>,
}

// ============================================================================
// Compliance Status
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RegionalComplianceStatus {
    pub organization_id: Uuid,
    pub jurisdiction: Jurisdiction,
    pub slovak_voting_enabled: bool,
    pub slovak_accounting_configured: bool,
    pub slovak_gdpr_configured: bool,
    pub czech_svj_configured: bool,
    pub configured_buildings: Vec<Uuid>,
    pub last_checked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SetJurisdiction {
    pub jurisdiction: Jurisdiction,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression for #2030: an un-computed monetary field must serialize as
    /// JSON `null`, not a misleading `0`, so a consumer can distinguish
    /// "not available" from a genuine zero. A real zero (`Some(0)`) must still
    /// serialize as a number.
    #[test]
    fn uncomputed_totals_serialize_as_null_not_zero() {
        let export = SlovakAccountingExport {
            export_id: Uuid::nil(),
            organization_id: Uuid::nil(),
            from_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            to_date: NaiveDate::from_ymd_opt(2026, 3, 31).unwrap(),
            format: SlovakAccountingFormat::Pohoda,
            invoice_count: 3,
            payment_count: 2,
            journal_entry_count: 5,
            total_revenue: Decimal::new(12345, 2),
            total_expenses: None,
            total_receivables: Decimal::ZERO,
            total_payables: None,
            partial: true,
            unsupported_fields: vec!["total_expenses".into(), "total_payables".into()],
            download_url: None,
            export_data: None,
            generated_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        };

        let json = serde_json::to_value(&export).expect("serialize export");

        // Un-computed fields are explicit null — NOT the number 0.
        assert!(json["total_expenses"].is_null());
        assert!(json["total_payables"].is_null());
        // A genuinely-zero figure still renders as a number, proving the two
        // cases are distinguishable on the wire.
        assert!(!json["total_receivables"].is_null());
        // Partial signalling is honest and enumerates the missing fields.
        assert_eq!(json["partial"], serde_json::json!(true));
        assert_eq!(
            json["unsupported_fields"],
            serde_json::json!(["total_expenses", "total_payables"])
        );
    }
}
