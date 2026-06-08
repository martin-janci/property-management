/**
 * Voting API types (Epic 5: Building Voting & Decisions, FR37-44).
 *
 * Mirrors the backend models in `backend/crates/db/src/models/vote.rs` and the
 * routes in `backend/servers/api-server/src/routes/voting.rs`. JSON field names
 * are the serde defaults (snake_case == Rust field names; no renames). Nullable
 * columns are typed `T | null` to match the wire shape; route wrappers strip
 * `null` → `undefined` for the camelCase UI types.
 */

// ============================================================================
// Enums
// ============================================================================

/** Lifecycle status of a vote (`vote.rs:10-18`). */
export type VoteStatus = 'draft' | 'scheduled' | 'active' | 'closed' | 'cancelled';

/** Question answer mode (`vote.rs:21-28`). */
export type QuestionType = 'yes_no' | 'single_choice' | 'multiple_choice' | 'ranked';

/** Quorum rule (`vote.rs:31-37`). */
export type QuorumType = 'simple_majority' | 'two_thirds' | 'weighted';

/** Immutable audit-trail actions (`vote.rs:40-52`). */
export type VoteAuditAction =
  | 'vote_created'
  | 'vote_published'
  | 'vote_cancelled'
  | 'question_added'
  | 'question_removed'
  | 'ballot_cast'
  | 'ballot_updated'
  | 'comment_added'
  | 'comment_hidden'
  | 'vote_closed'
  | 'results_calculated';

/**
 * Answer payload by question type:
 * - `yes_no`: boolean
 * - `single_choice`: option id (string)
 * - `multiple_choice`: option ids (string[])
 * - `ranked`: option ids in rank order (string[])
 */
export type AnswerValue = boolean | string | string[];

/** Map of `question_id` → answer (the `answers` JSONB payload). */
export type AnswerMap = Record<string, AnswerValue>;

// ============================================================================
// Core entities
// ============================================================================

/** A selectable option on a question (`vote.rs:197-201`). */
export interface QuestionOption {
  id: string;
  text: string;
  order?: number | null;
}

/** Full vote record (`votes` table / `vote.rs:59-86`). */
export interface Vote {
  id: string;
  organization_id: string;
  building_id: string;
  title: string;
  description: string | null;
  start_at: string | null;
  end_at: string;
  status: VoteStatus;
  quorum_type: QuorumType;
  quorum_percentage: number | null;
  allow_delegation: boolean;
  anonymous_voting: boolean;
  participation_count: number;
  eligible_count: number;
  quorum_met: boolean;
  results: VoteResults | null;
  results_calculated_at: string | null;
  created_by: string;
  published_by: string | null;
  published_at: string | null;
  cancelled_by: string | null;
  cancelled_at: string | null;
  cancellation_reason: string | null;
  created_at: string;
  updated_at: string;
}

/** Compact vote row for list views (`vote.rs:125-136`). */
export interface VoteSummary {
  id: string;
  building_id: string;
  title: string;
  description: string | null;
  status: VoteStatus;
  quorum_type: QuorumType;
  start_at: string | null;
  end_at: string;
  participation_count: number;
  eligible_count: number;
  quorum_met: boolean;
  question_count: number;
  created_at: string;
}

/** A question attached to a vote (`vote.rs:204-216`). */
export interface VoteQuestion {
  id: string;
  vote_id: string;
  question_text: string;
  description: string | null;
  question_type: QuestionType;
  options: QuestionOption[];
  display_order: number;
  is_required: boolean;
  created_at: string;
  updated_at: string;
}

/** Vote detail with its questions and (optional) cached results (`vote.rs:139-148`). */
export interface VoteWithDetails extends Vote {
  questions: VoteQuestion[];
}

/** A unit the current user may vote on (`vote.rs:420-429`). */
export interface EligibleUnit {
  unit_id: string;
  unit_designation: string | null;
  ownership_share: string | null;
  is_delegated: boolean;
  delegation_id: string | null;
  already_voted: boolean;
}

/** Result of an eligibility check for the current user (`vote.rs:410-417`). */
export interface VoteEligibility {
  vote_id: string;
  user_id: string;
  eligible_units: EligibleUnit[];
  can_vote: boolean;
  reason: string | null;
}

/** A cast ballot (`vote.rs:245-257`). */
export interface VoteResponse {
  id: string;
  vote_id: string;
  user_id: string;
  unit_id: string;
  delegation_id: string | null;
  is_delegated: boolean;
  answers: AnswerMap;
  vote_weight: string;
  response_hash: string;
  submitted_at: string;
}

