/**
 * Document share types and API helpers (Story 7A.5).
 *
 * Mirrors the backend models in:
 *   backend/crates/db/src/models/document.rs  (DocumentShare, ShareWithDocument)
 *   backend/servers/api-server/src/routes/documents.rs  (CreateShareRequest / CreateShareResponse)
 */

import { useQueryClient } from '@tanstack/react-query';
import { apiRequest, useApiMutation, useApiQuery } from '../../hooks/useApi';

// ─── Share type constants (mirrors backend share_type mod) ──────────────────

export const SHARE_TYPE = {
  USER: 'user',
  ROLE: 'role',
  BUILDING: 'building',
  LINK: 'link',
} as const;

export type ShareType = (typeof SHARE_TYPE)[keyof typeof SHARE_TYPE];

// ─── API shapes ──────────────────────────────────────────────────────────────

export interface DocumentShare {
  id: string;
  document_id: string;
  share_type: ShareType;
  target_id?: string | null;
  target_role?: string | null;
  shared_by: string;
  share_token?: string | null;
  expires_at?: string | null;
  revoked_at?: string | null;
  created_at: string;
}

export interface ShareWithDocument {
  id: string;
  document_id: string;
  share_type: ShareType;
  target_id?: string | null;
  target_role?: string | null;
  shared_by: string;
  share_token?: string | null;
  expires_at?: string | null;
  revoked_at?: string | null;
  created_at: string;
  document_title: string;
  shared_by_name: string;
}

export interface ShareListResponse {
  shares: ShareWithDocument[];
}

export interface CreateShareRequest {
  share_type: ShareType;
  target_id?: string;
  target_role?: string;
  password?: string;
  expires_at?: string;
}

export interface CreateShareResponse {
  id: string;
  share_token?: string | null;
  share_url?: string | null;
  message: string;
}

// ─── Query keys ──────────────────────────────────────────────────────────────

export const shareKeys = {
  list: (documentId: string) => ['documents', documentId, 'shares'] as const,
};

// ─── Hooks ───────────────────────────────────────────────────────────────────

/** List all (non-revoked) shares for a document. */
export function useDocumentShares(documentId: string) {
  return useApiQuery<ShareListResponse>(
    shareKeys.list(documentId),
    `/api/v1/documents/${documentId}/shares`,
    { enabled: !!documentId, staleTime: 30_000 }
  );
}

/** Create a share for a document. */
export function useCreateShare(documentId: string) {
  return useApiMutation<CreateShareResponse, CreateShareRequest>(
    `/api/v1/documents/${documentId}/shares`,
    'POST'
  );
}

/** Revoke a share (DELETE returns 204). */
export function useRevokeShare(documentId: string) {
  const queryClient = useQueryClient();

  return {
    revokeAsync: async (shareId: string): Promise<void> => {
      await apiRequest<void>(`/api/v1/documents/${documentId}/shares/${shareId}`, {
        method: 'DELETE',
      });
      await queryClient.invalidateQueries({ queryKey: shareKeys.list(documentId) });
    },
  };
}
