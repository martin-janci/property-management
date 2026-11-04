//! Vote model (Epic 5: Building Voting & Decisions).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Vote status enum values.
pub mod vote_status {
    pub const DRAFT: &str = "draft";
    pub const SCHEDULED: &str = "scheduled";
    pub const ACTIVE: &str = "active";
    pub const CLOSED: &str = "closed";
    pub const CANCELLED: &str = "cancelled";

    pub const ALL: &[&str] = &[DRAFT, SCHEDULED, ACTIVE, CLOSED, CANCELLED];
}

/// Question type enum values.
pub mod question_type {
    pub const YES_NO: &str = "yes_no";
    pub const SINGLE_CHOICE: &str = "single_choice";
    pub const MULTIPLE_CHOICE: &str = "multiple_choice";
    pub const RANKED: &str = "ranked";

    pub const ALL: &[&str] = &[YES_NO, SINGLE_CHOICE, MULTIPLE_CHOICE, RANKED];
}

/// Quorum type enum values.
pub mod quorum_type {
    pub const SIMPLE_MAJORITY: &str = "simple_majority";
    pub const TWO_THIRDS: &str = "two_thirds";
    pub const WEIGHTED: &str = "weighted";

    pub const ALL: &[&str] = &[SIMPLE_MAJORITY, TWO_THIRDS, WEIGHTED];
}

/// Audit action enum values.
pub mod audit_action {
    pub const VOTE_CREATED: &str = "vote_created";
    pub const VOTE_PUBLISHED: &str = "vote_published";
    pub const VOTE_CANCELLED: &str = "vote_cancelled";
    pub const QUESTION_ADDED: &str = "question_added";
    pub const QUESTION_REMOVED: &str = "question_removed";
    pub const BALLOT_CAST: &str = "ballot_cast";
    pub const BALLOT_UPDATED: &str = "ballot_updated";
    pub const COMMENT_ADDED: &str = "comment_added";
    pub const COMMENT_HIDDEN: &str = "comment_hidden";
    pub const VOTE_CLOSED: &str = "vote_closed";
    pub const RESULTS_CALCULATED: &str = "results_calculated";
}

// ============================================================================
// Vote
// ============================================================================

/// Vote entity from database.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct Vote {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub building_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: DateTime<Utc>,
    pub status: String,
    pub quorum_type: String,
    pub quorum_percentage: Option<i32>,
    pub allow_delegation: bool,
    pub anonymous_voting: bool,
    pub participation_count: Option<i32>,
    pub eligible_count: Option<i32>,
    pub quorum_met: Option<bool>,
    pub results: serde_json::Value,
    pub results_calculated_at: Option<DateTime<Utc>>,
    pub created_by: Uuid,
    pub published_by: Option<Uuid>,
    pub published_at: Option<DateTime<Utc>>,
    pub cancelled_by: Option<Uuid>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub cancellation_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Vote {
    /// Check if vote is in draft status.
    pub fn is_draft(&self) -> bool {
        self.status == vote_status::DRAFT
    }

    /// Check if vote is active.
    pub fn is_active(&self) -> bool {
        self.status == vote_status::ACTIVE
    }

    /// Check if vote is closed.
    pub fn is_closed(&self) -> bool {
        self.status == vote_status::CLOSED
    }

    /// Check if vote can be edited.
    pub fn can_edit(&self) -> bool {
        self.status == vote_status::DRAFT
    }

    /// Check if voting is currently allowed.
    pub fn can_vote(&self) -> bool {
        if self.status != vote_status::ACTIVE {
            return false;
        }
        let now = Utc::now();
        if let Some(start) = self.start_at {
            if now < start {
                return false;
            }
        }
        now < self.end_at
    }
}

/// Summary view of a vote.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct VoteSummary {
    pub id: Uuid,
    pub building_id: Uuid,
    pub title: String,
    pub status: String,
    pub end_at: DateTime<Utc>,
    pub quorum_type: String,
    pub participation_count: Option<i32>,
    pub eligible_count: Option<i32>,
    pub quorum_met: Option<bool>,
    /// Number of ballot questions (counted in SQL; VoteCard shows it).
    pub question_count: i64,
}

/// Vote with additional details.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VoteWithDetails {
    #[serde(flatten)]
    pub vote: Vote,
    pub building_name: String,
    pub created_by_name: String,
    pub question_count: i64,
    pub response_count: i64,
    pub comment_count: i64,
    /// The vote's questions (with their options), ordered by `display_order`.
    /// The ballot UI renders straight from the detail response, so it must
    /// not need a second round-trip to `GET /voting/{id}/questions`.
    #[serde(default)]
    pub questions: Vec<VoteQuestion>,
}

/// Data for creating a vote.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateVote {
    pub organization_id: Uuid,
    pub building_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: DateTime<Utc>,
    pub quorum_type: String,
    pub quorum_percentage: Option<i32>,
    pub allow_delegation: Option<bool>,
    pub anonymous_voting: Option<bool>,
    pub created_by: Uuid,
}

/// Data for updating a vote.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateVote {
    pub title: Option<String>,
    pub description: Option<String>,
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
    pub quorum_type: Option<String>,
    pub quorum_percentage: Option<i32>,
    pub allow_delegation: Option<bool>,
    pub anonymous_voting: Option<bool>,
}

/// Data for publishing a vote.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PublishVote {
    pub start_at: Option<DateTime<Utc>>,
}

/// Data for cancelling a vote.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CancelVote {
    pub reason: String,
}

