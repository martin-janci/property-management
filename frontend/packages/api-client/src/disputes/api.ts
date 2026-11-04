/**
 * Disputes API Client
 *
 * API functions for Disputes (Epic 80).
 */

import { bearerAuthHeader } from '../auth';
import type {
  AssignMediatorRequest,
  CreateActionItemRequest,
  CreateDisputeRequest,
  Dispute,
  DisputeActionItem,
  DisputeEvidence,
  DisputeListItem,
  DisputeListQuery,
  DisputeParty,
  DisputeResolution,
  DisputeStatistics,
  DisputeWithDetails,
  EscalateDisputeRequest,
  Escalation,
  MediationSession,
  PaginatedDisputes,
  PartySubmission,
  ProposeResolutionRequest,
  ResolutionVote,
  ResolutionWithVotes,
  ResolveDisputeRequest,
  ScheduleSessionRequest,
  SubmitResponseRequest,
  TimelineEvent,
  TimelineQuery,
  UpdateDisputeStatusRequest,
  UpdateSessionRequest,
  VoteOnResolutionRequest,
} from './types';

const API_BASE = '/api/v1/disputes';

// Helper function to build query string
function buildQueryString(params: object): string {
  const searchParams = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined && value !== null) {
      searchParams.append(key, String(value));
    }
  }
  const queryString = searchParams.toString();
  return queryString ? `?${queryString}` : '';
}

// Helper for API requests
async function apiRequest<T>(url: string, options: RequestInit = {}): Promise<T> {
  const response = await fetch(url, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...bearerAuthHeader(),
      ...options.headers,
    },
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({}));
    throw new Error(error.message || `HTTP error ${response.status}`);
  }

  if (response.status === 204) {
    return undefined as T;
  }

  return response.json();
}

/** Nil UUID for body fields the backend requires but always overwrites. */
const NIL_UUID = '00000000-0000-0000-0000-000000000000';

/** `undefined` for null/absent wire values so optional fields stay optional. */
function opt<T>(v: T | null | undefined): T | undefined {
  return v ?? undefined;
}

// ============================================
// Wire types (backend serde defaults: snake_case)
// ============================================

interface DisputeWire {
  id: string;
  organization_id: string;
  building_id?: string | null;
  unit_id?: string | null;
  reference_number: string;
  category: Dispute['category'];
  title: string;
  description: string;
  desired_resolution?: string | null;
  status: Dispute['status'];
  priority: Dispute['priority'];
  filed_by: string;
  assigned_to?: string | null;
  resolved_at?: string | null;
  resolution_notes?: string | null;
  mediation_notes?: string | null;
  created_at: string;
  updated_at: string;
}

interface DisputePartyWire {
  id: string;
  dispute_id: string;
  user_id: string;
  role: DisputeParty['role'];
  notified_at?: string | null;
  responded_at?: string | null;
  created_at: string;
}

interface DisputeWithDetailsWire {
  dispute: DisputeWire;
  parties: Array<{ party: DisputePartyWire; user_name: string; user_email: string }>;
  evidence_count: number;
  activity_count: number;
  session_count: number;
}

interface DisputeEvidenceWire {
  id: string;
  dispute_id: string;
  uploaded_by: string;
  filename: string;
  original_filename: string;
  content_type: string;
  size_bytes: number;
  storage_url: string;
  description?: string | null;
  created_at: string;
}

interface DisputeActivityWire {
  id: string;
  dispute_id: string;
  actor_id: string;
  activity_type: TimelineEvent['eventType'];
  description: string;
  metadata?: Record<string, unknown> | null;
  created_at: string;
}

interface DisputeResolutionWire {
  id: string;
  dispute_id: string;
  proposed_by: string;
  resolution_text: string;
  terms: Array<{
    id: string;
    description: string;
    responsible_party_id?: string | null;
    deadline?: string | null;
    completed: boolean;
    completed_at?: string | null;
  }>;
  status: DisputeResolution['status'];
  proposed_at: string;
  accepted_at?: string | null;
  implemented_at?: string | null;
  created_at: string;
  updated_at: string;
}

interface ResolutionVoteWire {
  id: string;
  resolution_id: string;
  party_id: string;
  accepted: boolean;
  comments?: string | null;
  voted_at: string;
}

