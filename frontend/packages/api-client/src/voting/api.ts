/**
 * Voting API client functions (Epic 5: Building Voting & Decisions, FR37-44).
 *
 * Thin fetch wrappers over `/api/v1/voting` (see
 * `backend/servers/api-server/src/routes/voting.rs`). Mirrors the conventions in
 * the `faults` module: a shared `fetchApi<T>` helper + `buildQueryString`.
 */

import type {
  AddCommentRequest,
  AddQuestionRequest,
  CancelVoteRequest,
  CastVoteRequest,
  CreateVoteRequest,
  HideCommentRequest,
  ListVotesQuery,
  PublishVoteRequest,
  UpdateQuestionRequest,
  UpdateVoteRequest,
  Vote,
  VoteAuditLog,
  VoteComment,
  VoteCommentWithUser,
  VoteEligibility,
  VoteQuestion,
  VoteReceipt,
  VoteResponse,
  VoteResults,
  VoteSummary,
  VoteWithDetails,
} from './types';

const API_BASE = '/api/v1/voting';

/** Helper to build query string from params. */
function buildQueryString(
  params: Record<string, string | number | boolean | undefined | null>
): string {
  const searchParams = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined && value !== null && value !== '') {
      searchParams.append(key, String(value));
    }
  }
  const query = searchParams.toString();
  return query ? `?${query}` : '';
}

/** Generic fetch wrapper with error handling. */
async function fetchApi<T>(url: string, options?: RequestInit): Promise<T> {
  const response = await fetch(url, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({ message: 'Request failed' }));
    throw new Error(error.message || `HTTP ${response.status}`);
  }

  // 204 No Content
  if (response.status === 204) {
    return undefined as T;
  }
  return response.json();
}

// ============================================================================
// Vote CRUD
// ============================================================================

/** List votes with optional filters. */
export async function listVotes(query: ListVotesQuery = {}): Promise<VoteSummary[]> {
  const queryString = buildQueryString({
    building_id: query.building_id,
    status: query.status,
    from_date: query.from_date,
    to_date: query.to_date,
    limit: query.limit,
    offset: query.offset,
  });
  return fetchApi<VoteSummary[]>(`${API_BASE}${queryString}`);
}

/** Get a vote with its questions (and cached results, when closed). */
export async function getVote(id: string): Promise<VoteWithDetails> {
  return fetchApi<VoteWithDetails>(`${API_BASE}/${id}`);
}

/** Create a new vote (starts in `draft`). */
export async function createVote(data: CreateVoteRequest): Promise<Vote> {
  return fetchApi<Vote>(API_BASE, {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

/** Update a vote — draft only. */
export async function updateVote(id: string, data: UpdateVoteRequest): Promise<Vote> {
  return fetchApi<Vote>(`${API_BASE}/${id}`, {
    method: 'PUT',
    body: JSON.stringify(data),
  });
}

/** Delete a vote — draft only. */
export async function deleteVote(id: string): Promise<void> {
  await fetchApi<void>(`${API_BASE}/${id}`, { method: 'DELETE' });
}

// ============================================================================
// Workflow transitions
// ============================================================================

/** Publish a draft vote (→ scheduled or active). */
export async function publishVote(id: string, data: PublishVoteRequest = {}): Promise<Vote> {
  return fetchApi<Vote>(`${API_BASE}/${id}/publish`, {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

/** Cancel a vote with a required reason. */
export async function cancelVote(id: string, data: CancelVoteRequest): Promise<Vote> {
  return fetchApi<Vote>(`${API_BASE}/${id}/cancel`, {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

/** Close an active vote and compute final results. */
export async function closeVote(id: string): Promise<Vote> {
  return fetchApi<Vote>(`${API_BASE}/${id}/close`, { method: 'POST' });
}

// ============================================================================
// Questions
// ============================================================================

/** Add a question to a draft vote. */
export async function addQuestion(voteId: string, data: AddQuestionRequest): Promise<VoteQuestion> {
  return fetchApi<VoteQuestion>(`${API_BASE}/${voteId}/questions`, {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

/** List a vote's questions. */
export async function listQuestions(voteId: string): Promise<VoteQuestion[]> {
  return fetchApi<VoteQuestion[]>(`${API_BASE}/${voteId}/questions`);
}

/** Update a question on a draft vote. */
export async function updateQuestion(
  voteId: string,
  questionId: string,
  data: UpdateQuestionRequest
): Promise<VoteQuestion> {
  return fetchApi<VoteQuestion>(`${API_BASE}/${voteId}/questions/${questionId}`, {
    method: 'PUT',
    body: JSON.stringify(data),
  });
}

/** Delete a question from a draft vote. */
export async function deleteQuestion(voteId: string, questionId: string): Promise<void> {
  await fetchApi<void>(`${API_BASE}/${voteId}/questions/${questionId}`, { method: 'DELETE' });
}

// ============================================================================
// Ballot
// ============================================================================

/** Check the current user's eligibility to vote. */
export async function getEligibility(voteId: string): Promise<VoteEligibility> {
  return fetchApi<VoteEligibility>(`${API_BASE}/${voteId}/eligibility`);
}

/** Cast a ballot for a unit. */
export async function castVote(voteId: string, data: CastVoteRequest): Promise<VoteReceipt> {
  return fetchApi<VoteReceipt>(`${API_BASE}/${voteId}/cast`, {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

/** Get the current user's submitted response for a unit (null if none). */
export async function getMyResponse(voteId: string, unitId: string): Promise<VoteResponse | null> {
  const queryString = buildQueryString({ unit_id: unitId });
  return fetchApi<VoteResponse | null>(`${API_BASE}/${voteId}/my-response${queryString}`);
}

// ============================================================================
// Results, report & audit
// ============================================================================

/** Get vote results (cached when closed, live tally when active). */
export async function getResults(voteId: string): Promise<VoteResults> {
  return fetchApi<VoteResults>(`${API_BASE}/${voteId}/results`);
}

/** Get the immutable audit log (admin). */
export async function getAuditLog(voteId: string): Promise<VoteAuditLog[]> {
  return fetchApi<VoteAuditLog[]>(`${API_BASE}/${voteId}/audit`);
}

// ============================================================================
// Comments
// ============================================================================

/** List top-level (non-hidden) comments. */
export async function listComments(voteId: string): Promise<VoteCommentWithUser[]> {
  return fetchApi<VoteCommentWithUser[]>(`${API_BASE}/${voteId}/comments`);
}

/** List replies to a comment. */
export async function listCommentReplies(
  voteId: string,
  commentId: string
): Promise<VoteCommentWithUser[]> {
  return fetchApi<VoteCommentWithUser[]>(`${API_BASE}/${voteId}/comments/${commentId}/replies`);
}

/** Add a comment (optionally threaded via `parent_id`). */
export async function addComment(voteId: string, data: AddCommentRequest): Promise<VoteComment> {
  return fetchApi<VoteComment>(`${API_BASE}/${voteId}/comments`, {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

/** Hide/moderate a comment (moderator). */
export async function hideComment(
  voteId: string,
  commentId: string,
  data: HideCommentRequest
): Promise<VoteComment> {
  return fetchApi<VoteComment>(`${API_BASE}/${voteId}/comments/${commentId}/hide`, {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

// ============================================================================
// Building-scoped
// ============================================================================

/** List active votes for a building. */
export async function listActiveVotesForBuilding(buildingId: string): Promise<VoteSummary[]> {
  return fetchApi<VoteSummary[]>(`${API_BASE}/building/${buildingId}/active`);
}
