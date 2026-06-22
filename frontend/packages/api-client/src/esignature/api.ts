/**
 * E-signature API client (Epic 84, Story 84.2).
 *
 * Endpoint: POST /api/v1/documents/{documentId}/signature-requests
 *           GET  /api/v1/documents/{documentId}/signature-requests
 */

import { getToken } from '../auth';
import type {
  CancelSignatureRequestBody,
  CancelSignatureRequestResponse,
  CreateSignatureRequestBody,
  CreateSignatureRequestResponse,
  ListSignatureRequestsResponse,
  SendReminderBody,
  SendReminderResponse,
  SignatureRequestResponse,
} from './types';

const DOCUMENTS_BASE = '/api/v1/documents';
const SIGNATURE_REQUESTS_BASE = '/api/v1/signature-requests';

function getAuthHeaders(): HeadersInit {
  const token = getToken();
  return token ? { Authorization: `Bearer ${token}` } : {};
}

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
    throw new Error(error.message || `HTTP ${response.status}`);
  }

  return response.json();
}

/**
 * Create a signature request for a document.
 * POST /api/v1/documents/{documentId}/signature-requests
 */
export async function createSignatureRequest(
  documentId: string,
  body: CreateSignatureRequestBody
): Promise<CreateSignatureRequestResponse> {
  return apiRequest<CreateSignatureRequestResponse>(
    `${DOCUMENTS_BASE}/${documentId}/signature-requests`,
    {
      method: 'POST',
      body: JSON.stringify(body),
    }
  );
}

/**
 * List signature requests for a document.
 * GET /api/v1/documents/{documentId}/signature-requests
 */
export async function listSignatureRequests(
  documentId: string
): Promise<ListSignatureRequestsResponse> {
  return apiRequest<ListSignatureRequestsResponse>(
    `${DOCUMENTS_BASE}/${documentId}/signature-requests`
  );
}

/**
 * Get a signature request by ID.
 * GET /api/v1/signature-requests/{id}
 */
export async function getSignatureRequest(id: string): Promise<SignatureRequestResponse> {
  return apiRequest<SignatureRequestResponse>(`${SIGNATURE_REQUESTS_BASE}/${id}`);
}

/**
 * Send reminders to a signature request's pending signers.
 * POST /api/v1/signature-requests/{id}/remind
 */
export async function sendSignatureReminder(
  id: string,
  body: SendReminderBody = {}
): Promise<SendReminderResponse> {
  return apiRequest<SendReminderResponse>(`${SIGNATURE_REQUESTS_BASE}/${id}/remind`, {
    method: 'POST',
    body: JSON.stringify(body),
  });
}

/**
 * Cancel an open signature request.
 * POST /api/v1/signature-requests/{id}/cancel
 */
export async function cancelSignatureRequest(
  id: string,
  body: CancelSignatureRequestBody = {}
): Promise<CancelSignatureRequestResponse> {
  return apiRequest<CancelSignatureRequestResponse>(`${SIGNATURE_REQUESTS_BASE}/${id}/cancel`, {
    method: 'POST',
    body: JSON.stringify(body),
  });
}
