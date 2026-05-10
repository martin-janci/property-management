//! Insurance management models for Epic 22.
//!
//! Includes insurance policies, claims, documents, and renewal reminders.

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Policy type constants.
pub mod policy_type {
    pub const PROPERTY: &str = "property";
    pub const LIABILITY: &str = "liability";
    pub const FLOOD: &str = "flood";
    pub const EARTHQUAKE: &str = "earthquake";
    pub const FIRE: &str = "fire";
    pub const UMBRELLA: &str = "umbrella";
    pub const EQUIPMENT: &str = "equipment";
    pub const DIRECTORS_OFFICERS: &str = "directors_officers";
    pub const CYBER: &str = "cyber";
    pub const WORKERS_COMP: &str = "workers_comp";
    pub const OTHER: &str = "other";

    pub const ALL: &[&str] = &[
        PROPERTY,
        LIABILITY,
        FLOOD,
        EARTHQUAKE,
        FIRE,
        UMBRELLA,
        EQUIPMENT,
        DIRECTORS_OFFICERS,
        CYBER,
        WORKERS_COMP,
        OTHER,
    ];
}

/// Policy status constants.
pub mod policy_status {
    pub const ACTIVE: &str = "active";
    pub const EXPIRED: &str = "expired";
    pub const CANCELLED: &str = "cancelled";
    pub const PENDING: &str = "pending";
    pub const SUSPENDED: &str = "suspended";

    pub const ALL: &[&str] = &[ACTIVE, EXPIRED, CANCELLED, PENDING, SUSPENDED];
}

/// Premium frequency constants.
pub mod premium_frequency {
    pub const MONTHLY: &str = "monthly";
    pub const QUARTERLY: &str = "quarterly";
    pub const SEMI_ANNUAL: &str = "semi_annual";
    pub const ANNUAL: &str = "annual";

    pub const ALL: &[&str] = &[MONTHLY, QUARTERLY, SEMI_ANNUAL, ANNUAL];
}

/// Claim status constants.
pub mod claim_status {
    pub const DRAFT: &str = "draft";
    pub const SUBMITTED: &str = "submitted";
    pub const UNDER_REVIEW: &str = "under_review";
    pub const INFORMATION_REQUESTED: &str = "information_requested";
    pub const APPROVED: &str = "approved";
    pub const PARTIALLY_APPROVED: &str = "partially_approved";
    pub const DENIED: &str = "denied";
    pub const PAID: &str = "paid";
    pub const CLOSED: &str = "closed";
    pub const WITHDRAWN: &str = "withdrawn";

    pub const ALL: &[&str] = &[
        DRAFT,
        SUBMITTED,
        UNDER_REVIEW,
        INFORMATION_REQUESTED,
        APPROVED,
        PARTIALLY_APPROVED,
        DENIED,
        PAID,
        CLOSED,
        WITHDRAWN,
    ];
}

/// Reminder type constants.
pub mod reminder_type {
    pub const EMAIL: &str = "email";
    pub const NOTIFICATION: &str = "notification";
    pub const BOTH: &str = "both";

    pub const ALL: &[&str] = &[EMAIL, NOTIFICATION, BOTH];
}

// ============================================
// Insurance Policy Models
// ============================================

/// Insurance policy entity.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct InsurancePolicy {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub policy_number: String,
    pub policy_name: String,
    pub provider_name: String,
    pub provider_contact: Option<String>,
    pub provider_phone: Option<String>,
    pub provider_email: Option<String>,
    pub policy_type: String,
    pub coverage_amount: Option<Decimal>,
    pub deductible: Option<Decimal>,
    pub premium_amount: Option<Decimal>,
    pub premium_frequency: Option<String>,
    pub building_id: Option<Uuid>,
    pub unit_id: Option<Uuid>,
    pub coverage_description: Option<String>,
    pub effective_date: NaiveDate,
    pub expiration_date: NaiveDate,
    pub renewal_date: Option<NaiveDate>,
    pub status: String,
    pub auto_renew: Option<bool>,
    pub terms: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// Create insurance policy request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateInsurancePolicy {
    pub policy_number: String,
    pub policy_name: String,
    pub provider_name: String,
    pub provider_contact: Option<String>,
    pub provider_phone: Option<String>,
    pub provider_email: Option<String>,
    pub policy_type: String,
    pub coverage_amount: Option<Decimal>,
    pub deductible: Option<Decimal>,
    pub premium_amount: Option<Decimal>,
    pub premium_frequency: Option<String>,
    pub building_id: Option<Uuid>,
    pub unit_id: Option<Uuid>,
    pub coverage_description: Option<String>,
    pub effective_date: NaiveDate,
    pub expiration_date: NaiveDate,
    pub renewal_date: Option<NaiveDate>,
    pub auto_renew: Option<bool>,
    pub terms: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
}