interface ActionItemWire {
  id: string;
  dispute_id: string;
  resolution_id?: string | null;
  resolution_term_id?: string | null;
  assigned_to: string;
  title: string;
  description: string;
  due_date: string;
  status: DisputeActionItem['status'];
  completed_at?: string | null;
  completion_notes?: string | null;
  reminder_sent_at?: string | null;
  escalated_at?: string | null;
  created_at: string;
  updated_at: string;
}

interface EscalationWire {
  id: string;
  dispute_id: string;
  action_item_id?: string | null;
  escalated_by: string;
  escalated_to?: string | null;
  reason: string;
  severity: Escalation['severity'];
  resolved: boolean;
  resolved_at?: string | null;
  resolution_notes?: string | null;
  created_at: string;
  updated_at: string;
}

interface DisputeStatisticsWire {
  total_disputes: number;
  by_status: Array<{ status: string; count: number }>;
  by_category: Array<{ category: string; count: number }>;
  by_priority: Array<{ priority: string; count: number }>;
  avg_resolution_days?: number | null;
  pending_actions: number;
  overdue_actions: number;
}

// ============================================
// Wire -> client mappers (exported for tests)
// ============================================

export function mapDispute(w: DisputeWire): Dispute {
  return {
    id: w.id,
    organizationId: w.organization_id,
    buildingId: opt(w.building_id),
    unitId: opt(w.unit_id),
    referenceNumber: w.reference_number,
    category: w.category,
    title: w.title,
    description: w.description,
    desiredResolution: opt(w.desired_resolution),
    status: w.status,
    priority: w.priority,
    filedBy: w.filed_by,
    assignedTo: opt(w.assigned_to),
    resolvedAt: opt(w.resolved_at),
    resolutionNotes: opt(w.resolution_notes),
    mediationNotes: opt(w.mediation_notes),
    createdAt: w.created_at,
    updatedAt: w.updated_at,
  };
}

export function mapDisputeWithDetails(w: DisputeWithDetailsWire): DisputeWithDetails {
  return {
    ...mapDispute(w.dispute),
    parties: (w.parties ?? []).map((p) => ({
      id: p.party.id,
      userId: p.party.user_id,
      role: p.party.role,
      userName: p.user_name,
      userEmail: p.user_email,
      notifiedAt: opt(p.party.notified_at),
      respondedAt: opt(p.party.responded_at),
      createdAt: p.party.created_at,
    })),
    evidenceCount: w.evidence_count,
    activityCount: w.activity_count,
    sessionCount: w.session_count,
  };
}

export function mapEvidence(w: DisputeEvidenceWire): DisputeEvidence {
  return {
    id: w.id,
    disputeId: w.dispute_id,
    uploadedBy: w.uploaded_by,
    filename: w.filename,
    originalFilename: w.original_filename,
    contentType: w.content_type,
    sizeBytes: w.size_bytes,
    storageUrl: w.storage_url,
    description: opt(w.description),
    createdAt: w.created_at,
  };
}

export function mapActivity(w: DisputeActivityWire): TimelineEvent {
  return {
    id: w.id,
    disputeId: w.dispute_id,
    eventType: w.activity_type,
    actorId: w.actor_id,
    description: w.description,
    metadata: opt(w.metadata),
    createdAt: w.created_at,
  };
}

export function mapResolution(w: DisputeResolutionWire): DisputeResolution {
  return {
    id: w.id,
    disputeId: w.dispute_id,
    proposedBy: w.proposed_by,
    resolutionText: w.resolution_text,
    terms: (w.terms ?? []).map((t) => ({
      id: t.id,
      description: t.description,
      responsiblePartyId: opt(t.responsible_party_id),
      deadline: opt(t.deadline),
      completed: t.completed,
      completedAt: opt(t.completed_at),
    })),
    status: w.status,
    proposedAt: w.proposed_at,
    acceptedAt: opt(w.accepted_at),
    implementedAt: opt(w.implemented_at),
    createdAt: w.created_at,
    updatedAt: w.updated_at,
  };
}

export function mapVote(w: ResolutionVoteWire): ResolutionVote {
  return {
    id: w.id,
    resolutionId: w.resolution_id,
    partyId: w.party_id,
    accepted: w.accepted,
    comments: opt(w.comments),
    votedAt: w.voted_at,
  };
}

