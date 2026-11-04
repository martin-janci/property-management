/**
 * Disputes Types
 *
 * TypeScript types for Disputes API (Epic 80).
 */

// ============================================
// Core Types (Story 80.1)
// ============================================

/** Dispute category values stored by the backend (`dispute_category`). */
export type DisputeCategory =
  | 'noise'
  | 'damage'
  | 'payment'
  | 'lease_terms'
  | 'common_area'
  | 'parking'
  | 'pets'
  | 'maintenance'
  | 'privacy'
  | 'harassment'
  | 'other';

/** Dispute status values stored by the backend (`dispute_status`). */
export type DisputeListStatus =
  | 'filed'
  | 'under_review'
  | 'mediation'
  | 'awaiting_response'
  | 'resolved'
  | 'escalated'
  | 'withdrawn'
  | 'closed';

export type DisputePriority = 'low' | 'medium' | 'high' | 'urgent';

/** Alias kept for callers that predate the backend-aligned status union. */
export type DisputeStatus = DisputeListStatus;

/**
 * A dispute record. Mirrors backend `Dispute` (snake_case on the wire, mapped
 * to camelCase by the api functions). `mediationNotes` is only returned to
 * managers and the assigned mediator.
 */
export interface Dispute {
  id: string;
  organizationId: string;
  buildingId?: string;
  unitId?: string;
  referenceNumber: string;
  category: DisputeCategory;
  title: string;
  description: string;
  desiredResolution?: string;
  status: DisputeListStatus;
  priority: DisputePriority;
  filedBy: string;
  assignedTo?: string;
  resolvedAt?: string;
  resolutionNotes?: string;
  mediationNotes?: string;
  createdAt: string;
  updatedAt: string;
}

export type DisputePartyRole = 'complainant' | 'respondent' | 'witness' | 'mediator';

/** A party to a dispute, joined with the user's name and email. */
export interface DisputeParty {
  id: string;
  userId: string;
  role: DisputePartyRole;
  userName: string;
  userEmail: string;
  notifiedAt?: string;
  respondedAt?: string;
  createdAt: string;
}

/** `GET /api/v1/disputes/{id}` — the dispute plus its parties and counters. */
export interface DisputeWithDetails extends Dispute {
  parties: DisputeParty[];
  evidenceCount: number;
  activityCount: number;
  sessionCount: number;
}

/**
 * Filters for `GET /api/v1/disputes`. The organization is taken from the
 * caller's JWT tenant, so it is not part of the query.
 */
export interface DisputeFilters {
  status?: DisputeListStatus;
  category?: DisputeCategory;
  priority?: DisputePriority;
  search?: string;
  buildingId?: string;
  filedBy?: string;
  assignedTo?: string;
  fromDate?: string;
  toDate?: string;
}

export interface DisputeListQuery extends DisputeFilters {
  /** 1-based page number. */
  page?: number;
  limit?: number;
}

/** One row of the dispute list. Mirrors backend `DisputeSummary`. */
export interface DisputeListItem {
  id: string;
  referenceNumber: string;
  category: DisputeCategory;
  title: string;
  status: DisputeListStatus;
  priority: DisputePriority;
  filedByName: string;
  assignedToName?: string;
  partyCount: number;
  createdAt: string;
  updatedAt: string;
}

export interface PaginatedDisputes {
  data: DisputeListItem[];
  /**
   * Lower bound on the total row count. The backend returns no count, so this
   * is exact on the last page and `offset + limit + 1` while more pages exist.
   */
  total: number;
  page: number;
  limit: number;
  hasMore: boolean;
}

// ============================================
// Dispute Filing Types (Story 80.2)
// ============================================

/**
 * Body for `POST /api/v1/disputes`. The organization and filer come from the
 * caller's JWT, so neither is sent.
 */
export interface CreateDisputeRequest {
  category: DisputeCategory;
  title: string;
  description: string;
  buildingId?: string;
  unitId?: string;
  desiredResolution?: string;
  respondentIds?: string[];
}

/**
 * Evidence metadata for a dispute. Mirrors backend `DisputeEvidence`.
 * The backend only records metadata of an already-stored file; it has no
 * evidence upload endpoint.
 */
