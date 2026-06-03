/**
 * Document React Query hooks (Epic 39).
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { getToken } from '../auth';
import { DocumentsService } from '../generated';
import * as api from './api';
import type {
  ClassificationFeedback,
  CreateShareRequest,
  DocumentListQuery,
  DocumentSearchRequest,
  GenerateSummaryRequest,
  VersionHistoryResponse,
} from './types';

export type { CreateFolderRequest, UpdateFolderRequest } from './api';

// Query keys
export const documentKeys = {
  all: ['documents'] as const,
  lists: () => [...documentKeys.all, 'list'] as const,
  list: (query?: DocumentListQuery) => [...documentKeys.lists(), query] as const,
  details: () => [...documentKeys.all, 'detail'] as const,
  detail: (id: string) => [...documentKeys.details(), id] as const,
  search: (query: DocumentSearchRequest) => [...documentKeys.all, 'search', query] as const,
  classification: (id: string) => [...documentKeys.all, 'classification', id] as const,
  classificationHistory: (id: string) =>
    [...documentKeys.all, 'classification-history', id] as const,
  folders: () => [...documentKeys.all, 'folders'] as const,
  folderTree: (buildingId?: string) => [...documentKeys.folders(), 'tree', buildingId] as const,
  stats: () => [...documentKeys.all, 'stats'] as const,
  shares: (documentId: string) => [...documentKeys.all, documentId, 'shares'] as const,
  versions: (documentId: string) => [...documentKeys.all, documentId, 'versions'] as const,
};

// List documents
export function useDocuments(query?: DocumentListQuery) {
  return useQuery({
    queryKey: documentKeys.list(query),
    queryFn: () => api.listDocuments(query),
  });
}

/**
 * Permission-based document list (7a-3 RLS-aware audience filter).
 *
 * Alias for `useDocuments` that makes `access_scope` a first-class concern.
 * The backend enforces RLS on top of this filter — callers cannot escalate
 * access by omitting it.
 */
export function useDocumentList(query?: DocumentListQuery) {
  return useDocuments(query);
}

// Get single document
export function useDocument(id: string) {
  return useQuery({
    queryKey: documentKeys.detail(id),
    queryFn: () => api.getDocument(id),
    enabled: !!id,
  });
}

// Search documents (Story 39.1)
export function useDocumentSearch(request: DocumentSearchRequest) {
  return useQuery({
    queryKey: documentKeys.search(request),
    queryFn: () => api.searchDocuments(request),
    enabled: !!request.query && request.query.length >= 2,
    staleTime: 30 * 1000, // 30 seconds
  });
}

// Get classification (Story 39.3)
export function useDocumentClassification(id: string) {
  return useQuery({
    queryKey: documentKeys.classification(id),
    queryFn: () => api.getClassification(id),
    enabled: !!id,
  });
}

// Get classification history (Story 39.3)
export function useClassificationHistory(id: string) {
  return useQuery({
    queryKey: documentKeys.classificationHistory(id),
    queryFn: () => api.getClassificationHistory(id),
    enabled: !!id,
  });
}

// Get folder tree
export function useFolderTree(buildingId?: string) {
  return useQuery({
    queryKey: documentKeys.folderTree(buildingId),
    queryFn: () => api.getFolderTree(buildingId),
  });
}

// Get intelligence stats
export function useIntelligenceStats() {
  return useQuery({
    queryKey: documentKeys.stats(),
    queryFn: () => api.getIntelligenceStats(),
    staleTime: 5 * 60 * 1000, // 5 minutes
  });
}

// Reprocess OCR (Story 39.2)
export function useReprocessOcr() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: string) => api.reprocessOcr(id),
    onSuccess: (_, id) => {
      queryClient.invalidateQueries({ queryKey: documentKeys.detail(id) });
    },
  });
}

// Submit classification feedback (Story 39.3)
export function useSubmitClassificationFeedback() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, feedback }: { id: string; feedback: ClassificationFeedback }) =>
      api.submitClassificationFeedback(id, feedback),
    onSuccess: (_, { id }) => {
      queryClient.invalidateQueries({ queryKey: documentKeys.classification(id) });
      queryClient.invalidateQueries({
        queryKey: documentKeys.classificationHistory(id),
      });
      queryClient.invalidateQueries({ queryKey: documentKeys.detail(id) });
    },
  });
}