// ============================================================================
// Vote Question
// ============================================================================

/// Question option structure.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QuestionOption {
    pub id: Uuid,
    pub text: String,
    pub order: i32,
}

/// Vote question entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct VoteQuestion {
    pub id: Uuid,
    pub vote_id: Uuid,
    pub question_text: String,
    pub description: Option<String>,
    pub question_type: String,
    pub options: serde_json::Value,
    pub display_order: i32,
    pub is_required: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Data for creating a question.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateVoteQuestion {
    pub vote_id: Uuid,
    pub question_text: String,
    pub description: Option<String>,
    pub question_type: String,
    pub options: Vec<QuestionOption>,
    pub display_order: Option<i32>,
    pub is_required: Option<bool>,
}

/// Data for updating a question.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateVoteQuestion {
    pub question_text: Option<String>,
    pub description: Option<String>,
    pub options: Option<Vec<QuestionOption>>,
    pub display_order: Option<i32>,
    pub is_required: Option<bool>,
}

// ============================================================================
// Vote Response (Ballot)
// ============================================================================

/// Vote response entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct VoteResponse {
    pub id: Uuid,
    pub vote_id: Uuid,
    pub user_id: Uuid,
    pub unit_id: Uuid,
    pub delegation_id: Option<Uuid>,
    pub is_delegated: bool,
    pub answers: serde_json::Value,
    pub vote_weight: rust_decimal::Decimal,
    pub response_hash: String,
    pub submitted_at: DateTime<Utc>,
}

/// Data for casting a vote.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CastVote {
    pub vote_id: Uuid,
    pub user_id: Uuid,
    pub unit_id: Uuid,
    pub delegation_id: Option<Uuid>,
    pub answers: serde_json::Value,
}

/// Vote receipt for confirmation.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VoteReceipt {
    pub response_id: Uuid,
    pub vote_id: Uuid,
    pub unit_id: Uuid,
    pub submitted_at: DateTime<Utc>,
    pub response_hash: String,
    pub confirmation_number: String,
}

// ============================================================================
// Vote Comment
// ============================================================================

/// Vote comment entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct VoteComment {
    pub id: Uuid,
    pub vote_id: Uuid,
    pub user_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub content: String,
    pub hidden: bool,
    pub hidden_by: Option<Uuid>,
    pub hidden_at: Option<DateTime<Utc>>,
    pub hidden_reason: Option<String>,
    pub ai_consent: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Comment with user info.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VoteCommentWithUser {
    #[serde(flatten)]
    pub comment: VoteComment,
    pub user_name: String,
    pub reply_count: i64,
}

/// Data for creating a comment.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateVoteComment {
    pub vote_id: Uuid,
    pub user_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub content: String,
    pub ai_consent: bool,
}

/// Data for hiding a comment.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HideVoteComment {
    pub reason: String,
}

// ============================================================================
// Vote Audit Log
// ============================================================================

/// Audit log entry.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct VoteAuditLog {
    pub id: Uuid,
    pub vote_id: Uuid,
    pub user_id: Option<Uuid>,
    pub action: String,
    pub data_hash: String,
    pub data_snapshot: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Data for creating an audit entry.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateVoteAuditLog {
    pub vote_id: Uuid,
    pub user_id: Option<Uuid>,
    pub action: String,
    pub data: serde_json::Value,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

// ============================================================================
// Results
// ============================================================================

/// Vote results structure.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VoteResults {
    pub vote_id: Uuid,
    pub participation_count: i32,
    pub eligible_count: i32,
    pub participation_rate: f64,
    pub quorum_met: bool,
    pub questions: Vec<QuestionResult>,
    pub calculated_at: DateTime<Utc>,
}

/// Result for a single question.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QuestionResult {
    pub question_id: Uuid,
    pub question_text: String,
    pub question_type: String,
    pub total_votes: i32,
    pub weighted_total: f64,
    pub results: Vec<OptionResult>,
    pub winner: Option<Uuid>,
}

/// Result for a single option.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OptionResult {
    pub option_id: Uuid,
    pub option_text: String,
    pub count: i32,
    pub weighted_count: f64,
    pub percentage: f64,
}

// ============================================================================
// Query types
// ============================================================================

/// Query for listing votes.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct VoteListQuery {
    pub building_id: Option<Uuid>,
    pub status: Option<Vec<String>>,
    pub created_by: Option<Uuid>,
    pub from_date: Option<chrono::NaiveDate>,
    pub to_date: Option<chrono::NaiveDate>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Eligibility check result.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VoteEligibility {
    pub vote_id: Uuid,
    pub user_id: Uuid,
    pub eligible_units: Vec<EligibleUnit>,
    pub can_vote: bool,
    pub reason: Option<String>,
}

/// Unit eligible for voting.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EligibleUnit {
    pub unit_id: Uuid,
    pub unit_designation: String,
    pub ownership_share: rust_decimal::Decimal,
    pub is_owner: bool,
    pub is_delegated: bool,
    pub delegation_id: Option<Uuid>,
    pub already_voted: bool,
}

/// PDF report data (stub).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VoteReportData {
    pub vote: Vote,
    pub questions: Vec<VoteQuestion>,
    pub results: Option<VoteResults>,
    pub participation_details: Vec<ParticipationDetail>,
    pub generated_at: DateTime<Utc>,
}

/// Participation detail for reports.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct ParticipationDetail {
    pub unit_designation: String,
    pub voted: bool,
    pub vote_weight: rust_decimal::Decimal,
}
