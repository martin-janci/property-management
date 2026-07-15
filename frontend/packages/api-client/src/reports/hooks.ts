/**
 * Reports TanStack Query Hooks (Epic 81).
 *
 * React hooks for managing report schedules and executions.
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  createSchedule,
  getReportExecutionDownloadUrl,
  getReportExecutionHistory,
  listSchedules,
  pauseSchedule,
  resumeSchedule,
  retryReportExecution,
  updateSchedule,
  updateScheduleCron,
} from './api';
import type {
  CreateReportSchedule,
  CronScheduleUpdateRequest,
  ListSchedulesParams,
  ReportExecutionHistoryParams,
  ReportExecutionStatus,
} from './types';

// Query keys factory for cache management
export const reportKeys = {
  all: ['reports'] as const,
  schedules: () => [...reportKeys.all, 'schedules'] as const,
  scheduleList: (organizationId: string, filters?: Omit<ListSchedulesParams, 'organization_id'>) =>
    [...reportKeys.schedules(), 'list', organizationId, filters] as const,
  schedule: (id: string) => [...reportKeys.schedules(), id] as const,
  executions: (scheduleId: string) => [...reportKeys.schedule(scheduleId), 'executions'] as const,
  executionList: (
    scheduleId: string,
    filters?: { status?: ReportExecutionStatus; dateFrom?: string; dateTo?: string },
    pagination?: { offset?: number; limit?: number }
  ) => [...reportKeys.executions(scheduleId), filters, pagination] as const,
  execution: (id: string) => [...reportKeys.all, 'execution', id] as const,
};

// NOTE (issue #2324): the former `useReports` hook was removed. It fetched
// `GET /api/v1/reports/definitions`, a route that does not exist (there is no
// report-definitions table by design — #2198), so it never populated the
// "New Schedule" report selector. The selector now uses the fixed built-in
// report set (`ppt-web` `features/reports/builtinReports.ts`) instead.

/**
 * Hook to list report schedules for an organization (Epic 53 / gap-81-1).
 *
 * Backs the Schedules tab list and lets a freshly-created schedule appear once
 * `useCreateSchedule` invalidates the `schedules` key.
 */
export function useReportSchedules(
  organizationId: string,
  filters?: Omit<ListSchedulesParams, 'organization_id'>,
  options?: { enabled?: boolean }
) {
  return useQuery({
    queryKey: reportKeys.scheduleList(organizationId, filters),
    queryFn: () => listSchedules({ organization_id: organizationId, ...filters }),
    enabled: options?.enabled !== false && !!organizationId,
  });
}

/**
 * Hook to create a report schedule (gap-81-1).
 *
 * Wires the "New Schedule" form's submit to POST /api/v1/reports/schedules.
 * On success the `schedules` query key is invalidated so the new row shows up
 * in the Schedules list without a manual refetch.
 */
export function useCreateSchedule(organizationId: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (data: CreateReportSchedule) => createSchedule(organizationId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: reportKeys.schedules() });
    },
  });
}

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
 * Hook to update a report schedule via the cron-based endpoint (gap-81-1).
 *
 * Sends cron_expression, recipients, and/or enabled to PUT /schedules/{id}.
 */
export function useUpdateScheduleCron() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, data }: { id: string; data: CronScheduleUpdateRequest }) =>
      updateScheduleCron(id, data),
    onSuccess: (updatedSchedule) => {
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

      // Guard against a malformed/empty presigned URL: an empty href would
      // navigate the anchor to the current page instead of downloading the
      // report, so treat it as a failure the caller's onError can surface.
      if (!url) {
        throw new Error('Download URL is missing from the server response');
      }

      // Trigger browser download. Use try/finally so the temporary anchor is
      // always removed from the DOM, even if click() throws (otherwise a thrown
      // click leaks a detached <a> node into <body>).
      const link = document.createElement('a');
      link.href = url;
      link.download = fileName;
      document.body.appendChild(link);
      try {
        link.click();
      } finally {
        document.body.removeChild(link);
      }

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
