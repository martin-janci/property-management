/**
 * E-signature types (Epic 84, Story 84.2).
 * Mirrors backend db::models::signature_request.
 */

export type SignatureRequestStatus =
  | 'pending'
  | 'in_progress'
  | 'completed'
  | 'declined'
  | 'expired'
  | 'cancelled';

export type SignerStatus = 'pending' | 'sent' | 'viewed' | 'signed' | 'declined';

export interface Signer {
  email: string;
  name: string;
  order: number;
  status: SignerStatus;
  signed_at?: string;
  declined_at?: string;
  declined_reason?: string;
  provider_signer_id?: string;
}

export interface SignatureRequest {
  id: string;
  document_id: string;
  organization_id: string;
  status: SignatureRequestStatus;
  subject?: string;
  message?: string;
  signers: Signer[];
  provider?: string;
  provider_request_id?: string;
  signed_document_id?: string;
  created_by: string;
  expires_at?: string;
  completed_at?: string;
  created_at: string;
  updated_at: string;
}

export interface SignerCounts {
  total: number;
  pending: number;
  sent: number;
  viewed: number;
  signed: number;
  declined: number;
}

export interface CreateSigner {
  email: string;
  name: string;
  order?: number;
}

export interface CreateSignatureRequestBody {
  signers: CreateSigner[];
  subject?: string;
  message?: string;
  provider?: string;
  expires_in_days?: number;
}

export interface CreateSignatureRequestResponse {
  signature_request: SignatureRequest;
  message: string;
}

export interface SignatureRequestResponse {
  signature_request: SignatureRequest;
  signer_counts: SignerCounts;
}

export interface ListSignatureRequestsResponse {
  signature_requests: SignatureRequest[];
  total: number;
}

export interface SendReminderBody {
  /** Specific signer emails to remind; empty/omitted reminds all pending signers. */
  signer_emails?: string[];
  message?: string;
}

export interface SendReminderResponse {
  reminders_sent: number;
  message: string;
}

export interface CancelSignatureRequestBody {
  reason?: string;
}

export interface CancelSignatureRequestResponse {
  signature_request: SignatureRequest;
  message: string;
}

// ---------------------------------------------------------------------------
// Signer-facing /sign consumer endpoint (Epic 84.2).
// Public: authority is the HMAC token in the `?token=` query string, so these
// carry NO auth header. Mirrors backend `SignRenderContext` /
// `SubmitSignatureRequest` / `SubmitSignatureResponse` in
// `routes/signatures.rs` (snake_case JSON, matching the types above).
// ---------------------------------------------------------------------------

/** Render context returned by `GET /api/v1/signatures/sign?token=…`. */
export interface SignRenderContext {
  /** The signature request (envelope) id. */
  request_id: string;
  /** The document being signed. */
  document_id: string;
  /** Human-facing subject/title of the request. */
  subject?: string;
  /** Optional message the requester attached for signers. */
  message?: string;
  /** The signer's display name (from the request roster). */
  signer_name: string;
  /** The signer's email (echoed from the verified token). */
  signer_email: string;
  /** The signer's current status (e.g. `pending`, `viewed`). */
  signer_status: string;
}

/** Body for `POST /api/v1/signatures/sign?token=…`. A bare `{}` is accepted. */
export interface SubmitSignatureBody {
  /** The full name the signer typed to adopt their signature (evidence). */
  typed_name?: string;
}

/** Response from `POST /api/v1/signatures/sign?token=…`. */
export interface SubmitSignatureResponse {
  /** Request status AFTER recording this signature (`in_progress` / `completed`). */
  status: string;
  /** Human-facing confirmation message. */
  message: string;
}
