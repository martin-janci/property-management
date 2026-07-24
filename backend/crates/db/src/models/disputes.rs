//! Dispute resolution models (Epic 77).
//!
//! Provides structured process for resolving disputes between parties
//! (tenant-landlord, neighbor-neighbor).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// =============================================================================
// Story 77.1: Dispute Filing
// =============================================================================

/// Dispute category constants.
pub mod dispute_category {
    pub const NOISE: &str = "noise";
    pub const DAMAGE: &str = "damage";
    pub const PAYMENT: &str = "payment";
    pub const LEASE_TERMS: &str = "lease_terms";
    pub const COMMON_AREA: &str = "common_area";
    pub const PARKING: &str = "parking";
    pub const PETS: &str = "pets";
    pub const MAINTENANCE: &str = "maintenance";
    pub const PRIVACY: &str = "privacy";
    pub const HARASSMENT: &str = "harassment";
    pub const OTHER: &str = "other";

    pub const ALL: &[&str] = &[
        NOISE,
        DAMAGE,
        PAYMENT,
        LEASE_TERMS,
        COMMON_AREA,
        PARKING,
        PETS,
        MAINTENANCE,
        PRIVACY,
        HARASSMENT,
        OTHER,
    ];
}

/// Dispute status constants.
pub mod dispute_status {
    pub const FILED: &str = "filed";
    pub const UNDER_REVIEW: &str = "under_review";
    pub const MEDIATION: &str = "mediation";
    pub const AWAITING_RESPONSE: &str = "awaiting_response";
    pub const RESOLVED: &str = "resolved";
    pub const ESCALATED: &str = "escalated";
    pub const WITHDRAWN: &str = "withdrawn";
    pub const CLOSED: &str = "closed";

    pub const ALL: &[&str] = &[
        FILED,
        UNDER_REVIEW,
        MEDIATION,
        AWAITING_RESPONSE,
        RESOLVED,
        ESCALATED,
        WITHDRAWN,
        CLOSED,
    ];
}

/// Dispute state machine — valid status transitions.
///
/// Lifecycle: `filed` (open) → `under_review` (in-review) → `resolved`
pub mod dispute_state_machine {
    /// Returns `true` when the `from` → `to` status transition is permitted.
    pub fn is_valid_transition(from: &str, to: &str) -> bool {
        match from {
            "filed" => matches!(to, "under_review" | "awaiting_response" | "withdrawn"),
            "under_review" => {
                matches!(
                    to,
                    "mediation" | "awaiting_response" | "resolved" | "escalated"
                )
            }
            "awaiting_response" => {
                matches!(to, "under_review" | "mediation" | "resolved" | "withdrawn")
            }
            "mediation" => matches!(to, "under_review" | "resolved" | "escalated"),
            "escalated" => matches!(to, "under_review" | "resolved" | "closed"),
            "resolved" => to == "closed",
            _ => false,
        }
    }

    /// Returns the allowed next statuses from `from`, or an empty slice for terminals.
    pub fn allowed_transitions(from: &str) -> &'static [&'static str] {
        match from {
            "filed" => &["under_review", "awaiting_response", "withdrawn"],
            "under_review" => &["mediation", "awaiting_response", "resolved", "escalated"],
            "awaiting_response" => &["under_review", "mediation", "resolved", "withdrawn"],
            "mediation" => &["under_review", "resolved", "escalated"],
            "escalated" => &["under_review", "resolved", "closed"],
            "resolved" => &["closed"],
            _ => &[],
        }
    }
}

/// Dispute priority constants.
pub mod dispute_priority {
    pub const LOW: &str = "low";
    pub const MEDIUM: &str = "medium";
    pub const HIGH: &str = "high";
    pub const URGENT: &str = "urgent";

    pub const ALL: &[&str] = &[LOW, MEDIUM, HIGH, URGENT];
}

/// Dispute party role constants.
pub mod party_role {
    pub const COMPLAINANT: &str = "complainant";
    pub const RESPONDENT: &str = "respondent";
    pub const WITNESS: &str = "witness";
    pub const MEDIATOR: &str = "mediator";