export interface DisputeEvidence {
  id: string;
  disputeId: string;
  uploadedBy: string;
  filename: string;
  originalFilename: string;
  contentType: string;
  sizeBytes: number;
  storageUrl: string;
  description?: string;
  createdAt: string;
}

// ============================================
// Mediation & Resolution Types (Story 80.3)
// ============================================

export type ResolutionType =
  | 'mutual_agreement'
  | 'favor_filer'
  | 'favor_respondent'
  | 'withdrawn'
  | 'dismissed';

/**
 * Resolution form input. The backend stores a single free-text
 * `resolution_notes`; {@link resolveDispute} composes it from these fields.
 */
export interface ResolveDisputeRequest {
  resolutionType: ResolutionType;
  resolutionDetails: string;
  terms?: string;
  requiresConfirmation: boolean;
}

/** Adds the user as the dispute's mediator (`POST /{id}/parties`, role `mediator`). */
export interface AssignMediatorRequest {
  mediatorId: string;
}

export type EscalationSeverity = 'warning' | 'formal_notice' | 'legal_referral';

/** Body for `POST /api/v1/disputes/{id}/escalations`. */
export interface EscalateDisputeRequest {
  reason: string;
  severity?: EscalationSeverity;
  escalateTo?: string;
  actionItemId?: string;
}

/** An escalation record. Mirrors backend `Escalation`. */
export interface Escalation {
  id: string;
  disputeId: string;
  actionItemId?: string;
  escalatedBy: string;
  escalatedTo?: string;
  reason: string;
  severity: EscalationSeverity;
  resolved: boolean;
  resolvedAt?: string;
  resolutionNotes?: string;
  createdAt: string;
  updatedAt: string;
}

/** Body for `PATCH /api/v1/disputes/{id}` (manager only). */
export interface UpdateDisputeStatusRequest {
  status: DisputeListStatus;
  reason?: string;
}

// ============================================
// Mediation Session Types (Story 80.3)
// ============================================

export type SessionType = 'in_person' | 'video_call' | 'phone' | 'written';

export type SessionStatus = 'scheduled' | 'in_progress' | 'completed' | 'cancelled' | 'rescheduled';

/**
 * A mediation session as returned by the api-server.
 *
 * Field names mirror the backend `MediationSession` model exactly. The
 * backend serializes with serde defaults (no `rename_all`), so the wire
 * format is snake_case — these property names match the JSON on the wire.
 */
export interface MediationSession {
  id: string;
  dispute_id: string;
  mediator_id: string;
  session_type: SessionType;
  scheduled_at: string;
  duration_minutes?: number | null;
  location?: string | null;
  meeting_url?: string | null;
  status: SessionStatus;
  notes?: string | null;
  outcome?: string | null;
  created_at: string;
  updated_at: string;
}

/**
 * Request body for scheduling a session
 * (`POST /api/v1/disputes/{id}/sessions`).
 *
 * The backend derives `dispute_id` from the path and `mediator_id` from the
 * authenticated user, so neither needs to be sent by the client.
 */
export interface ScheduleSessionRequest {
  session_type: SessionType;
  scheduled_at: string;
  duration_minutes?: number | null;
  location?: string | null;
  meeting_url?: string | null;
  attendee_party_ids?: string[];
}

/**
 * Partial update for an existing session
 * (`PATCH /api/v1/disputes/{id}/sessions/{sessionId}`) — reschedule,
 * relocate, or change status.
 */
export interface UpdateSessionRequest {
  scheduled_at?: string;
  duration_minutes?: number | null;
  location?: string | null;
  meeting_url?: string | null;
  status?: SessionStatus;
}

// ============================================
// Party Submission Types (Story 80.3)
// ============================================

/**
 * A submission filed by a party during mediation
 * (`GET /api/v1/disputes/{id}/submissions`).
 *
 * Field names mirror the backend `PartySubmission` model exactly. The backend
 * serializes with serde defaults (no `rename_all`), so the wire format is
 * snake_case — these property names match the JSON on the wire.
 */
export interface PartySubmission {
  id: string;
  dispute_id: string;
  party_id: string;
  submission_type: string;
  content: string;
  is_visible_to_all: boolean;
  created_at: string;
}