// Request summarization (Story 39.4)
export function useRequestSummarization() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, options }: { id: string; options?: GenerateSummaryRequest }) =>
      api.requestSummarization(id, options),
    onSuccess: (_, { id }) => {
      // The actual summary will be available after async processing
      // Invalidate after a delay to pick up the result
      setTimeout(() => {
        queryClient.invalidateQueries({ queryKey: documentKeys.detail(id) });
      }, 5000);
    },
  });
}

// Get download URL
export function useDownloadUrl(id: string) {
  return useQuery({
    queryKey: [...documentKeys.detail(id), 'download'] as const,
    queryFn: () => api.getDownloadUrl(id),
    enabled: !!id,
    staleTime: 4 * 60 * 1000, // URLs expire in 5 min, refetch at 4 min
  });
}

// Get preview URL
export function usePreviewUrl(id: string) {
  return useQuery({
    queryKey: [...documentKeys.detail(id), 'preview'] as const,
    queryFn: () => api.getPreviewUrl(id),
    enabled: !!id,
    staleTime: 4 * 60 * 1000,
  });
}

// Upload document (Story 39.2)
export function useUploadDocument() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: api.UploadDocumentParams) => api.uploadDocument(params),
    onSuccess: () => {
      // Invalidate document lists to show the new document
      queryClient.invalidateQueries({ queryKey: documentKeys.lists() });
    },
  });
}

// Create folder (gap-7a-2)
export function useCreateFolder() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (data: api.CreateFolderRequest) => api.createFolder(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: documentKeys.folders() });
    },
  });
}

// Update folder (gap-7a-2)
export function useUpdateFolder() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, data }: { id: string; data: api.UpdateFolderRequest }) =>
      api.updateFolder(id, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: documentKeys.folders() });
    },
  });
}

// Delete folder (gap-7a-2)
export function useDeleteFolder() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, cascade = false }: { id: string; cascade?: boolean }) =>
      api.deleteFolder(id, cascade),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: documentKeys.folders() });
      queryClient.invalidateQueries({ queryKey: documentKeys.lists() });
    },
  });
}

// Move document to folder (gap-7a-2)
export function useMoveDocument() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ documentId, folderId }: { documentId: string; folderId: string | null }) =>
      api.moveDocument(documentId, folderId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: documentKeys.lists() });
      queryClient.invalidateQueries({ queryKey: documentKeys.folders() });
    },
  });
}

// --- Document Sharing hooks (Story 7A.5) ---

export function useDocumentShares(documentId: string) {
  return useQuery({
    queryKey: documentKeys.shares(documentId),
    queryFn: () => api.listDocumentShares(documentId),
    enabled: !!documentId,
    staleTime: 30_000,
  });
}

export function useCreateDocumentShare(documentId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: CreateShareRequest) => api.createDocumentShare(documentId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: documentKeys.shares(documentId) });
    },
  });
}

export function useRevokeDocumentShare(documentId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (shareId: string) => api.revokeDocumentShare(documentId, shareId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: documentKeys.shares(documentId) });
    },
  });
}

// --- Document Versions hook (#974, list-only) ---

/**
 * List the version history for a document (`GET /api/v1/documents/{id}/versions`).
 *
 * Wraps the generated `DocumentsService.documentsApiListVersions`. The Bearer
 * token is sourced from the registered token provider (same source the rest of
 * the client uses); the tenant is derived server-side from the JWT, so an empty
 * `xTenantId` is passed for the generated header parameter.
 *
 * The generated OpenAPI client models this endpoint as a bare camelCase array
 * (`Array<Documents_Document>`), but the real backend returns the
 * `VersionHistoryResponse` envelope (`{ history: { versions: [...] } }`,
 * snake_case). The transport just parses the JSON body, so we re-type the
 * result to the real backend shape via `unknown` — consumers then read the
 * actual `history.versions` payload.
 *
 * LIST-ONLY: there is no generated restore method, so restore is not exposed.
 */
export function useDocumentVersions(documentId: string) {
  return useQuery({
    queryKey: documentKeys.versions(documentId),
    queryFn: () =>
      DocumentsService.documentsApiListVersions({
        authorization: `Bearer ${getToken() ?? ''}`,
        xTenantId: '',
        id: documentId,
      }) as unknown as Promise<VersionHistoryResponse>,
    enabled: !!documentId,
    staleTime: 30_000,
  });
}