    pub const ALL: &[&str] = &[COMPLAINANT, RESPONDENT, WITNESS, MEDIATOR];
}

/// A dispute case filed between parties.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Dispute {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub building_id: Option<Uuid>,
    pub unit_id: Option<Uuid>,
    pub reference_number: String,
    pub category: String,
    pub title: String,
    pub description: String,
    pub desired_resolution: Option<String>,
    pub status: String,
    pub priority: String,
    pub filed_by: Uuid,
    pub assigned_to: Option<Uuid>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolution_notes: Option<String>,
    pub mediation_notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A party involved in a dispute.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DisputeParty {
    pub id: Uuid,
    pub dispute_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub notified_at: Option<DateTime<Utc>>,
    pub responded_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Evidence or attachment for a dispute.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DisputeEvidence {
    pub id: Uuid,
    pub dispute_id: Uuid,
    pub uploaded_by: Uuid,
    pub filename: String,
    pub original_filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub storage_url: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Request to file a new dispute.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDispute {
    pub organization_id: Uuid,
    pub building_id: Option<Uuid>,
    pub unit_id: Option<Uuid>,
    pub category: String,
    pub title: String,
    pub description: String,
    pub desired_resolution: Option<String>,
    pub respondent_ids: Vec<Uuid>,
    pub filed_by: Uuid,
}

/// Request to add evidence to a dispute.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddEvidence {
    pub dispute_id: Uuid,
    pub uploaded_by: Uuid,
    pub filename: String,
    pub original_filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub storage_url: String,
    pub description: Option<String>,
}

// =============================================================================
// Story 77.2: Mediation Process
// =============================================================================

/// Mediation session type constants.
pub mod session_type {
    pub const IN_PERSON: &str = "in_person";
    pub const VIDEO_CALL: &str = "video_call";
    pub const PHONE: &str = "phone";
    pub const WRITTEN: &str = "written";

    pub const ALL: &[&str] = &[IN_PERSON, VIDEO_CALL, PHONE, WRITTEN];
}

/// Mediation session status constants.
pub mod session_status {
    pub const SCHEDULED: &str = "scheduled";
    pub const IN_PROGRESS: &str = "in_progress";
    pub const COMPLETED: &str = "completed";
    pub const CANCELLED: &str = "cancelled";
    pub const RESCHEDULED: &str = "rescheduled";

    pub const ALL: &[&str] = &[SCHEDULED, IN_PROGRESS, COMPLETED, CANCELLED, RESCHEDULED];
}

/// A mediation session for a dispute.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MediationSession {
    pub id: Uuid,
    pub dispute_id: Uuid,
    pub mediator_id: Uuid,
    pub session_type: String,
    pub scheduled_at: DateTime<Utc>,
    pub duration_minutes: Option<i32>,
    pub location: Option<String>,
    pub meeting_url: Option<String>,
    pub status: String,
    pub notes: Option<String>,
    pub outcome: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Attendance record for a mediation session.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SessionAttendance {
    pub id: Uuid,
    pub session_id: Uuid,
    pub party_id: Uuid,
    pub confirmed: bool,
    pub attended: Option<bool>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A submission from a party during mediation.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PartySubmission {
    pub id: Uuid,
    pub dispute_id: Uuid,
    pub party_id: Uuid,
    pub submission_type: String,
    pub content: String,
    pub is_visible_to_all: bool,
    pub created_at: DateTime<Utc>,
}

/// Request to schedule a mediation session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleSession {
    pub dispute_id: Uuid,
    pub mediator_id: Uuid,
    pub session_type: String,
    pub scheduled_at: DateTime<Utc>,
    pub duration_minutes: Option<i32>,
    pub location: Option<String>,
    pub meeting_url: Option<String>,
    pub attendee_party_ids: Vec<Uuid>,
}

/// Request to record session notes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordSessionNotes {
    pub session_id: Uuid,
    pub notes: String,
    pub outcome: Option<String>,
}

/// Request to submit a party response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitResponse {
    pub dispute_id: Uuid,
    pub party_id: Uuid,
    pub submission_type: String,
    pub content: String,
    pub is_visible_to_all: bool,
}

// =============================================================================
// Story 77.3: Resolution Tracking
// =============================================================================