/** Confirmation returned after casting a ballot (`vote.rs:270-278`). */
export interface VoteReceipt {
  response_id: string;
  vote_id: string;
  unit_id: string;
  submitted_at: string;
  response_hash: string;
  confirmation_number: string;
}

/** Per-option tally inside a question result. */
export interface OptionResult {
  option_id: string;
  option_text: string;
  count: number;
  weighted_count: number;
  percentage: number;
}

/** Tally for a single question (`vote.rs:372-381`). */
export interface QuestionResult {
  question_id: string;
  question_text: string;
  question_type: QuestionType;
  options: OptionResult[];
  total_responses: number;
  winner: string | null;
}

/** Computed / cached results for a vote (`vote.rs:360-369`). */
export interface VoteResults {
  vote_id: string;
  participation_count: number;
  eligible_count: number;
  participation_rate: number;
  quorum_met: boolean;
  questions: QuestionResult[];
  calculated_at: string;
}

/** Immutable audit-log entry (`vote.rs:331-342`). */
export interface VoteAuditLog {
  id: string;
  vote_id: string;
  user_id: string | null;
  action: VoteAuditAction;
  data_hash: string;
  data_snapshot: unknown | null;
  ip_address: string | null;
  user_agent: string | null;
  created_at: string;
}

/** A discussion comment (`vote.rs:285-299`). */
export interface VoteComment {
  id: string;
  vote_id: string;
  user_id: string;
  parent_id: string | null;
  content: string;
  hidden: boolean;
  hidden_by: string | null;
  hidden_at: string | null;
  hidden_reason: string | null;
  ai_consent: boolean;
  created_at: string;
  updated_at: string;
}

/** Comment flattened with author + reply count (`vote.rs:302-308`). */
export interface VoteCommentWithUser extends VoteComment {
  user_name: string;
  reply_count: number;
}

// ============================================================================
// Request payloads
// ============================================================================

/** `POST /api/v1/voting` (`voting.rs:70-81`). */
export interface CreateVoteRequest {
  building_id: string;
  title: string;
  description?: string | null;
  start_at?: string | null;
  end_at: string;
  quorum_type: QuorumType;
  quorum_percentage?: number | null;
  allow_delegation?: boolean | null;
  anonymous_voting?: boolean | null;
}

/** `PUT /api/v1/voting/{id}` — draft only (`voting.rs:84-94`). */
export interface UpdateVoteRequest {
  title?: string | null;
  description?: string | null;
  start_at?: string | null;
  end_at?: string | null;
  quorum_type?: QuorumType | null;
  quorum_percentage?: number | null;
  allow_delegation?: boolean | null;
  anonymous_voting?: boolean | null;
}

/** `POST /api/v1/voting/{id}/publish` (`voting.rs:97-100`). */
export interface PublishVoteRequest {
  start_at?: string | null;
}

/** `POST /api/v1/voting/{id}/cancel` (`voting.rs:103-106`). */
export interface CancelVoteRequest {
  reason: string;
}

/** `POST /api/v1/voting/{id}/questions` (`voting.rs:109-117`). */
export interface AddQuestionRequest {
  question_text: string;
  description?: string | null;
  question_type: QuestionType;
  options: QuestionOption[];
  display_order?: number | null;
  is_required?: boolean | null;
}

/** `PUT /api/v1/voting/{id}/questions/{question_id}` (`voting.rs:120-127`). */
export interface UpdateQuestionRequest {
  question_text?: string | null;
  description?: string | null;
  options?: QuestionOption[] | null;
  display_order?: number | null;
  is_required?: boolean | null;
}

/** `POST /api/v1/voting/{id}/cast` (`voting.rs:130-135`). */
export interface CastVoteRequest {
  unit_id: string;
  delegation_id?: string | null;
  answers: AnswerMap;
}

/** `POST /api/v1/voting/{id}/comments` (`voting.rs:138-143`). */
export interface AddCommentRequest {
  parent_id?: string | null;
  content: string;
  ai_consent: boolean;
}

/** `POST /api/v1/voting/{id}/comments/{comment_id}/hide` (`voting.rs:146-149`). */
export interface HideCommentRequest {
  reason: string;
}

// ============================================================================
// Query params
// ============================================================================

/** `GET /api/v1/voting` filters (`voting.rs:152-160`). */
export interface ListVotesQuery {
  building_id?: string;
  status?: VoteStatus;
  from_date?: string;
  to_date?: string;
  limit?: number;
  offset?: number;
}
