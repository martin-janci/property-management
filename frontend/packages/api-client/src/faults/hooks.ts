/**
 * TanStack Query hooks for Fault API (Epic 4, Epic 126).
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import * as api from './api';
import type {
  AddCommentRequest,
  AddWorkNoteRequest,
  AiSuggestionResponse,
  CreateFaultRequest,
  FaultComment,
  FaultDetailResponse,
  FaultListQuery,
  FaultListResponse,
  FaultStatistics,
  ResolveFaultRequest,
  TriageFaultRequest,
  UpdateFaultRequest,
} from './types';

// ============================================================================
// Query Keys
// ============================================================================

export const faultKeys = {
  all: ['faults'] as const,
  lists: () => [...faultKeys.all, 'list'] as const,
  list: (query: FaultListQuery) => [...faultKeys.lists(), query] as const,
  details: () => [...faultKeys.all, 'detail'] as const,
  detail: (id: string) => [...faultKeys.details(), id] as const,
  timeline: (id: string) => [...faultKeys.all, 'timeline', id] as const,
  attachments: (id: string) => [...faultKeys.all, 'attachments', id] as const,
  comments: (id: string) => [...faultKeys.all, 'comments', id] as const,
  suggestion: (id: string) => [...faultKeys.all, 'suggestion', id] as const,
  statistics: (buildingId?: string) => [...faultKeys.all, 'statistics', buildingId] as const,
};

// ============================================================================
// Query Hooks
// ============================================================================

/** List faults with optional filters */
export function useFaults(query: FaultListQuery = {}) {
  return useQuery<FaultListResponse>({
    queryKey: faultKeys.list(query),
    queryFn: () => api.listFaults(query),
  });
}

/** Get fault details by ID */
export function useFault(id: string) {
  return useQuery<FaultDetailResponse>({
    queryKey: faultKeys.detail(id),
    queryFn: () => api.getFault(id),
    enabled: !!id,
  });
}

/** Get fault comments */
export function useFaultComments(faultId: string) {
  return useQuery<FaultComment[]>({
    queryKey: faultKeys.comments(faultId),
    queryFn: () => api.listFaultComments(faultId),
    enabled: !!faultId,
  });
}

/** Get fault statistics */
export function useFaultStatistics(buildingId?: string) {
  return useQuery<FaultStatistics>({
    queryKey: faultKeys.statistics(buildingId),
    queryFn: () => api.getFaultStatistics(buildingId),
  });
}

/** Get AI suggestion for a fault (Epic 126) */
export function useAiSuggestion(faultId: string, enabled = true) {
  return useQuery<AiSuggestionResponse>({
    queryKey: faultKeys.suggestion(faultId),
    queryFn: () => api.getAiSuggestion(faultId),
    enabled: enabled && !!faultId,
    staleTime: 1000 * 60 * 5, // Cache for 5 minutes
  });
}

// ============================================================================
// Mutation Hooks
// ============================================================================

/** Create a new fault */
export function useCreateFault() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (data: CreateFaultRequest) => api.createFault(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: faultKeys.lists() });
    },
  });
}

/** Update a fault */
export function useUpdateFault(faultId: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (data: UpdateFaultRequest) => api.updateFault(faultId, data),
    onSuccess: (updatedFault) => {
      queryClient.setQueryData<FaultDetailResponse>(faultKeys.detail(faultId), (old) =>
        old ? { ...old, fault: { ...old.fault, ...updatedFault } } : old
      );
      queryClient.invalidateQueries({ queryKey: faultKeys.lists() });
    },
  });
}

/** Delete a fault */
export function useDeleteFault() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (faultId: string) => api.deleteFault(faultId),
    onSuccess: (_, faultId) => {
      queryClient.removeQueries({ queryKey: faultKeys.detail(faultId) });
      queryClient.invalidateQueries({ queryKey: faultKeys.lists() });
      // The Total/Open/Closed stats bar sits above the deletable list, so it
      // must refresh after a delete too (#1588 F3). Prefix-invalidate every
      // statistics query (any buildingId variant).
      queryClient.invalidateQueries({ queryKey: [...faultKeys.all, 'statistics'] });
    },
  });
}

/** Triage a fault */
export function useTriageFault(faultId: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (data: TriageFaultRequest) => api.triageFault(faultId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: faultKeys.detail(faultId) });
      queryClient.invalidateQueries({ queryKey: faultKeys.lists() });
    },
  });
}

/** Assign a fault */
export function useAssignFault(faultId: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (assignedTo: string) => api.assignFault(faultId, assignedTo),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: faultKeys.detail(faultId) });
      queryClient.invalidateQueries({ queryKey: faultKeys.lists() });
    },
  });
}

/** Resolve a fault */
export function useResolveFault(faultId: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (data: ResolveFaultRequest) => api.resolveFault(faultId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: faultKeys.detail(faultId) });
      queryClient.invalidateQueries({ queryKey: faultKeys.lists() });
    },
  });
}

/** Confirm fault resolution */
export function useConfirmFault(faultId: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (rating?: number) => api.confirmFault(faultId, rating),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: faultKeys.detail(faultId) });
      queryClient.invalidateQueries({ queryKey: faultKeys.lists() });
    },
  });
}

/** Reopen a fault */
export function useReopenFault(faultId: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (reason: string) => api.reopenFault(faultId, reason),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: faultKeys.detail(faultId) });
      queryClient.invalidateQueries({ queryKey: faultKeys.lists() });
    },
  });
}

/** Add a comment */
export function useAddComment(faultId: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (data: AddCommentRequest) => api.addComment(faultId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: faultKeys.comments(faultId) });
      queryClient.invalidateQueries({ queryKey: faultKeys.detail(faultId) });
    },
  });
}

/** Add a work note */
export function useAddWorkNote(faultId: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (data: AddWorkNoteRequest) => api.addWorkNote(faultId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: faultKeys.detail(faultId) });
    },
  });
}

/** Add an attachment */
export function useAddAttachment(faultId: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (file: File) => api.addAttachment(faultId, file),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: faultKeys.attachments(faultId) });
      queryClient.invalidateQueries({ queryKey: faultKeys.detail(faultId) });
    },
  });
}

/** Delete an attachment */
export function useDeleteAttachment(faultId: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (attachmentId: string) => api.deleteAttachment(faultId, attachmentId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: faultKeys.attachments(faultId) });
      queryClient.invalidateQueries({ queryKey: faultKeys.detail(faultId) });
    },
  });
}

/** Request AI suggestion for a fault (Epic 126) */
export function useRequestAiSuggestion() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (faultId: string) => api.getAiSuggestion(faultId),
    onSuccess: (data, faultId) => {
      queryClient.setQueryData(faultKeys.suggestion(faultId), data);
    },
  });
}

/** Accept AI suggestion and update fault (Epic 126) */
export function useAcceptAiSuggestion(faultId: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (suggestion: { category: string; priority?: string }) =>
      api.acceptAiSuggestion(faultId, suggestion),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: faultKeys.detail(faultId) });
      queryClient.invalidateQueries({ queryKey: faultKeys.lists() });
    },
  });
}