/// Resolution status constants.
pub mod resolution_status {
    pub const PROPOSED: &str = "proposed";
    pub const ACCEPTED: &str = "accepted";
    pub const REJECTED: &str = "rejected";
    pub const PARTIALLY_ACCEPTED: &str = "partially_accepted";
    pub const IMPLEMENTED: &str = "implemented";

    pub const ALL: &[&str] = &[
        PROPOSED,
        ACCEPTED,
        REJECTED,
        PARTIALLY_ACCEPTED,
        IMPLEMENTED,
    ];
}

/// A proposed resolution for a dispute.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DisputeResolution {
    pub id: Uuid,
    pub dispute_id: Uuid,
    pub proposed_by: Uuid,
    pub resolution_text: String,
    pub terms: sqlx::types::Json<Vec<ResolutionTerm>>,
    pub status: String,
    pub proposed_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub implemented_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A specific term within a resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionTerm {
    pub id: String,
    pub description: String,
    pub responsible_party_id: Option<Uuid>,
    pub deadline: Option<DateTime<Utc>>,
    pub completed: bool,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Acceptance or rejection of a resolution by a party.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ResolutionVote {
    pub id: Uuid,
    pub resolution_id: Uuid,
    pub party_id: Uuid,
    pub accepted: bool,
    pub comments: Option<String>,
    pub voted_at: DateTime<Utc>,
}

/// Timeline entry for dispute activity.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DisputeActivity {
    pub id: Uuid,
    pub dispute_id: Uuid,
    pub actor_id: Uuid,
    pub activity_type: String,
    pub description: String,
    pub metadata: Option<sqlx::types::Json<serde_json::Value>>,
    pub created_at: DateTime<Utc>,
}

/// Activity type constants.
pub mod activity_type {
    pub const DISPUTE_FILED: &str = "dispute_filed";
    pub const STATUS_CHANGED: &str = "status_changed";
    pub const PARTY_ADDED: &str = "party_added";
    pub const EVIDENCE_ADDED: &str = "evidence_added";
    pub const SESSION_SCHEDULED: &str = "session_scheduled";
    pub const SESSION_COMPLETED: &str = "session_completed";
    pub const RESOLUTION_PROPOSED: &str = "resolution_proposed";
    pub const RESOLUTION_VOTED: &str = "resolution_voted";
    pub const RESOLUTION_ACCEPTED: &str = "resolution_accepted";
    pub const ACTION_CREATED: &str = "action_created";
    pub const ACTION_COMPLETED: &str = "action_completed";
    pub const COMMENT_ADDED: &str = "comment_added";
    pub const ESCALATED: &str = "escalated";
    pub const CLOSED: &str = "closed";

    pub const ALL: &[&str] = &[
        DISPUTE_FILED,
        STATUS_CHANGED,
        PARTY_ADDED,
        EVIDENCE_ADDED,
        SESSION_SCHEDULED,
        SESSION_COMPLETED,
        RESOLUTION_PROPOSED,
        RESOLUTION_VOTED,
        RESOLUTION_ACCEPTED,
        ACTION_CREATED,
        ACTION_COMPLETED,
        COMMENT_ADDED,
        ESCALATED,
        CLOSED,
    ];
}

/// Request to propose a resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposeResolution {
    pub dispute_id: Uuid,
    pub proposed_by: Uuid,
    pub resolution_text: String,
    pub terms: Vec<ResolutionTerm>,
}

/// Request to vote on a resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteOnResolution {
    pub resolution_id: Uuid,
    pub party_id: Uuid,
    pub accepted: bool,
    pub comments: Option<String>,
}

/// Request to update dispute status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDisputeStatus {
    pub dispute_id: Uuid,
    /// Tenancy scope — `update_status` MUST filter by this so a manager in
    /// org A cannot drive a dispute in org B (issue #520).
    pub organization_id: Uuid,
    pub status: String,
    pub reason: Option<String>,
    pub updated_by: Uuid,
}

/// Request to resolve a dispute with resolution notes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveDispute {
    pub dispute_id: Uuid,
    pub resolution_notes: String,
    pub resolved_by: Uuid,
    /// Organization the caller belongs to — prevents cross-org IDOR.
    pub organization_id: Uuid,
}

