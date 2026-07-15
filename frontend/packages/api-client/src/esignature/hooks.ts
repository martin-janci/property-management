/**
 * E-signature React Query hooks (Epic 84, Story 84.2).
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import * as api from './api';
import type {
  CancelSignatureRequestBody,
  CreateSignatureRequestBody,
  SendReminderBody,
  SubmitSignatureBody,
} from './types';

// Query keys
export const esignatureKeys = {
  all: ['esignature'] as const,
  requestsForDocument: (documentId: string) =>
    [...esignatureKeys.all, 'document', documentId] as const,
  request: (id: string) => [...esignatureKeys.all, 'request', id] as const,
  signContext: (token: string) => [...esignatureKeys.all, 'sign-context', token] as const,
};

/**
 * List signature requests for a document.
 */
export function useSignatureRequests(documentId: string | undefined) {
  return useQuery({
    queryKey: esignatureKeys.requestsForDocument(documentId ?? ''),
    queryFn: () => api.listSignatureRequests(documentId!),
    enabled: !!documentId,
    staleTime: 30_000,
  });
}

/**
 * Get a single signature request by ID.
 */
export function useSignatureRequest(id: string | undefined) {
  return useQuery({
    queryKey: esignatureKeys.request(id ?? ''),
    queryFn: () => api.getSignatureRequest(id!),
    enabled: !!id,
    staleTime: 30_000,
  });
}

/**
 * Create a signature request for a document.
 * Invalidates the document's signature-request list on success.
 */
export function useCreateSignatureRequest(documentId: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (body: CreateSignatureRequestBody) => api.createSignatureRequest(documentId, body),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: esignatureKeys.requestsForDocument(documentId),
      });
    },
  });
}

/**
 * Send reminders to a signature request's pending signers.
 * Invalidates the request and (when known) the owning document's list.
 */
export function useSendSignatureReminder(documentId?: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, body }: { id: string; body?: SendReminderBody }) =>
      api.sendSignatureReminder(id, body),
    onSuccess: (_data, { id }) => {
      queryClient.invalidateQueries({ queryKey: esignatureKeys.request(id) });
      if (documentId) {
        queryClient.invalidateQueries({
          queryKey: esignatureKeys.requestsForDocument(documentId),
        });
      }
    },
  });
}

/**
 * Cancel an open signature request.
 * Invalidates the request and (when known) the owning document's list.
 */
export function useCancelSignatureRequest(documentId?: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, body }: { id: string; body?: CancelSignatureRequestBody }) =>
      api.cancelSignatureRequest(id, body),
    onSuccess: (_data, { id }) => {
      queryClient.invalidateQueries({ queryKey: esignatureKeys.request(id) });
      if (documentId) {
        queryClient.invalidateQueries({
          queryKey: esignatureKeys.requestsForDocument(documentId),
        });
      }
    },
  });
}

/**
 * Signer-facing render context for a signing link (Epic 84.2).
 * Read-only — does NOT consume the single-use nonce. Token errors (expired /
 * invalid / already-signed) are surfaced as `SignError`; we disable retries so
 * the signer sees the terminal state immediately rather than after backoff.
 */
export function useSignContext(token: string | undefined) {
  return useQuery({
    queryKey: esignatureKeys.signContext(token ?? ''),
    queryFn: () => api.getSignContext(token!),
    enabled: !!token,
    retry: false,
    staleTime: 0,
    gcTime: 0,
  });
}

/**
 * Record the signature for a signing link (Epic 84.2).
 * The single-use nonce is consumed server-side, so this is not retried.
 */
export function useSubmitSignature(token: string) {
  return useMutation({
    mutationFn: (body: SubmitSignatureBody = {}) => api.submitSignature(token, body),
  });
}