export function mapActionItem(w: ActionItemWire): DisputeActionItem {
  return {
    id: w.id,
    disputeId: w.dispute_id,
    resolutionId: opt(w.resolution_id),
    resolutionTermId: opt(w.resolution_term_id),
    assignedTo: w.assigned_to,
    title: w.title,
    description: w.description,
    dueDate: w.due_date,
    status: w.status,
    completedAt: opt(w.completed_at),
    completionNotes: opt(w.completion_notes),
    reminderSentAt: opt(w.reminder_sent_at),
    escalatedAt: opt(w.escalated_at),
    createdAt: w.created_at,
    updatedAt: w.updated_at,
  };
}

export function mapEscalation(w: EscalationWire): Escalation {
  return {
    id: w.id,
    disputeId: w.dispute_id,
    actionItemId: opt(w.action_item_id),
    escalatedBy: w.escalated_by,
    escalatedTo: opt(w.escalated_to),
    reason: w.reason,
    severity: w.severity,
    resolved: w.resolved,
    resolvedAt: opt(w.resolved_at),
    resolutionNotes: opt(w.resolution_notes),
    createdAt: w.created_at,
    updatedAt: w.updated_at,
  };
}

function countsByKey<K extends string>(
  rows: Array<Record<string, string | number>>,
  key: string
): Partial<Record<K, number>> {
  return Object.fromEntries(rows.map((r) => [String(r[key]), Number(r.count)])) as Partial<
    Record<K, number>
  >;
}

export function mapStatistics(w: DisputeStatisticsWire): DisputeStatistics {
  return {
    totalDisputes: w.total_disputes,
    byStatus: countsByKey(w.by_status ?? [], 'status'),
    byCategory: countsByKey(w.by_category ?? [], 'category'),
    byPriority: countsByKey(w.by_priority ?? [], 'priority'),
    averageResolutionDays: opt(w.avg_resolution_days),
    pendingActions: w.pending_actions,
    overdueActions: w.overdue_actions,
  };
}

// ============================================
// Dispute Listing & Details (Story 80.1)
// ============================================

/** Backend `DisputeSummary` row (snake_case on the wire). */
interface DisputeSummaryWire {
  id: string;
  reference_number: string;
  category: DisputeListItem['category'];
  title: string;
  status: DisputeListItem['status'];
  priority: DisputeListItem['priority'];
  filed_by_name: string;
  assigned_to_name?: string | null;
  party_count: number;
  created_at: string;
  updated_at: string;
}

/**
 * List disputes for the caller's organization (`GET /api/v1/disputes`).
 *
 * `organizationId` must be the caller's own organization; the backend scopes
 * the list to the JWT tenant and rejects a mismatching id with 403.
 */
export async function listDisputes(
  organizationId: string,
  query: DisputeListQuery = {}
): Promise<PaginatedDisputes> {
  const limit = query.limit ?? 20;
  const page = Math.max(1, query.page ?? 1);
  const offset = (page - 1) * limit;
  const qs = buildQueryString({
    organization_id: organizationId,
    status: query.status,
    category: query.category,
    priority: query.priority,
    search: query.search?.trim() || undefined,
    building_id: query.buildingId,
    filed_by: query.filedBy,
    assigned_to: query.assignedTo,
    from_date: query.fromDate,
    to_date: query.toDate,
    // Ask for one extra row to learn whether another page exists.
    limit: limit + 1,
    offset,
  });
  const rows = await apiRequest<DisputeSummaryWire[]>(`${API_BASE}${qs}`);
  const hasMore = rows.length > limit;
  const data: DisputeListItem[] = rows.slice(0, limit).map((r) => ({
    id: r.id,
    referenceNumber: r.reference_number,
    category: r.category,
    title: r.title,
    status: r.status,
    priority: r.priority,
    filedByName: r.filed_by_name,
    assignedToName: r.assigned_to_name ?? undefined,
    partyCount: r.party_count,
    createdAt: r.created_at,
    updatedAt: r.updated_at,
  }));
  return {
    data,
    total: offset + data.length + (hasMore ? 1 : 0),
    page,
    limit,
    hasMore,
  };
}