/// Request to update mediation notes on a dispute.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMediationNotes {
    pub dispute_id: Uuid,
    pub notes: String,
    /// User performing the update — recorded in the audit trail.
    pub updated_by: Uuid,
    /// Organization the caller belongs to — prevents cross-org IDOR.
    pub organization_id: Uuid,
}

// =============================================================================
// Story 77.4: Resolution Enforcement
// =============================================================================

/// Action item status constants.
pub mod action_status {
    pub const PENDING: &str = "pending";
    pub const IN_PROGRESS: &str = "in_progress";
    pub const COMPLETED: &str = "completed";
    pub const OVERDUE: &str = "overdue";
    pub const ESCALATED: &str = "escalated";

    pub const ALL: &[&str] = &[PENDING, IN_PROGRESS, COMPLETED, OVERDUE, ESCALATED];
}

/// An action item assigned to a party as part of resolution enforcement.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ActionItem {
    pub id: Uuid,
    pub dispute_id: Uuid,
    pub resolution_id: Option<Uuid>,
    pub resolution_term_id: Option<String>,
    pub assigned_to: Uuid,
    pub title: String,
    pub description: String,
    pub due_date: DateTime<Utc>,
    pub status: String,
    pub completed_at: Option<DateTime<Utc>>,
    pub completion_notes: Option<String>,
    pub reminder_sent_at: Option<DateTime<Utc>>,
    pub escalated_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Escalation record for non-compliance.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Escalation {
    pub id: Uuid,
    pub dispute_id: Uuid,
    pub action_item_id: Option<Uuid>,
    pub escalated_by: Uuid,
    pub escalated_to: Option<Uuid>,
    pub reason: String,
    pub severity: String,
    pub resolved: bool,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolution_notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Escalation severity constants.
pub mod escalation_severity {
    pub const WARNING: &str = "warning";
    pub const FORMAL_NOTICE: &str = "formal_notice";
    pub const LEGAL_REFERRAL: &str = "legal_referral";

    pub const ALL: &[&str] = &[WARNING, FORMAL_NOTICE, LEGAL_REFERRAL];
}

/// Request to create an action item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateActionItem {
    pub dispute_id: Uuid,
    pub resolution_id: Option<Uuid>,
    pub resolution_term_id: Option<String>,
    pub assigned_to: Uuid,
    pub title: String,
    pub description: String,
    pub due_date: DateTime<Utc>,
}

/// Request to complete an action item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteActionItem {
    pub action_item_id: Uuid,
    pub completion_notes: Option<String>,
}

/// Request to escalate non-compliance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEscalation {
    pub dispute_id: Uuid,
    pub action_item_id: Option<Uuid>,
    pub escalated_by: Uuid,
    pub escalated_to: Option<Uuid>,
    pub reason: String,
    pub severity: String,
}

/// Request to resolve an escalation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveEscalation {
    pub escalation_id: Uuid,
    pub resolution_notes: String,
}

// =============================================================================
// Query & View Types
// =============================================================================

/// Query parameters for listing disputes.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DisputeQuery {
    pub building_id: Option<Uuid>,
    pub category: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub filed_by: Option<Uuid>,
    pub assigned_to: Option<Uuid>,
    pub search: Option<String>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

/// Dispute with extended details for display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisputeWithDetails {
    pub dispute: Dispute,
    pub parties: Vec<DisputePartyWithUser>,
    pub evidence_count: i64,
    pub activity_count: i64,
    pub session_count: i64,
    pub active_resolution: Option<DisputeResolution>,
    pub pending_actions: Vec<ActionItem>,
}

/// Dispute party with user details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisputePartyWithUser {
    pub party: DisputeParty,
    pub user_name: String,
    pub user_email: String,
}

