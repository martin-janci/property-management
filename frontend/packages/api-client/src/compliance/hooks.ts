/**
 * Compliance React Hooks (Epic 90 - Frontend API Integration).
 * Originally from Epic 67, integrated as part of Epic 90.
 *
 * TanStack Query hooks for AML, content moderation, and DSA compliance.
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import * as api from './api';
import type {
  DecideAppealRequest,
  GenerateDsaReportRequest,
  InitiateEddRequest,
  ReviewAmlAssessmentRequest,
  TakeModerationActionRequest,
} from './types';

// =============================================================================
// Query Keys
// =============================================================================

export const complianceKeys = {
  all: ['compliance'] as const,
  // Moderation
  moderation: () => [...complianceKeys.all, 'moderation'] as const,
  moderationCases: (params?: Record<string, unknown>) =>
    [...complianceKeys.moderation(), 'cases', params] as const,
  moderationCase: (id: string) => [...complianceKeys.moderation(), 'case', id] as const,
  moderationStats: () => [...complianceKeys.moderation(), 'stats'] as const,
  moderationTemplates: () => [...complianceKeys.moderation(), 'templates'] as const,
  // AML
  aml: () => [...complianceKeys.all, 'aml'] as const,
  amlAssessments: (params?: Record<string, unknown>) =>
    [...complianceKeys.aml(), 'assessments', params] as const,
  amlThresholds: () => [...complianceKeys.aml(), 'thresholds'] as const,
  countryRisks: () => [...complianceKeys.aml(), 'country-risks'] as const,
  // DSA
  dsa: () => [...complianceKeys.all, 'dsa'] as const,
  dsaReports: (params?: Record<string, unknown>) =>
    [...complianceKeys.dsa(), 'reports', params] as const,
  dsaMetrics: () => [...complianceKeys.dsa(), 'metrics'] as const,
};

// =============================================================================
// Content Moderation Hooks
// =============================================================================

export function useModerationCases(params?: {
  status?: string;
  content_type?: string;
  violation_type?: string;
  priority?: number;
  unassigned_only?: boolean;
  overdue?: boolean;
  limit?: number;
  offset?: number;
}) {
  return useQuery({
    queryKey: complianceKeys.moderationCases(params),
    queryFn: () => api.listModerationCases(params),
  });
}

export function useModerationCase(id: string) {
  return useQuery({
    queryKey: complianceKeys.moderationCase(id),
    queryFn: () => api.getModerationCase(id),
    enabled: !!id,
  });
}

export function useModerationStats() {
  return useQuery({
    queryKey: complianceKeys.moderationStats(),
    queryFn: () => api.getModerationStats(),
  });
}

export function useModerationTemplates() {
  return useQuery({
    queryKey: complianceKeys.moderationTemplates(),
    queryFn: () => api.getModerationTemplates(),
  });
}

export function useAssignModerationCase() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (caseId: string) => api.assignModerationCase(caseId),
    onSuccess: (_, caseId) => {
      queryClient.invalidateQueries({ queryKey: complianceKeys.moderationCase(caseId) });
      queryClient.invalidateQueries({ queryKey: complianceKeys.moderation() });
    },
  });
}

export function useUnassignModerationCase() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (caseId: string) => api.unassignModerationCase(caseId),
    onSuccess: (_, caseId) => {
      queryClient.invalidateQueries({ queryKey: complianceKeys.moderationCase(caseId) });
      queryClient.invalidateQueries({ queryKey: complianceKeys.moderation() });
    },
  });
}

export function useTakeModerationAction() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ caseId, request }: { caseId: string; request: TakeModerationActionRequest }) =>
      api.takeModerationAction(caseId, request),
    onSuccess: (_, { caseId }) => {
      queryClient.invalidateQueries({ queryKey: complianceKeys.moderationCase(caseId) });
      queryClient.invalidateQueries({ queryKey: complianceKeys.moderation() });
    },
  });
}

export function useDecideModerationAppeal() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ caseId, request }: { caseId: string; request: DecideAppealRequest }) =>
      api.decideModerationAppeal(caseId, request),
    onSuccess: (_, { caseId }) => {
      queryClient.invalidateQueries({ queryKey: complianceKeys.moderationCase(caseId) });
      queryClient.invalidateQueries({ queryKey: complianceKeys.moderation() });
    },
  });
}

// =============================================================================
// AML Hooks
// =============================================================================

export function useAmlAssessments(params?: {
  status?: string;
  risk_level?: string;
  flagged_only?: boolean;
  limit?: number;
  offset?: number;
}) {
  return useQuery({
    queryKey: complianceKeys.amlAssessments(params),
    queryFn: () => api.listAmlAssessments(params),
  });
}

export function useAmlThresholds() {
  return useQuery({
    queryKey: complianceKeys.amlThresholds(),
    queryFn: () => api.getAmlThresholds(),
  });
}

export function useCountryRisks() {
  return useQuery({
    queryKey: complianceKeys.countryRisks(),
    queryFn: () => api.getCountryRisks(),
  });
}

export function useInitiateEdd() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (request: InitiateEddRequest) => api.initiateEdd(request),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: complianceKeys.aml() });
    },
  });
}

export function useReviewAmlAssessment() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({
      assessmentId,
      request,
    }: {
      assessmentId: string;
      request: ReviewAmlAssessmentRequest;
    }) => api.reviewAmlAssessment(assessmentId, request),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: complianceKeys.aml() });
    },
  });
}

// =============================================================================
// DSA Hooks
// =============================================================================

export function useDsaReports(params?: { status?: string; limit?: number; offset?: number }) {
  return useQuery({
    queryKey: complianceKeys.dsaReports(params),
    queryFn: () => api.listDsaReports(params),
  });
}

export function useDsaMetrics() {
  return useQuery({
    queryKey: complianceKeys.dsaMetrics(),
    queryFn: () => api.getDsaMetrics(),
  });
}

export function useGenerateDsaReport() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (request: GenerateDsaReportRequest) => api.generateDsaReport(request),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: complianceKeys.dsa() });
    },
  });
}

export function usePublishDsaReport() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (reportId: string) => api.publishDsaReport(reportId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: complianceKeys.dsa() });
    },
  });
}

export function useDownloadDsaReportPdf() {
  return useMutation({
    mutationFn: (reportId: string) => api.downloadDsaReportPdf(reportId),
    onSuccess: (data) => {
      // Trigger PDF download
      window.open(data.url, '_blank');
    },
  });
}