/** `GET /api/v1/disputes/{id}` — dispute, parties (with names) and counters. */
export async function getDispute(id: string): Promise<DisputeWithDetails> {
  const wire = await apiRequest<DisputeWithDetailsWire>(`${API_BASE}/${id}`);
  return mapDisputeWithDetails(wire);
}

/** `GET /api/v1/disputes/statistics` — scoped to the caller's JWT tenant. */
export async function getDisputeStatistics(organizationId: string): Promise<DisputeStatistics> {
  const qs = buildQueryString({ organization_id: organizationId });
  const wire = await apiRequest<DisputeStatisticsWire>(`${API_BASE}/statistics${qs}`);
  return mapStatistics(wire);
}

// ============================================
// Dispute Filing (Story 80.2)
// ============================================

/**
 * `POST /api/v1/disputes`. The organization comes from the JWT tenant (a
 * mismatching `organization_id` is rejected with 403) and the filer is always
 * the caller.
 */
export async function createDispute(
  organizationId: string,
  data: CreateDisputeRequest
): Promise<Dispute> {
  const wire = await apiRequest<DisputeWire>(API_BASE, {
    method: 'POST',
    body: JSON.stringify({
      organization_id: organizationId,
      category: data.category,
      title: data.title,
      description: data.description,
      building_id: data.buildingId || undefined,
      unit_id: data.unitId || undefined,
      desired_resolution: data.desiredResolution?.trim() || undefined,
      respondent_ids: data.respondentIds ?? [],
    }),
  });
  return mapDispute(wire);
}

export async function listEvidence(disputeId: string): Promise<DisputeEvidence[]> {
  const rows = await apiRequest<DisputeEvidenceWire[]>(`${API_BASE}/${disputeId}/evidence`);
  return rows.map(mapEvidence);
}

export async function deleteEvidence(disputeId: string, evidenceId: string): Promise<void> {
  return apiRequest<void>(`${API_BASE}/${disputeId}/evidence/${evidenceId}`, {
    method: 'DELETE',
  });
}

// ============================================
// Status, Mediation & Resolution (Story 80.3)
// ============================================

/**
 * Assign a mediator by adding them as a `mediator` party
 * (`POST /api/v1/disputes/{id}/parties`). The backend has no dedicated
 * assignment route.
 */
export async function assignMediator(
  disputeId: string,
  data: AssignMediatorRequest
): Promise<DisputeParty> {
  const wire = await apiRequest<DisputePartyWire>(`${API_BASE}/${disputeId}/parties`, {
    method: 'POST',
    body: JSON.stringify({ user_id: data.mediatorId, role: 'mediator' }),
  });
  return {
    id: wire.id,
    userId: wire.user_id,
    role: wire.role,
    // The add-party response carries no user join; callers refetch the detail.
    userName: '',
    userEmail: '',
    notifiedAt: opt(wire.notified_at),
    respondedAt: opt(wire.responded_at),
    createdAt: wire.created_at,
  };
}

/** `PATCH /api/v1/disputes/{id}` — manager only; enforces the status state machine. */
export async function updateDisputeStatus(
  disputeId: string,
  data: UpdateDisputeStatusRequest
): Promise<Dispute> {
  const wire = await apiRequest<DisputeWire>(`${API_BASE}/${disputeId}`, {
    method: 'PATCH',
    body: JSON.stringify({ status: data.status, reason: data.reason?.trim() || undefined }),
  });
  return mapDispute(wire);
}

/** Compose the backend's single free-text `resolution_notes` from the form fields. */
export function composeResolutionNotes(data: ResolveDisputeRequest): string {
  const parts = [`[${data.resolutionType}] ${data.resolutionDetails.trim()}`];
  if (data.terms?.trim()) parts.push(data.terms.trim());
  return parts.join('\n\n');
}

/** `PATCH /api/v1/disputes/{id}/resolve` — manager/admin only. */
export async function resolveDispute(
  disputeId: string,
  data: ResolveDisputeRequest
): Promise<Dispute> {
  const wire = await apiRequest<DisputeWire>(`${API_BASE}/${disputeId}/resolve`, {
    method: 'PATCH',
    body: JSON.stringify({ resolution_notes: composeResolutionNotes(data) }),
  });
  return mapDispute(wire);
}

