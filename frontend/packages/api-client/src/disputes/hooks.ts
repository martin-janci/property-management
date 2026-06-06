/**
 * Disputes React Query Hooks
 *
 * React Query hooks for Disputes API (Epic 80).
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import * as api from './api';
import type {
  AddMediationNoteRequest,
  AssignMediatorRequest,
  CreateDisputeRequest,
  DisputeFilters,
  DisputeListQuery,
  EscalateDisputeRequest,
  ResolveDisputeRequest,
  ScheduleSessionRequest,
  TimelineQuery,
  UpdateDisputeStatusRequest,
  UpdateSessionRequest,
} from './types';

// ============================================
// Query Key Factory
// ============================================

export const disputeKeys = {
  all: ['disputes'] as const,
  lists: () => [...disputeKeys.all, 'list'] as const,
  list: (orgId: string, filters?: DisputeFilters) =>
    [...disputeKeys.lists(), orgId, filters] as const,
  details: () => [...disputeKeys.all, 'detail'] as const,
  detail: (id: string) => [...disputeKeys.details(), id] as const,
  timeline: (id: string) => [...disputeKeys.detail(id), 'timeline'] as const,
  evidence: (id: string) => [...disputeKeys.detail(id), 'evidence'] as const,
  notes: (id: string) => [...disputeKeys.detail(id), 'notes'] as const,
  sessions: (id: string) => [...disputeKeys.detail(id), 'sessions'] as const,
  statistics: (orgId: string) => [...disputeKeys.all, 'statistics', orgId] as const,
};

// ============================================
// List & Detail Queries (Story 80.1)
// ============================================

export function useDisputes(organizationId: string, query?: DisputeListQuery) {
  return useQuery({
    queryKey: disputeKeys.list(organizationId, query),
    queryFn: () => api.listDisputes(organizationId, query),
    staleTime: 30_000,
  });
}

export function useDispute(id: string) {
  return useQuery({
    queryKey: disputeKeys.detail(id),
    queryFn: () => api.getDispute(id),
    enabled: !!id,
    staleTime: 60_000,
  });
}

export function useDisputeStatistics(organizationId: string) {
  return useQuery({
    queryKey: disputeKeys.statistics(organizationId),
    queryFn: () => api.getDisputeStatistics(organizationId),
    staleTime: 60_000,
  });
}

// ============================================
// Timeline & Evidence Queries (Story 80.1)
// ============================================

export function useDisputeTimeline(disputeId: string, query?: TimelineQuery) {
  return useQuery({
    queryKey: [...disputeKeys.timeline(disputeId), query],
    queryFn: () => api.getTimeline(disputeId, query),
    enabled: !!disputeId,
  });
}

export function useDisputeEvidence(disputeId: string) {
  return useQuery({
    queryKey: disputeKeys.evidence(disputeId),
    queryFn: () => api.listEvidence(disputeId),
    enabled: !!disputeId,
  });
}

export function useMediationNotes(disputeId: string) {
  return useQuery({
    queryKey: disputeKeys.notes(disputeId),
    queryFn: () => api.listMediationNotes(disputeId),
    enabled: !!disputeId,
  });
}

// ============================================
// Mediation Session Queries & Mutations (Story 80.3)
// ============================================

export function useMediationSessions(disputeId: string) {
  return useQuery({
    queryKey: disputeKeys.sessions(disputeId),
    queryFn: () => api.listSessions(disputeId),
    enabled: !!disputeId,
    staleTime: 30_000,
  });
}

export function useScheduleSession(disputeId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: ScheduleSessionRequest) => api.scheduleSession(disputeId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: disputeKeys.sessions(disputeId) });
      queryClient.invalidateQueries({ queryKey: disputeKeys.timeline(disputeId) });
    },
  });
}

export function useUpdateSession(disputeId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ sessionId, data }: { sessionId: string; data: UpdateSessionRequest }) =>
      api.updateSession(disputeId, sessionId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: disputeKeys.sessions(disputeId) });
      queryClient.invalidateQueries({ queryKey: disputeKeys.timeline(disputeId) });
    },
  });
}

export function useCancelSession(disputeId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (sessionId: string) => api.cancelSession(disputeId, sessionId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: disputeKeys.sessions(disputeId) });
      queryClient.invalidateQueries({ queryKey: disputeKeys.timeline(disputeId) });
    },
  });
}

// ============================================
// Create & Upload Mutations (Story 80.2)
// ============================================

export function useCreateDispute(organizationId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: CreateDisputeRequest) => api.createDispute(organizationId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: disputeKeys.lists(),
      });
      queryClient.invalidateQueries({
        queryKey: disputeKeys.statistics(organizationId),
      });
    },
  });
}

export function useUploadEvidence(disputeId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ file, description }: { file: File; description: string }) =>
      api.uploadEvidence(disputeId, file, description),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: disputeKeys.detail(disputeId),
      });
      queryClient.invalidateQueries({
        queryKey: disputeKeys.evidence(disputeId),
      });
      queryClient.invalidateQueries({
        queryKey: disputeKeys.timeline(disputeId),
      });
    },
  });
}

export function useDeleteEvidence(disputeId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (evidenceId: string) => api.deleteEvidence(disputeId, evidenceId),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: disputeKeys.detail(disputeId),
      });
      queryClient.invalidateQueries({
        queryKey: disputeKeys.evidence(disputeId),
      });
    },
  });
}

// ============================================
// Mediation & Resolution Mutations (Story 80.3)
// ============================================

export function useAssignMediator() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ disputeId, data }: { disputeId: string; data: AssignMediatorRequest }) =>
      api.assignMediator(disputeId, data),
    onSuccess: (_, { disputeId }) => {
      queryClient.invalidateQueries({
        queryKey: disputeKeys.detail(disputeId),
      });
      queryClient.invalidateQueries({
        queryKey: disputeKeys.lists(),
      });
      queryClient.invalidateQueries({
        queryKey: disputeKeys.timeline(disputeId),
      });
    },
  });
}

export function useUpdateDisputeStatus(organizationId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ disputeId, data }: { disputeId: string; data: UpdateDisputeStatusRequest }) =>
      api.updateDisputeStatus(disputeId, data),
    onSuccess: (_, { disputeId }) => {
      queryClient.invalidateQueries({
        queryKey: disputeKeys.detail(disputeId),
      });
      queryClient.invalidateQueries({
        queryKey: disputeKeys.lists(),
      });
      queryClient.invalidateQueries({
        queryKey: disputeKeys.timeline(disputeId),
      });
      queryClient.invalidateQueries({
        queryKey: disputeKeys.statistics(organizationId),
      });
    },
  });
}

export function useResolveDispute(organizationId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ disputeId, data }: { disputeId: string; data: ResolveDisputeRequest }) =>
      api.resolveDispute(disputeId, data),
    onSuccess: (_, { disputeId }) => {
      queryClient.invalidateQueries({
        queryKey: disputeKeys.detail(disputeId),
      });
      queryClient.invalidateQueries({
        queryKey: disputeKeys.lists(),
      });
      queryClient.invalidateQueries({
        queryKey: disputeKeys.timeline(disputeId),
      });
      queryClient.invalidateQueries({
        queryKey: disputeKeys.statistics(organizationId),
      });
    },
  });
}

export function useEscalateDispute(organizationId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ disputeId, data }: { disputeId: string; data: EscalateDisputeRequest }) =>
      api.escalateDispute(disputeId, data),
    onSuccess: (_, { disputeId }) => {
      queryClient.invalidateQueries({
        queryKey: disputeKeys.detail(disputeId),
      });
      queryClient.invalidateQueries({
        queryKey: disputeKeys.lists(),
      });
      queryClient.invalidateQueries({
        queryKey: disputeKeys.timeline(disputeId),
      });
      queryClient.invalidateQueries({
        queryKey: disputeKeys.statistics(organizationId),
      });
    },
  });
}

export function useAddMediationNote(disputeId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: AddMediationNoteRequest) => api.addMediationNote(disputeId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: disputeKeys.detail(disputeId),
      });
      queryClient.invalidateQueries({
        queryKey: disputeKeys.notes(disputeId),
      });
      queryClient.invalidateQueries({
        queryKey: disputeKeys.timeline(disputeId),
      });
    },
  });
}
