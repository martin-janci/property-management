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
