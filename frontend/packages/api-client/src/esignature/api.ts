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
  SignRenderContext,
  SubmitSignatureBody,
  SubmitSignatureResponse,
} from './types';

const DOCUMENTS_BASE = '/api/v1/documents';
const SIGNATURE_REQUESTS_BASE = '/api/v1/signature-requests';
const SIGN_BASE = '/api/v1/signatures/sign';

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

// ---------------------------------------------------------------------------
// Signer-facing /sign consumer endpoints (Epic 84.2).
// ---------------------------------------------------------------------------

/**
 * Error thrown by the signer-facing `/sign` endpoints. Carries the backend
 * error `code` and HTTP `status` so the signing page can branch its UI on
 * `SIGNING_LINK_EXPIRED` / `INVALID_SIGNING_LINK` / `ALREADY_SIGNED` /
 * `LINK_ALREADY_USED` / `REQUEST_NOT_OPEN` instead of showing one generic
 * message. `apiRequest` above throws a bare `Error`; the signer page needs the
 * discriminant, hence this dedicated type + fetch wrapper.
 */
export class SignError extends Error {
  readonly code: string;
  readonly status: number;

  constructor(code: string, status: number, message: string) {
    super(message);
    this.name = 'SignError';
    this.code = code;
    this.status = status;
  }
}

/**
 * Fetch wrapper for the PUBLIC `/sign` endpoints. Unlike `apiRequest` it sends
 * NO `Authorization` header — the signer is typically not a platform user and
 * all authority comes from the HMAC token in the query string. It also does
 * NOT go through the shared axios client, so a `401 INVALID_SIGNING_LINK` never
 * triggers the app's 401→login redirect (the signer must see that state).
 */
async function signRequest<T>(url: string, options: RequestInit = {}): Promise<T> {
  const response = await fetch(url, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options.headers,
    },
  });

  if (!response.ok) {
    const body = (await response.json().catch(() => ({}))) as {
      error?: string;
      message?: string;
    };
    throw new SignError(
      body.error ?? `HTTP_${response.status}`,
      response.status,
      body.message ?? `HTTP ${response.status}`
    );
  }

  return response.json();
}

/**
 * Fetch the render context for a signing link (read-only; does not consume the
 * single-use nonce).
 * GET /api/v1/signatures/sign?token=…
 */
export async function getSignContext(token: string): Promise<SignRenderContext> {
  return signRequest<SignRenderContext>(`${SIGN_BASE}?token=${encodeURIComponent(token)}`);
}

/**
 * Record the signature for a signing link. Single-use: the backend consumes
 * the nonce atomically, so a replay/double-submit is rejected with 409.
 * POST /api/v1/signatures/sign?token=…
 */
export async function submitSignature(
  token: string,
  body: SubmitSignatureBody = {}
): Promise<SubmitSignatureResponse> {
  return signRequest<SubmitSignatureResponse>(`${SIGN_BASE}?token=${encodeURIComponent(token)}`, {
    method: 'POST',
    body: JSON.stringify(body),
  });
}