/**
 * Record an escalation (`POST /api/v1/disputes/{id}/escalations`). This does
 * not change the dispute status; use {@link updateDisputeStatus} for that.
 */
export async function escalateDispute(
  disputeId: string,
  data: EscalateDisputeRequest
): Promise<Escalation> {
  const wire = await apiRequest<EscalationWire>(`${API_BASE}/${disputeId}/escalations`, {
    method: 'POST',
    body: JSON.stringify({
      reason: data.reason,
      severity: data.severity ?? 'formal_notice',
      escalated_to: data.escalateTo || undefined,
      action_item_id: data.actionItemId || undefined,
    }),
  });
  return mapEscalation(wire);
}

// ============================================
// Resolution Tracking (Story 77.3)
// ============================================

export async function listResolutions(disputeId: string): Promise<DisputeResolution[]> {
  const rows = await apiRequest<DisputeResolutionWire[]>(`${API_BASE}/${disputeId}/resolutions`);
  return rows.map(mapResolution);
}

export async function getResolution(
  disputeId: string,
  resolutionId: string
): Promise<ResolutionWithVotes> {
  const wire = await apiRequest<{
    resolution: DisputeResolutionWire;
    votes: ResolutionVoteWire[];
    acceptance_rate: number;
  }>(`${API_BASE}/${disputeId}/resolutions/${resolutionId}`);
  return {
    resolution: mapResolution(wire.resolution),
    votes: (wire.votes ?? []).map(mapVote),
    acceptanceRate: wire.acceptance_rate,
  };
}

/**
 * `POST /api/v1/disputes/{id}/resolutions`. The backend flattens a model that
 * also requires `dispute_id` / `proposed_by`; it overwrites both from the path
 * and the caller, so placeholders are sent.
 */
export async function proposeResolution(
  disputeId: string,
  data: ProposeResolutionRequest
): Promise<DisputeResolution> {
  const wire = await apiRequest<DisputeResolutionWire>(`${API_BASE}/${disputeId}/resolutions`, {
    method: 'POST',
    body: JSON.stringify({
      dispute_id: disputeId,
      proposed_by: NIL_UUID,
      resolution_text: data.resolutionText,
      terms: data.terms.map((term, i) => ({
        id: `term-${i + 1}`,
        description: term.description,
        responsible_party_id: term.responsiblePartyId || undefined,
        deadline: term.deadline ? toEndOfDayIso(term.deadline) : undefined,
        completed: false,
      })),
    }),
  });
  return mapResolution(wire);
}

/** `POST .../resolutions/{rid}/vote` — only parties to the dispute may vote. */
export async function voteOnResolution(
  disputeId: string,
  resolutionId: string,
  data: VoteOnResolutionRequest
): Promise<ResolutionVote> {
  const wire = await apiRequest<ResolutionVoteWire>(
    `${API_BASE}/${disputeId}/resolutions/${resolutionId}/vote`,
    {
      method: 'POST',
      body: JSON.stringify({ accepted: data.accepted, comments: data.comments || undefined }),
    }
  );
  return mapVote(wire);
}

export async function acceptResolution(
  disputeId: string,
  resolutionId: string
): Promise<DisputeResolution> {
  const wire = await apiRequest<DisputeResolutionWire>(
    `${API_BASE}/${disputeId}/resolutions/${resolutionId}/accept`,
    { method: 'POST' }
  );
  return mapResolution(wire);
}

export async function implementResolution(
  disputeId: string,
  resolutionId: string
): Promise<DisputeResolution> {
  const wire = await apiRequest<DisputeResolutionWire>(
    `${API_BASE}/${disputeId}/resolutions/${resolutionId}/implement`,
    { method: 'POST' }
  );
  return mapResolution(wire);
}

// ============================================
// Enforcement (Story 77.4)
// ============================================

/** A bare `YYYY-MM-DD` becomes the end of that day in UTC; anything else passes through. */
function toEndOfDayIso(date: string): string {
  return /^\d{4}-\d{2}-\d{2}$/.test(date) ? `${date}T23:59:59Z` : date;
}

export async function listActionItems(disputeId: string): Promise<DisputeActionItem[]> {
  const rows = await apiRequest<ActionItemWire[]>(`${API_BASE}/${disputeId}/actions`);
  return rows.map(mapActionItem);
}

