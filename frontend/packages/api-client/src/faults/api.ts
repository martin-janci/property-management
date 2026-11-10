/**
 * Fault API client functions (Epic 4: Fault Reporting & Resolution).
 */

import { createUploadUrl, deleteUploadedFile, uploadFileToPresignedUrl } from '../documents/api';
import { authenticatedFetch } from '../lib/fetch';
import type {
  AddCommentRequest,
  AddFaultAttachmentRequest,
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
  MyFaultsQuery,
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

/**
 * List the faults reported by the currently authenticated user.
 *
 * Hits `GET /api/v1/faults/my`, where the reporter filter is applied
 * server-side (from the request principal) *before* `limit`. This is the
 * correct primitive for "my faults" surfaces: it returns the user's own
 * most-recent faults rather than filtering a capped, org-wide list on the
 * client (which can hide a resident's faults behind newer third-party ones).
 */
export async function listMyFaults(query: MyFaultsQuery = {}): Promise<FaultListResponse> {
  const queryString = buildQueryString({
    limit: query.limit,
    offset: query.offset,
  });
  return fetchApi<FaultListResponse>(`${API_BASE}/my${queryString}`);
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

/**
 * Withdraw a fault report that has not been triaged yet (reporter only).
 * The server closes it with a "withdrawn by reporter" timeline note.
 */
export async function withdrawFault(id: string): Promise<void> {
  await fetchApi<unknown>(`${API_BASE}/${id}/withdraw`, { method: 'POST' });
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

/**
 * Upload a file and attach it to a fault (gap audit R6 P1-10).
 *
 * Same presigned flow as documents and the mobile app:
 *   1. `POST /api/v1/documents/upload-url` → presigned PUT URL + org-scoped key;
 *   2. PUT the bytes straight to storage;
 *   3. `POST /api/v1/faults/{id}/attachments` with the key as `storage_url`.
 * When step 3 fails the uploaded object is deleted again (best effort).
 */
export async function addAttachment(faultId: string, file: File): Promise<FaultAttachment> {
  const presigned = await createUploadUrl({
    file_name: file.name,
    mime_type: file.type,
    size_bytes: file.size,
  });
  await uploadFileToPresignedUrl(presigned.url, file, presigned.content_type);

  const body: AddFaultAttachmentRequest = {
    filename: presigned.file_key.split('/').pop() ?? file.name,
    original_filename: file.name,
    content_type: presigned.content_type,
    size_bytes: file.size,
    storage_url: presigned.file_key,
  };
  try {
    return await fetchApi<FaultAttachment>(`${API_BASE}/${faultId}/attachments`, {
      method: 'POST',
      body: JSON.stringify(body),
    });
  } catch (err) {
    await deleteUploadedFile(presigned.file_key).catch(() => undefined);
    throw err;
  }
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
