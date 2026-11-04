/**
 * Disputes React Query Hooks
 *
 * React Query hooks for Disputes API (Epic 80).
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import * as api from './api';
import type {
  AssignMediatorRequest,
  CreateActionItemRequest,
  CreateDisputeRequest,
  DisputeFilters,
  DisputeListQuery,
  EscalateDisputeRequest,
  ProposeResolutionRequest,
  ResolveDisputeRequest,
  ScheduleSessionRequest,
  SubmitResponseRequest,
  TimelineQuery,
  UpdateDisputeStatusRequest,
  UpdateSessionRequest,
  VoteOnResolutionRequest,
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
  resolutions: (id: string) => [...disputeKeys.detail(id), 'resolutions'] as const,
  actions: (id: string) => [...disputeKeys.detail(id), 'actions'] as const,
  sessions: (id: string) => [...disputeKeys.detail(id), 'sessions'] as const,
  submissions: (id: string) => [...disputeKeys.detail(id), 'submissions'] as const,
  statistics: (orgId: string) => [...disputeKeys.all, 'statistics', orgId] as const,
};

// ============================================
// List & Detail Queries (Story 80.1)
// ============================================

export function useDisputes(organizationId: string, query?: DisputeListQuery) {
  return useQuery({
    queryKey: disputeKeys.list(organizationId, query),
    queryFn: () => api.listDisputes(organizationId, query),
    // No organization yet (e.g. while auth restores): don't fetch.
    enabled: !!organizationId,
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
// Party Submission Queries & Mutations (Story 80.3)
// ============================================

export function useDisputeSubmissions(disputeId: string) {
  return useQuery({
    queryKey: disputeKeys.submissions(disputeId),
    queryFn: () => api.listSubmissions(disputeId),
    enabled: !!disputeId,
    staleTime: 30_000,
  });
}

export function useSubmitResponse(disputeId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: SubmitResponseRequest) => api.submitResponse(disputeId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: disputeKeys.submissions(disputeId) });
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

// ============================================
// Resolution Tracking (Story 77.3)
// ============================================

/**
 * Resolutions for a dispute, each with its votes and acceptance rate. The list
 * route carries no votes, so each resolution is fetched with its votes.
 */
export function useDisputeResolutions(disputeId: string) {
  return useQuery({
    queryKey: disputeKeys.resolutions(disputeId),
    queryFn: async () => {
      const list = await api.listResolutions(disputeId);
      return Promise.all(list.map((r) => api.getResolution(disputeId, r.id)));
    },
    enabled: !!disputeId,
  });
}

export function useProposeResolution(disputeId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: ProposeResolutionRequest) => api.proposeResolution(disputeId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: disputeKeys.detail(disputeId) });
    },
  });
}

export function useVoteOnResolution(disputeId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ resolutionId, data }: { resolutionId: string; data: VoteOnResolutionRequest }) =>
      api.voteOnResolution(disputeId, resolutionId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: disputeKeys.detail(disputeId) });
    },
  });
}

export function useAcceptResolution(disputeId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (resolutionId: string) => api.acceptResolution(disputeId, resolutionId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: disputeKeys.detail(disputeId) });
    },
  });
}

export function useImplementResolution(disputeId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (resolutionId: string) => api.implementResolution(disputeId, resolutionId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: disputeKeys.detail(disputeId) });
    },
  });
}

// ============================================
// Enforcement (Story 77.4)
// ============================================

export function useDisputeActionItems(disputeId: string) {
  return useQuery({
    queryKey: disputeKeys.actions(disputeId),
    queryFn: () => api.listActionItems(disputeId),
    enabled: !!disputeId,
  });
}

export function useCreateActionItem(disputeId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: CreateActionItemRequest) => api.createActionItem(disputeId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: disputeKeys.detail(disputeId) });
    },
  });
}

export function useCompleteActionItem(disputeId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ actionId, notes }: { actionId: string; notes?: string }) =>
      api.completeActionItem(disputeId, actionId, notes),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: disputeKeys.detail(disputeId) });
    },
  });
}

export function useSendActionReminder(disputeId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (actionId: string) => api.sendActionReminder(disputeId, actionId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: disputeKeys.actions(disputeId) });
    },
  });
}
