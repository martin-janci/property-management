/**
 * Disputes API Client
 *
 * API functions for Disputes (Epic 80).
 */

import { getToken } from '../auth';
import type {
  AddMediationNoteRequest,
  AssignMediatorRequest,
  CreateDisputeRequest,
  CreateDisputeResponse,
  Dispute,
  DisputeEvidence,
  DisputeListQuery,
  DisputeStatistics,
  DisputeWithDetails,
  EscalateDisputeRequest,
  MediationNote,
  MediationSession,
  PaginatedDisputes,
  ResolveDisputeRequest,
  ScheduleSessionRequest,
  TimelineEvent,
  TimelineQuery,
  UpdateDisputeStatusRequest,
  UpdateSessionRequest,
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

/**
 * Get authorization header from the configured token provider.
 *
 * Uses the centralized token provider which should be configured
 * during app initialization to integrate with AuthContext.
 *
 * @see setTokenProvider for configuration
 */
function getAuthHeaders(): HeadersInit {
  const token = getToken();
  return token ? { Authorization: `Bearer ${token}` } : {};
}

// Helper for API requests
async function apiRequest<T>(url: string, options: RequestInit = {}): Promise<T> {
  const response = await fetch(url, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...getAuthHeaders(),
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

// Helper for multipart form data requests (file uploads)
async function apiFormDataRequest<T>(url: string, formData: FormData): Promise<T> {
  const response = await fetch(url, {
    method: 'POST',
    body: formData,
    headers: {
      ...getAuthHeaders(),
    },
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({}));
    throw new Error(error.message || `HTTP error ${response.status}`);
  }

  return response.json();
}

// ============================================
// Dispute Listing & Details (Story 80.1)
// ============================================

export async function listDisputes(
  organizationId: string,
  query?: DisputeListQuery
): Promise<PaginatedDisputes> {
  const qs = buildQueryString(query || {});
  return apiRequest<PaginatedDisputes>(`${API_BASE}/organizations/${organizationId}/list${qs}`);
}

export async function getDispute(id: string): Promise<DisputeWithDetails> {
  return apiRequest<DisputeWithDetails>(`${API_BASE}/${id}`);
}

export async function getDisputeStatistics(organizationId: string): Promise<DisputeStatistics> {
  return apiRequest<DisputeStatistics>(`${API_BASE}/organizations/${organizationId}/statistics`);
}

// ============================================
// Dispute Filing (Story 80.2)
// ============================================

export async function createDispute(
  organizationId: string,
  data: CreateDisputeRequest
): Promise<CreateDisputeResponse> {
  return apiRequest<CreateDisputeResponse>(`${API_BASE}/organizations/${organizationId}`, {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

export async function uploadEvidence(
  disputeId: string,
  file: File,
  description: string
): Promise<DisputeEvidence> {
  const formData = new FormData();
  formData.append('file', file);
  formData.append('description', description);
  return apiFormDataRequest<DisputeEvidence>(`${API_BASE}/${disputeId}/evidence`, formData);
}

export async function listEvidence(disputeId: string): Promise<DisputeEvidence[]> {
  return apiRequest<DisputeEvidence[]>(`${API_BASE}/${disputeId}/evidence`);
}

export async function deleteEvidence(disputeId: string, evidenceId: string): Promise<void> {
  return apiRequest<void>(`${API_BASE}/${disputeId}/evidence/${evidenceId}`, {
    method: 'DELETE',
  });
}

// ============================================
// Mediation & Resolution (Story 80.3)
// ============================================

export async function assignMediator(
  disputeId: string,
  data: AssignMediatorRequest
): Promise<Dispute> {
  return apiRequest<Dispute>(`${API_BASE}/${disputeId}/assign`, {
    method: 'PUT',
    body: JSON.stringify(data),
  });
}

export async function updateDisputeStatus(
  disputeId: string,
  data: UpdateDisputeStatusRequest
): Promise<Dispute> {
  return apiRequest<Dispute>(`${API_BASE}/${disputeId}/status`, {
    method: 'PUT',
    body: JSON.stringify(data),
  });
}

export async function resolveDispute(
  disputeId: string,
  data: ResolveDisputeRequest
): Promise<Dispute> {
  return apiRequest<Dispute>(`${API_BASE}/${disputeId}/resolve`, {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

export async function escalateDispute(
  disputeId: string,
  data: EscalateDisputeRequest
): Promise<Dispute> {
  return apiRequest<Dispute>(`${API_BASE}/${disputeId}/escalate`, {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

export async function addMediationNote(
  disputeId: string,
  data: AddMediationNoteRequest
): Promise<MediationNote> {
  return apiRequest<MediationNote>(`${API_BASE}/${disputeId}/notes`, {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

export async function listMediationNotes(disputeId: string): Promise<MediationNote[]> {
  return apiRequest<MediationNote[]>(`${API_BASE}/${disputeId}/notes`);
}

// ============================================
// Mediation Sessions (Story 80.3)
// ============================================

/** List all mediation sessions for a dispute. */
export async function listSessions(disputeId: string): Promise<MediationSession[]> {
  return apiRequest<MediationSession[]>(`${API_BASE}/${disputeId}/sessions`);
}

/** Schedule a new mediation session. */
export async function scheduleSession(
  disputeId: string,
  data: ScheduleSessionRequest
): Promise<MediationSession> {
  return apiRequest<MediationSession>(`${API_BASE}/${disputeId}/sessions`, {
    method: 'POST',
    body: JSON.stringify(data),
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
// Timeline (Story 80.3)
// ============================================

export async function getTimeline(
  disputeId: string,
  query?: TimelineQuery
): Promise<TimelineEvent[]> {
  const qs = buildQueryString(query || {});
  return apiRequest<TimelineEvent[]>(`${API_BASE}/${disputeId}/timeline${qs}`);
}
