/**
 * E-signature API client (Epic 84, Story 84.2).
 *
 * Endpoint: POST /api/v1/documents/{documentId}/signature-requests
 *           GET  /api/v1/documents/{documentId}/signature-requests
 */

import type {
  CreateSignatureRequestBody,
  CreateSignatureRequestResponse,
  ListSignatureRequestsResponse,
  SignatureRequestResponse,
} from './types';

const DOCUMENTS_BASE = '/api/v1/documents';
const SIGNATURE_REQUESTS_BASE = '/api/v1/signature-requests';

async function fetchApi<T>(url: string, options: RequestInit = {}): Promise<T> {
  const response = await fetch(url, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options.headers,
    },
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({ message: 'Request failed' }));
    throw new Error((error as { message?: string }).message || `HTTP ${response.status}`);
  }

  return response.json() as Promise<T>;
}

/**
 * Create a signature request for a document.
 * POST /api/v1/documents/{documentId}/signature-requests
 */
export async function createSignatureRequest(
  documentId: string,
  body: CreateSignatureRequestBody
): Promise<CreateSignatureRequestResponse> {
  return fetchApi<CreateSignatureRequestResponse>(
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
  return fetchApi<ListSignatureRequestsResponse>(
    `${DOCUMENTS_BASE}/${documentId}/signature-requests`
  );
}

/**
 * Get a signature request by ID.
 * GET /api/v1/signature-requests/{id}
 */
export async function getSignatureRequest(id: string): Promise<SignatureRequestResponse> {
  return fetchApi<SignatureRequestResponse>(`${SIGNATURE_REQUESTS_BASE}/${id}`);
}