/// Update insurance policy request.
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct UpdateInsurancePolicy {
    pub policy_number: Option<String>,
    pub policy_name: Option<String>,
    pub provider_name: Option<String>,
    pub provider_contact: Option<String>,
    pub provider_phone: Option<String>,
    pub provider_email: Option<String>,
    pub policy_type: Option<String>,
    pub coverage_amount: Option<Decimal>,
    pub deductible: Option<Decimal>,
    pub premium_amount: Option<Decimal>,
    pub premium_frequency: Option<String>,
    pub building_id: Option<Uuid>,
    pub unit_id: Option<Uuid>,
    pub coverage_description: Option<String>,
    pub effective_date: Option<NaiveDate>,
    pub expiration_date: Option<NaiveDate>,
    pub renewal_date: Option<NaiveDate>,
    pub status: Option<String>,
    pub auto_renew: Option<bool>,
    pub terms: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
}

/// Query filter for insurance policies.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InsurancePolicyQuery {
    pub policy_type: Option<String>,
    pub status: Option<String>,
    pub building_id: Option<Uuid>,
    pub unit_id: Option<Uuid>,
    pub provider_name: Option<String>,
    pub expiring_within_days: Option<i32>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ============================================
// Insurance Claim Models
// ============================================

/// Insurance claim entity.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct InsuranceClaim {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub policy_id: Uuid,
    pub claim_number: Option<String>,
    pub provider_claim_number: Option<String>,
    pub incident_date: NaiveDate,
    pub incident_description: String,
    pub incident_location: Option<String>,
    pub building_id: Option<Uuid>,
    pub unit_id: Option<Uuid>,
    pub fault_id: Option<Uuid>,
    pub claimed_amount: Option<Decimal>,
    pub approved_amount: Option<Decimal>,
    pub paid_amount: Option<Decimal>,
    pub deductible_applied: Option<Decimal>,
    pub currency: Option<String>,
    pub status: String,
    pub submitted_by: Option<Uuid>,
    pub submitted_at: Option<DateTime<Utc>>,
    pub reviewed_by: Option<Uuid>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub adjuster_name: Option<String>,
    pub adjuster_phone: Option<String>,
    pub adjuster_email: Option<String>,
    pub resolution_notes: Option<String>,
    pub denial_reason: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// Create insurance claim request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateInsuranceClaim {
    pub policy_id: Uuid,
    pub claim_number: Option<String>,
    pub incident_date: NaiveDate,
    pub incident_description: String,
    pub incident_location: Option<String>,
    pub building_id: Option<Uuid>,
    pub unit_id: Option<Uuid>,
    pub fault_id: Option<Uuid>,
    pub claimed_amount: Option<Decimal>,
    pub currency: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// Update insurance claim request.
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct UpdateInsuranceClaim {
    pub claim_number: Option<String>,
    pub provider_claim_number: Option<String>,
    pub incident_date: Option<NaiveDate>,
    pub incident_description: Option<String>,
    pub incident_location: Option<String>,
    pub building_id: Option<Uuid>,
    pub unit_id: Option<Uuid>,
    pub fault_id: Option<Uuid>,
    pub claimed_amount: Option<Decimal>,
    pub approved_amount: Option<Decimal>,
    pub deductible_applied: Option<Decimal>,
    pub status: Option<String>,
    pub adjuster_name: Option<String>,
    pub adjuster_phone: Option<String>,
    pub adjuster_email: Option<String>,
    pub resolution_notes: Option<String>,
    pub denial_reason: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// Query filter for insurance claims.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InsuranceClaimQuery {
    pub policy_id: Option<Uuid>,
    pub status: Option<String>,
    pub building_id: Option<Uuid>,
    pub unit_id: Option<Uuid>,
    pub incident_date_from: Option<NaiveDate>,
    pub incident_date_to: Option<NaiveDate>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Insurance claim with policy details.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct InsuranceClaimWithPolicy {
    // Claim fields
    pub id: Uuid,
    pub organization_id: Uuid,
    pub policy_id: Uuid,
    pub claim_number: Option<String>,
    pub provider_claim_number: Option<String>,
    pub incident_date: NaiveDate,
    pub incident_description: String,
    pub incident_location: Option<String>,
    pub building_id: Option<Uuid>,
    pub unit_id: Option<Uuid>,
    pub fault_id: Option<Uuid>,
    pub claimed_amount: Option<Decimal>,
    pub approved_amount: Option<Decimal>,
    pub paid_amount: Option<Decimal>,
    pub deductible_applied: Option<Decimal>,
    pub currency: Option<String>,
    pub status: String,
    pub submitted_by: Option<Uuid>,
    pub submitted_at: Option<DateTime<Utc>>,
    pub reviewed_by: Option<Uuid>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub adjuster_name: Option<String>,
    pub adjuster_phone: Option<String>,
    pub adjuster_email: Option<String>,
    pub resolution_notes: Option<String>,
    pub denial_reason: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    // Policy fields
    pub policy_number: Option<String>,
    pub policy_name: Option<String>,
    pub policy_type: Option<String>,
    pub provider_name: Option<String>,
}

// ============================================
// Claim History Models
// ============================================

/// Insurance claim status history entry.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct InsuranceClaimHistory {
    pub id: Uuid,
    pub claim_id: Uuid,
    pub old_status: Option<String>,
    pub new_status: String,
    pub changed_by: Option<Uuid>,
    pub notes: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

// ============================================
// Policy Document Models
// ============================================

/// Insurance policy document junction.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct InsurancePolicyDocument {
    pub id: Uuid,
    pub policy_id: Uuid,
    pub document_id: Uuid,
    pub document_type: String,
    pub created_at: Option<DateTime<Utc>>,
}