/// Summary view for dispute listing.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DisputeSummary {
    pub id: Uuid,
    pub reference_number: String,
    pub category: String,
    pub title: String,
    pub status: String,
    pub priority: String,
    pub filed_by_name: String,
    pub assigned_to_name: Option<String>,
    pub party_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Statistics for disputes dashboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisputeStatistics {
    pub total_disputes: i64,
    pub by_status: Vec<StatusCount>,
    pub by_category: Vec<CategoryCount>,
    pub by_priority: Vec<PriorityCount>,
    pub avg_resolution_days: Option<f64>,
    pub pending_actions: i64,
    pub overdue_actions: i64,
}

/// Count by status.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct StatusCount {
    pub status: String,
    pub count: i64,
}

/// Count by category.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CategoryCount {
    pub category: String,
    pub count: i64,
}

/// Count by priority.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PriorityCount {
    pub priority: String,
    pub count: i64,
}

/// Lifecycle funnel KPIs for a dispute cohort (issue #2533).
///
/// Backs the `dispute.funnel.*` metrics defined in
/// `docs/data/dispute-lifecycle-kpis.md`. `reached_mediation` is computed from
/// the structured `dispute_activities.metadata->>'to'` written on every
/// `status_changed` (with a legacy free-text fallback for pre-#2533 rows).
/// Rates are `None` (not `0.0`) when the denominator is `0` — an empty cohort
/// reads as "no data", never a misleading `0%`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisputeFunnelKpis {
    /// Disputes filed in the window — the funnel denominator.
    pub filed: i64,
    /// Cohort disputes that entered `mediation` at any point.
    pub reached_mediation: i64,
    /// Cohort disputes that reached `resolved` (`resolved_at IS NOT NULL`).
    pub reached_resolved: i64,
    /// Cohort disputes currently sitting in `mediation`.
    pub currently_in_mediation: i64,
    /// `reached_mediation / filed`, or `None` when `filed = 0`.
    pub mediation_rate: Option<f64>,
    /// `reached_resolved / filed`, or `None` when `filed = 0`.
    pub resolution_rate: Option<f64>,
}

/// Time-to-resolution percentile KPIs for a dispute cohort (issue #2533).
///
/// Backs `dispute.ttr.*`. TTR = `resolved_at - created_at`; only disputes with
/// `resolved_at IS NOT NULL` enter the population. Durations are reported in
/// **hours** (the canonical unit from the KPI spec). Percentiles use
/// `percentile_cont` for stable interpolation on small samples, and are `None`
/// when the sample is empty.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisputeTtrKpis {
    /// Number of resolved disputes in the cohort (TTR sample size).
    pub count: i64,
    /// 50th percentile (median) TTR in hours.
    pub p50_hours: Option<f64>,
    /// 90th percentile TTR in hours.
    pub p90_hours: Option<f64>,
    /// 95th percentile TTR in hours.
    pub p95_hours: Option<f64>,
    /// Arithmetic mean TTR in hours.
    pub mean_hours: Option<f64>,
    /// Slowest resolution in the cohort, in hours.
    pub max_hours: Option<f64>,
}

/// Dispute lifecycle KPIs for a `[window_start, window_end)` cohort (issue #2533).
///
/// SQL-backed counterpart to the definitions in
/// `docs/data/dispute-lifecycle-kpis.md`. Cohort membership is keyed on
/// `disputes.created_at` (filing time) so a dispute stays in exactly one window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisputeKpis {
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
    pub funnel: DisputeFunnelKpis,
    pub ttr: DisputeTtrKpis,
}

/// Mediation case view with sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediationCase {
    pub dispute: Dispute,
    pub sessions: Vec<MediationSessionWithAttendance>,
    pub submissions: Vec<PartySubmission>,
    pub resolutions: Vec<DisputeResolution>,
}

/// Mediation session with attendance records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediationSessionWithAttendance {
    pub session: MediationSession,
    pub attendance: Vec<SessionAttendance>,
}

/// Resolution with votes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionWithVotes {
    pub resolution: DisputeResolution,
    pub votes: Vec<ResolutionVote>,
    pub acceptance_rate: f64,
}

/// Action items dashboard for a party.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartyActionsDashboard {
    pub user_id: Uuid,
    pub pending: Vec<ActionItem>,
    pub overdue: Vec<ActionItem>,
    pub completed_recently: Vec<ActionItem>,
    pub total_pending: i64,
    pub total_overdue: i64,
}