/**
 * Request body for submitting a party response
 * (`POST /api/v1/disputes/{id}/submissions`).
 *
 * The backend derives `dispute_id` from the path and `party_id` from the
 * authenticated user's party membership, so neither needs to be sent.
 */
export interface SubmitResponseRequest {
  submission_type: string;
  content: string;
  is_visible_to_all: boolean;
}

// ============================================
// Timeline Types (Story 80.3)
// ============================================

/** Backend `activity_type` values recorded in a dispute's activity log. */
export type TimelineEventType =
  | 'dispute_filed'
  | 'status_changed'
  | 'party_added'
  | 'evidence_added'
  | 'session_scheduled'
  | 'session_completed'
  | 'resolution_proposed'
  | 'resolution_voted'
  | 'resolution_accepted'
  | 'action_created'
  | 'action_completed'
  | 'comment_added'
  | 'escalated'
  | 'closed';

/**
 * One activity-log entry (`GET /api/v1/disputes/{id}/activities`). The backend
 * returns only the actor id; resolve a display name from the dispute parties.
 */
export interface TimelineEvent {
  id: string;
  disputeId: string;
  eventType: TimelineEventType;
  actorId: string;
  description: string;
  metadata?: Record<string, unknown>;
  createdAt: string;
}

export interface TimelineQuery {
  limit?: number;
  offset?: number;
}

// ============================================
// Resolution Tracking Types (Story 77.3)
// ============================================

export type ResolutionStatus =
  | 'proposed'
  | 'accepted'
  | 'rejected'
  | 'partially_accepted'
  | 'implemented';

export interface ResolutionTerm {
  id: string;
  description: string;
  responsiblePartyId?: string;
  deadline?: string;
  completed: boolean;
  completedAt?: string;
}

/** A proposed resolution. Mirrors backend `DisputeResolution`. */
export interface DisputeResolution {
  id: string;
  disputeId: string;
  proposedBy: string;
  resolutionText: string;
  terms: ResolutionTerm[];
  status: ResolutionStatus;
  proposedAt: string;
  acceptedAt?: string;
  implementedAt?: string;
  createdAt: string;
  updatedAt: string;
}

export interface ResolutionVote {
  id: string;
  resolutionId: string;
  partyId: string;
  accepted: boolean;
  comments?: string;
  votedAt: string;
}

/** `GET /api/v1/disputes/{id}/resolutions/{rid}` */
export interface ResolutionWithVotes {
  resolution: DisputeResolution;
  votes: ResolutionVote[];
  acceptanceRate: number;
}

export interface ProposeResolutionTermInput {
  description: string;
  deadline?: string;
  responsiblePartyId?: string;
}

export interface ProposeResolutionRequest {
  resolutionText: string;
  terms: ProposeResolutionTermInput[];
}

export interface VoteOnResolutionRequest {
  accepted: boolean;
  comments?: string;
}

// ============================================
// Enforcement Types (Story 77.4)
// ============================================

export type ActionItemStatus = 'pending' | 'in_progress' | 'completed' | 'overdue' | 'escalated';

/** A post-resolution action item. Mirrors backend `ActionItem`. */
export interface DisputeActionItem {
  id: string;
  disputeId: string;
  resolutionId?: string;
  resolutionTermId?: string;
  assignedTo: string;
  title: string;
  description: string;
  dueDate: string;
  status: ActionItemStatus;
  completedAt?: string;
  completionNotes?: string;
  reminderSentAt?: string;
  escalatedAt?: string;
  createdAt: string;
  updatedAt: string;
}

export interface CreateActionItemRequest {
  title: string;
  description: string;
  assignedTo: string;
  /** ISO date-time; a bare `YYYY-MM-DD` date is sent as the end of that day (UTC). */
  dueDate: string;
  resolutionId?: string;
  resolutionTermId?: string;
}

// ============================================
// Statistics Types
// ============================================

/** `GET /api/v1/disputes/statistics` for the caller's organization. */
export interface DisputeStatistics {
  totalDisputes: number;
  byStatus: Partial<Record<DisputeListStatus, number>>;
  byCategory: Partial<Record<DisputeCategory, number>>;
  byPriority: Partial<Record<DisputePriority, number>>;
  averageResolutionDays?: number;
  pendingActions: number;
  overdueActions: number;
}
