/**
 * Reports TanStack Query Hooks (Epic 81).
 *
 * React hooks for managing report schedules and executions.
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  getReportExecutionDownloadUrl,
  getReportExecutionHistory,
  pauseSchedule,
  resumeSchedule,
  retryReportExecution,
  updateSchedule,
} from './api';
import type {
  CreateReportSchedule,
  ReportExecutionHistoryParams,
  ReportExecutionStatus,
} from './types';

// Query keys factory for cache management
export const reportKeys = {
  all: ['reports'] as const,
  schedules: () => [...reportKeys.all, 'schedules'] as const,
  schedule: (id: string) => [...reportKeys.schedules(), id] as const,
  executions: (scheduleId: string) => [...reportKeys.schedule(scheduleId), 'executions'] as const,
  executionList: (
    scheduleId: string,
    filters?: { status?: ReportExecutionStatus; dateFrom?: string; dateTo?: string },
    pagination?: { offset?: number; limit?: number }
  ) => [...reportKeys.executions(scheduleId), filters, pagination] as const,
  execution: (id: string) => [...reportKeys.all, 'execution', id] as const,
};

/**
 * Hook to update a report schedule (Story 81.1).
 */
export function useUpdateSchedule() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, data }: { id: string; data: Partial<CreateReportSchedule> }) =>
      updateSchedule(id, data),
    onSuccess: (updatedSchedule) => {
      // Invalidate schedule-related queries
      queryClient.invalidateQueries({ queryKey: reportKeys.schedules() });
      queryClient.invalidateQueries({ queryKey: reportKeys.schedule(updatedSchedule.id) });
    },
  });
}

/**
 * Hook to pause a report schedule (Story 81.1).
 */
export function usePauseSchedule() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: string) => pauseSchedule(id),
    onSuccess: (updatedSchedule) => {
      queryClient.invalidateQueries({ queryKey: reportKeys.schedules() });
      queryClient.invalidateQueries({ queryKey: reportKeys.schedule(updatedSchedule.id) });
    },
  });
}

/**
 * Hook to resume a paused report schedule (Story 81.1).
 */
export function useResumeSchedule() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: string) => resumeSchedule(id),
    onSuccess: (updatedSchedule) => {
      queryClient.invalidateQueries({ queryKey: reportKeys.schedules() });
      queryClient.invalidateQueries({ queryKey: reportKeys.schedule(updatedSchedule.id) });
    },
  });
}

/**
 * Hook to get execution history for a schedule (Story 81.2).
 */
export function useReportExecutionHistory(
  params: Omit<ReportExecutionHistoryParams, 'limit' | 'offset'>,
  options?: {
    limit?: number;
    offset?: number;
    enabled?: boolean;
    refetchInterval?: number | false | ((data: unknown) => number | false);
  }
) {
  return useQuery({
    queryKey: reportKeys.executionList(
      params.scheduleId,
      {
        status: params.status,
        dateFrom: params.dateFrom,
        dateTo: params.dateTo,
      },
      {
        offset: options?.offset ?? 0,
        limit: options?.limit ?? 20,
      }
    ),
    queryFn: () =>
      getReportExecutionHistory({
        ...params,
        limit: options?.limit ?? 20,
        offset: options?.offset ?? 0,
      }),
    enabled: options?.enabled !== false && !!params.scheduleId,
    refetchInterval: options?.refetchInterval,
  });
}

/**
 * Hook to download a report (Story 81.2).
 */
export function useDownloadReport() {
  return useMutation({
    mutationFn: async (executionId: string) => {
      const { url, fileName } = await getReportExecutionDownloadUrl(executionId);

      // Trigger browser download
      const link = document.createElement('a');
      link.href = url;
      link.download = fileName;
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);

      return { url, fileName };
    },
  });
}

/**
 * Hook to retry a failed report execution (Story 81.2).
 */
export function useRetryReportExecution() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (executionId: string) => retryReportExecution(executionId),
    onSuccess: (newExecution) => {
      // Invalidate execution history to show the new execution
      queryClient.invalidateQueries({
        queryKey: reportKeys.executions(newExecution.scheduleId),
      });
    },
  });
}
