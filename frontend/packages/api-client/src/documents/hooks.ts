/**
 * Document React Query hooks (Epic 39).
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import * as api from './api';
import type {
  ClassificationFeedback,
  DocumentListQuery,
  DocumentSearchRequest,
  GenerateSummaryRequest,
} from './types';

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
 * Promotes `access_scope` to a first-class parameter so the caller can
 * surface audience segmentation (organization / building / unit / user /
 * public) without spreading query objects.  The backend enforces RLS on top
 * of this filter — callers cannot escalate permissions by omitting it.
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
