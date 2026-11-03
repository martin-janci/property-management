/**
 * Fault API client functions (Epic 4: Fault Reporting & Resolution).
 */

import { authenticatedFetch } from '../lib/fetch';
import type {
  AddCommentRequest,
  AddWorkNoteRequest,
  AiSuggestionResponse,
  CreateFaultRequest,
  CreateFaultResponse,
  FaultAttachment,
  FaultComment,
  FaultDetailResponse,
  FaultListQuery,
  FaultListResponse,
  FaultStatistics,
  FaultSummary,
  FaultTimelineEntry,
  ResolveFaultRequest,
  TriageFaultRequest,
  UpdateFaultRequest,
  WorkNote,
} from './types';

const API_BASE = '/api/v1/faults';

/** Helper to build query string from params */
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

/** Generic fetch wrapper with error handling */
async function fetchApi<T>(url: string, options?: RequestInit): Promise<T> {
  const response = await authenticatedFetch(url, {
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

  return response.json();
}

// ============================================================================
// Fault CRUD Operations
// ============================================================================

/** List faults with optional filters */
export async function listFaults(query: FaultListQuery = {}): Promise<FaultListResponse> {
  const queryString = buildQueryString({
    building_id: query.building_id,
    status: query.status,
    category: query.category,
    priority: query.priority,
    assigned_to: query.assigned_to,
    search: query.search,
    page: query.page,
    limit: query.limit,
  });
  return fetchApi<FaultListResponse>(`${API_BASE}${queryString}`);
}

/** Get fault by ID with full details */
export async function getFault(id: string): Promise<FaultDetailResponse> {
  return fetchApi<FaultDetailResponse>(`${API_BASE}/${id}`);
}

/** Create a new fault */
export async function createFault(data: CreateFaultRequest): Promise<CreateFaultResponse> {
  return fetchApi<CreateFaultResponse>(API_BASE, {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

/** Update an existing fault */
export async function updateFault(id: string, data: UpdateFaultRequest): Promise<FaultSummary> {
  return fetchApi<FaultSummary>(`${API_BASE}/${id}`, {
    method: 'PUT',
    body: JSON.stringify(data),
  });
}

/** Delete a fault */
export async function deleteFault(id: string): Promise<void> {
  await fetchApi<void>(`${API_BASE}/${id}`, {
    method: 'DELETE',
  });
}

// ============================================================================
// Fault Status Operations
// ============================================================================

/** Triage a fault (set priority/category/assignment) */
export async function triageFault(id: string, data: TriageFaultRequest): Promise<FaultSummary> {
  return fetchApi<FaultSummary>(`${API_BASE}/${id}/triage`, {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

/** Assign a fault to a user */
export async function assignFault(id: string, assignedTo: string): Promise<FaultSummary> {
  return fetchApi<FaultSummary>(`${API_BASE}/${id}/assign`, {
    method: 'POST',
    body: JSON.stringify({ assigned_to: assignedTo }),
  });
}

/** Resolve a fault */
export async function resolveFault(id: string, data: ResolveFaultRequest): Promise<FaultSummary> {
  return fetchApi<FaultSummary>(`${API_BASE}/${id}/resolve`, {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

/** Confirm fault resolution */
export async function confirmFault(id: string, rating?: number): Promise<FaultSummary> {
  return fetchApi<FaultSummary>(`${API_BASE}/${id}/confirm`, {
    method: 'POST',
    body: JSON.stringify({ rating }),
  });
}

/** Reopen a fault */
export async function reopenFault(id: string, reason: string): Promise<FaultSummary> {
  return fetchApi<FaultSummary>(`${API_BASE}/${id}/reopen`, {
    method: 'POST',
    body: JSON.stringify({ reason }),
  });
}

// ============================================================================
// Comments & Notes
// ============================================================================

/** List comments on a fault */
export async function listFaultComments(faultId: string): Promise<FaultComment[]> {
  const response = await fetchApi<{ comments: FaultComment[] }>(`${API_BASE}/${faultId}/comments`);
  return response.comments;
}

/** Add a comment to a fault */
export async function addComment(faultId: string, data: AddCommentRequest): Promise<FaultComment> {
  return fetchApi<FaultComment>(`${API_BASE}/${faultId}/comments`, {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

/** Add a work note to a fault */
export async function addWorkNote(faultId: string, data: AddWorkNoteRequest): Promise<WorkNote> {
  return fetchApi<WorkNote>(`${API_BASE}/${faultId}/work-notes`, {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

// ============================================================================
// Attachments
// ============================================================================

/** List attachments on a fault */
export async function listAttachments(faultId: string): Promise<FaultAttachment[]> {
  const response = await fetchApi<{ attachments: FaultAttachment[] }>(
    `${API_BASE}/${faultId}/attachments`
  );
  return response.attachments;
}

/** Add an attachment to a fault */
export async function addAttachment(faultId: string, file: File): Promise<FaultAttachment> {
  const formData = new FormData();
  formData.append('file', file);

  const response = await authenticatedFetch(`${API_BASE}/${faultId}/attachments`, {
    method: 'POST',
    body: formData,
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({ message: 'Upload failed' }));
    throw new Error(error.message || `HTTP ${response.status}`);
  }

  return response.json();
}

/** Delete an attachment from a fault */
export async function deleteAttachment(faultId: string, attachmentId: string): Promise<void> {
  await fetchApi<void>(`${API_BASE}/${faultId}/attachments/${attachmentId}`, {
    method: 'DELETE',
  });
}

// ============================================================================
// Timeline
// ============================================================================

/** Get fault timeline */
export async function getFaultTimeline(faultId: string): Promise<FaultTimelineEntry[]> {
  const response = await fetchApi<{ entries: FaultTimelineEntry[] }>(
    `${API_BASE}/${faultId}/timeline`
  );
  return response.entries;
}

// ============================================================================
// AI Suggestions (Epic 126)
// ============================================================================

/**
 * Get AI-suggested category and priority for a fault.
 * Call this after creating a fault to get recommendations.
 */
export async function getAiSuggestion(faultId: string): Promise<AiSuggestionResponse> {
  return fetchApi<AiSuggestionResponse>(`${API_BASE}/${faultId}/suggest`, {
    method: 'POST',
  });
}

/**
 * Accept AI suggestion for a fault.
 * Updates the fault with the suggested category and priority.
 */
export async function acceptAiSuggestion(
  faultId: string,
  suggestion: { category: string; priority?: string }
): Promise<FaultSummary> {
  return fetchApi<FaultSummary>(`${API_BASE}/${faultId}`, {
    method: 'PUT',
    body: JSON.stringify({
      category: suggestion.category,
      // priority will be set via triage if provided
    }),
  });
}

// ============================================================================
// Statistics
// ============================================================================

/** Get fault statistics */
export async function getFaultStatistics(
  buildingId?: string,
  dateFrom?: string,
  dateTo?: string
): Promise<FaultStatistics> {
  const query = buildQueryString({ building_id: buildingId, date_from: dateFrom, date_to: dateTo });
  const response = await fetchApi<{ statistics: FaultStatistics }>(
    `${API_BASE}/statistics${query}`
  );
  return response.statistics;
}