/** `POST /api/v1/disputes/{id}/actions` (`dispute_id` is overwritten from the path). */
export async function createActionItem(
  disputeId: string,
  data: CreateActionItemRequest
): Promise<DisputeActionItem> {
  const wire = await apiRequest<ActionItemWire>(`${API_BASE}/${disputeId}/actions`, {
    method: 'POST',
    body: JSON.stringify({
      dispute_id: disputeId,
      resolution_id: data.resolutionId || undefined,
      resolution_term_id: data.resolutionTermId || undefined,
      assigned_to: data.assignedTo,
      title: data.title,
      description: data.description,
      due_date: toEndOfDayIso(data.dueDate),
    }),
  });
  return mapActionItem(wire);
}

export async function completeActionItem(
  disputeId: string,
  actionId: string,
  completionNotes?: string
): Promise<DisputeActionItem> {
  const wire = await apiRequest<ActionItemWire>(
    `${API_BASE}/${disputeId}/actions/${actionId}/complete`,
    {
      method: 'POST',
      body: JSON.stringify({ completion_notes: completionNotes?.trim() || undefined }),
    }
  );
  return mapActionItem(wire);
}

/** `POST .../actions/{aid}/remind` — 204 No Content. */
export async function sendActionReminder(disputeId: string, actionId: string): Promise<void> {
  return apiRequest<void>(`${API_BASE}/${disputeId}/actions/${actionId}/remind`, {
    method: 'POST',
  });
}

// ============================================
// Mediation Sessions (Story 80.3)
// ============================================

/** List all mediation sessions for a dispute. */
export async function listSessions(disputeId: string): Promise<MediationSession[]> {
  return apiRequest<MediationSession[]>(`${API_BASE}/${disputeId}/sessions`);
}

/**
 * Schedule a new mediation session. The backend flattens a model that also
 * requires `dispute_id`, `mediator_id` and `attendee_party_ids`; it overwrites
 * the first two from the path and the caller, so placeholders are sent.
 */
export async function scheduleSession(
  disputeId: string,
  data: ScheduleSessionRequest
): Promise<MediationSession> {
  return apiRequest<MediationSession>(`${API_BASE}/${disputeId}/sessions`, {
    method: 'POST',
    body: JSON.stringify({
      ...data,
      dispute_id: disputeId,
      mediator_id: NIL_UUID,
      attendee_party_ids: data.attendee_party_ids ?? [],
    }),
  });
}

/** Update an existing session — used for rescheduling and relocating. */
export async function updateSession(
  disputeId: string,
  sessionId: string,
  data: UpdateSessionRequest
): Promise<MediationSession> {
  return apiRequest<MediationSession>(`${API_BASE}/${disputeId}/sessions/${sessionId}`, {
    method: 'PATCH',
    body: JSON.stringify(data),
  });
}

/** Cancel a scheduled session. */
export async function cancelSession(
  disputeId: string,
  sessionId: string
): Promise<MediationSession> {
  return apiRequest<MediationSession>(`${API_BASE}/${disputeId}/sessions/${sessionId}/cancel`, {
    method: 'POST',
  });
}

// ============================================
// Party Submissions (Story 80.3)
// ============================================

/** List all party submissions for a dispute. */
export async function listSubmissions(disputeId: string): Promise<PartySubmission[]> {
  return apiRequest<PartySubmission[]>(`${API_BASE}/${disputeId}/submissions`);
}

/**
 * Submit a party response/statement for a dispute. The backend resolves the
 * submitting party from the authenticated user's membership in the dispute.
 */
export async function submitResponse(
  disputeId: string,
  data: SubmitResponseRequest
): Promise<PartySubmission> {
  return apiRequest<PartySubmission>(`${API_BASE}/${disputeId}/submissions`, {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

// ============================================
// Timeline (Story 80.3)
// ============================================

/** `GET /api/v1/disputes/{id}/activities` — newest-first activity log. */
export async function getTimeline(
  disputeId: string,
  query?: TimelineQuery
): Promise<TimelineEvent[]> {
  const qs = buildQueryString({ limit: query?.limit, offset: query?.offset });
  const rows = await apiRequest<DisputeActivityWire[]>(`${API_BASE}/${disputeId}/activities${qs}`);
  return rows.map(mapActivity);
}
