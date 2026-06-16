/**
 * TanStack Query hooks for the Voting API (Epic 5, FR37-44).
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import * as api from './api';
import type {
  AddCommentRequest,
  AddQuestionRequest,
  CancelVoteRequest,
  CastVoteRequest,
  CreateVoteRequest,
  ListVotesQuery,
  PublishVoteRequest,
  UpdateQuestionRequest,
  UpdateVoteRequest,
} from './types';

// ============================================================================
// Query keys
// ============================================================================

export const votingKeys = {
  all: ['voting'] as const,
  lists: () => [...votingKeys.all, 'list'] as const,
  list: (query: ListVotesQuery) => [...votingKeys.lists(), query] as const,
  buildingActive: (buildingId: string) =>
    [...votingKeys.all, 'building-active', buildingId] as const,
  details: () => [...votingKeys.all, 'detail'] as const,
  detail: (id: string) => [...votingKeys.details(), id] as const,
  questions: (id: string) => [...votingKeys.all, 'questions', id] as const,
  eligibility: (id: string) => [...votingKeys.all, 'eligibility', id] as const,
  myResponse: (id: string, unitId: string) =>
    [...votingKeys.all, 'my-response', id, unitId] as const,
  results: (id: string) => [...votingKeys.all, 'results', id] as const,
  audit: (id: string) => [...votingKeys.all, 'audit', id] as const,
  comments: (id: string) => [...votingKeys.all, 'comments', id] as const,
};

// ============================================================================
// Query hooks
// ============================================================================

/** List votes with optional filters. */
export function useVotes(query: ListVotesQuery = {}) {
  return useQuery({
    queryKey: votingKeys.list(query),
    queryFn: () => api.listVotes(query),
  });
}

/** List active votes for a building. */
export function useActiveVotesForBuilding(buildingId: string) {
  return useQuery({
    queryKey: votingKeys.buildingActive(buildingId),
    queryFn: () => api.listActiveVotesForBuilding(buildingId),
    enabled: !!buildingId,
  });
}

/** Get a vote with its questions. */
export function useVote(id: string) {
  return useQuery({
    queryKey: votingKeys.detail(id),
    queryFn: () => api.getVote(id),
    enabled: !!id,
  });
}

/** List a vote's questions. */
export function useVoteQuestions(id: string) {
  return useQuery({
    queryKey: votingKeys.questions(id),
    queryFn: () => api.listQuestions(id),
    enabled: !!id,
  });
}

/** Get the current user's eligibility for a vote. */
export function useVoteEligibility(id: string) {
  return useQuery({
    queryKey: votingKeys.eligibility(id),
    queryFn: () => api.getEligibility(id),
    enabled: !!id,
  });
}

/** Get the current user's response for a unit. */
export function useMyVoteResponse(id: string, unitId: string) {
  return useQuery({
    queryKey: votingKeys.myResponse(id, unitId),
    queryFn: () => api.getMyResponse(id, unitId),
    enabled: !!id && !!unitId,
  });
}

/** Get vote results (live tally while active, cached once closed). */
export function useVoteResults(id: string, enabled = true) {
  return useQuery({
    queryKey: votingKeys.results(id),
    queryFn: () => api.getResults(id),
    enabled: enabled && !!id,
  });
}

/** Get the immutable audit log (admin). */
export function useVoteAuditLog(id: string, enabled = true) {
  return useQuery({
    queryKey: votingKeys.audit(id),
    queryFn: () => api.getAuditLog(id),
    enabled: enabled && !!id,
  });
}

/** List a vote's comments. */
export function useVoteComments(id: string) {
  return useQuery({
    queryKey: votingKeys.comments(id),
    queryFn: () => api.listComments(id),
    enabled: !!id,
  });
}

// ============================================================================
// Mutation hooks
// ============================================================================

/** Create a vote. */
export function useCreateVote() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: CreateVoteRequest) => api.createVote(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: votingKeys.lists() });
    },
  });
}

/** Update a draft vote. */
export function useUpdateVote(voteId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: UpdateVoteRequest) => api.updateVote(voteId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: votingKeys.detail(voteId) });
      queryClient.invalidateQueries({ queryKey: votingKeys.lists() });
    },
  });
}

/** Delete a draft vote. */
export function useDeleteVote() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (voteId: string) => api.deleteVote(voteId),
    onSuccess: (_, voteId) => {
      queryClient.removeQueries({ queryKey: votingKeys.detail(voteId) });
      queryClient.invalidateQueries({ queryKey: votingKeys.lists() });
    },
  });
}

/** Publish a draft vote. */
export function usePublishVote(voteId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: PublishVoteRequest = {}) => api.publishVote(voteId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: votingKeys.detail(voteId) });
      queryClient.invalidateQueries({ queryKey: votingKeys.lists() });
    },
  });
}

/** Cancel a vote. */
export function useCancelVote(voteId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: CancelVoteRequest) => api.cancelVote(voteId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: votingKeys.detail(voteId) });
      queryClient.invalidateQueries({ queryKey: votingKeys.lists() });
    },
  });
}

/** Close an active vote. */
export function useCloseVote(voteId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => api.closeVote(voteId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: votingKeys.detail(voteId) });
      queryClient.invalidateQueries({ queryKey: votingKeys.results(voteId) });
      queryClient.invalidateQueries({ queryKey: votingKeys.lists() });
    },
  });
}

/** Add a question to a draft vote. */
export function useAddQuestion(voteId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: AddQuestionRequest) => api.addQuestion(voteId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: votingKeys.questions(voteId) });
      queryClient.invalidateQueries({ queryKey: votingKeys.detail(voteId) });
    },
  });
}

/** Update a question on a draft vote. */
export function useUpdateQuestion(voteId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ questionId, data }: { questionId: string; data: UpdateQuestionRequest }) =>
      api.updateQuestion(voteId, questionId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: votingKeys.questions(voteId) });
      queryClient.invalidateQueries({ queryKey: votingKeys.detail(voteId) });
    },
  });
}

/** Delete a question from a draft vote. */
export function useDeleteQuestion(voteId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (questionId: string) => api.deleteQuestion(voteId, questionId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: votingKeys.questions(voteId) });
      queryClient.invalidateQueries({ queryKey: votingKeys.detail(voteId) });
    },
  });
}

/** Cast a ballot. */
export function useCastVote(voteId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: CastVoteRequest) => api.castVote(voteId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: votingKeys.detail(voteId) });
      queryClient.invalidateQueries({ queryKey: votingKeys.results(voteId) });
      queryClient.invalidateQueries({ queryKey: votingKeys.eligibility(voteId) });
      // Prefix match invalidates my-response for every unit of this vote.
      queryClient.invalidateQueries({ queryKey: [...votingKeys.all, 'my-response', voteId] });
    },
  });
}

/** Add a comment to a vote. */
export function useAddVoteComment(voteId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: AddCommentRequest) => api.addComment(voteId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: votingKeys.comments(voteId) });
    },
  });
}