/// Add document to policy request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddPolicyDocument {
    pub document_id: Uuid,
    pub document_type: Option<String>,
}

// ============================================
// Claim Document Models
// ============================================

/// Insurance claim document junction.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct InsuranceClaimDocument {
    pub id: Uuid,
    pub claim_id: Uuid,
    pub document_id: Uuid,
    pub document_type: String,
    pub created_at: Option<DateTime<Utc>>,
}

/// Add document to claim request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddClaimDocument {
    pub document_id: Uuid,
    pub document_type: Option<String>,
}

// ============================================
// Renewal Reminder Models
// ============================================

/// Insurance renewal reminder entity.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct InsuranceRenewalReminder {
    pub id: Uuid,
    pub policy_id: Uuid,
    pub days_before_expiry: i32,
    pub reminder_type: String,
    pub sent_at: Option<DateTime<Utc>>,
    pub is_active: Option<bool>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// Create renewal reminder request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRenewalReminder {
    pub days_before_expiry: i32,
    pub reminder_type: Option<String>,
}

/// Update renewal reminder request.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateRenewalReminder {
    pub days_before_expiry: Option<i32>,
    pub reminder_type: Option<String>,
    pub is_active: Option<bool>,
}

// ============================================
// Statistics and Summary Models
// ============================================

/// Expiring policy summary.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct ExpiringPolicy {
    pub policy_id: Uuid,
    pub policy_number: String,
    pub policy_name: String,
    pub policy_type: String,
    pub provider_name: String,
    pub expiration_date: NaiveDate,
    pub days_until_expiry: i32,
    pub coverage_amount: Option<Decimal>,
    pub auto_renew: Option<bool>,
}

/// Insurance coverage statistics.
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct InsuranceStatistics {
    pub total_policies: i64,
    pub active_policies: i64,
    pub expiring_soon: i64,
    pub total_coverage: Option<Decimal>,
    pub total_premiums: Option<Decimal>,
    pub total_claims: i64,
    pub open_claims: i64,
    pub total_claimed: Option<Decimal>,
    pub total_paid: Option<Decimal>,
}

/// Claim summary by status.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct ClaimStatusSummary {
    pub status: String,
    pub count: i64,
    pub total_claimed: Option<Decimal>,
    pub total_approved: Option<Decimal>,
    pub total_paid: Option<Decimal>,
}

/// Policy type coverage summary.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct PolicyTypeSummary {
    pub policy_type: String,
    pub policy_count: i64,
    pub total_coverage: Option<Decimal>,
    pub total_premiums: Option<Decimal>,
}
