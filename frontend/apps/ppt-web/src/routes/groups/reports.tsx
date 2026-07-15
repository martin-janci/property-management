/**
 * Reports route group (Epic 81).
 *
 * Owns the reports route-wrapper components and the `<Route>` table fragment.
 * Extracted from App.tsx to isolate reports work.
 */
import type {
  CreateReportSchedule,
  CronScheduleUpdateRequest,
  PeriodComparison,
  ReportSchedule,
  TrendAnalysis,
} from '@ppt/api-client';
import {
  useCreateSchedule,
  useDownloadReport,
  useReportExecutionHistory,
  useReportSchedules,
  useRetryReportExecution,
  useUpdateScheduleCron,
} from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Route, useNavigate, useParams } from 'react-router-dom';
import { ProtectedRoute, useToast } from '../../components';
import { useAuth } from '../../contexts';
import { ReportsPage, ScheduleDetailPage } from '../lazyRoutes';

/**
 * Reports page route — wires pause/resume/update schedule hooks (Story 81.1).
 *
 * The EditScheduleModal pause/resume buttons call these handlers which in
 * turn invoke the live backend endpoints PUT /api/v1/reports/schedules/{id}/pause
 * and PUT /api/v1/reports/schedules/{id}/resume (from PR #448).
 */
function ReportsPageRoute() {
  const { showToast } = useToast();
  const { t } = useTranslation();
  const { user } = useAuth();
  const organizationId = user?.organizationId ?? '';

  // issue #2324: the "New Schedule" report selector is fed by the fixed set of
  // built-in report types (ScheduleForm → BUILTIN_REPORTS), not a
  // report-definitions endpoint — there is deliberately no such table (#2198).
  // The Schedules tab list is backed by GET /api/v1/reports/schedules.
  const { data: schedulesData, isLoading: schedulesLoading } = useReportSchedules(organizationId);
  const createScheduleMutation = useCreateSchedule(organizationId);

  const handleCreateSchedule = async (data: CreateReportSchedule) => {
    try {
      await createScheduleMutation.mutateAsync(data);
      showToast({
        type: 'success',
        title: t('common.success'),
        message: t('reports.schedule.created', 'Schedule created successfully.'),
      });
    } catch (e) {
      // Surface the backend's specific error (e.g. INVALID_FREQUENCY) instead of
      // a fixed string so the user sees why creation failed (issue #2324).
      const detail = e instanceof Error && e.message ? e.message : undefined;
      showToast({
        type: 'error',
        title: t('common.error'),
        message: detail ?? t('reports.schedule.createFailed', 'Failed to create schedule.'),
      });
      throw e;
    }
  };

  // gap-81-1: cron-based update (cron_expression, recipients, enabled)
  const updateScheduleCronMutation = useUpdateScheduleCron();

  const handleUpdateSchedule = async (id: string, data: CronScheduleUpdateRequest) => {
    try {
      await updateScheduleCronMutation.mutateAsync({ id, data });
      showToast({
        type: 'success',
        title: t('common.success'),
        message: t('reports.schedule.updated', 'Schedule updated successfully.'),
      });
    } catch (e) {
      showToast({
        type: 'error',
        title: t('common.error'),
        message: t('reports.schedule.updateFailed', 'Failed to update schedule.'),
      });
      throw e;
    }
  };

  // --- Story 81.2: Execution history state and hooks ---
  const [activeScheduleId, setActiveScheduleId] = useState<string>('');
  const [executionOffset, setExecutionOffset] = useState(0);
  const EXECUTION_PAGE_SIZE = 20;

  const { data: executionHistoryData, isLoading: executionsLoading } = useReportExecutionHistory(
    { scheduleId: activeScheduleId },
    {
      limit: EXECUTION_PAGE_SIZE,
      offset: executionOffset,
      enabled: !!activeScheduleId,
      refetchInterval: activeScheduleId ? 10_000 : false,
    }
  );

  const downloadReport = useDownloadReport();
  const retryExecution = useRetryReportExecution();

  const executions = executionHistoryData?.executions ?? [];
  const hasMore = executionHistoryData?.hasMore ?? false;

  const handleFetchExecutions = (scheduleId: string) => {
    setActiveScheduleId(scheduleId);
    setExecutionOffset(0);
  };

  const handleLoadMoreExecutions = () => {
    setExecutionOffset((prev) => prev + EXECUTION_PAGE_SIZE);
  };

  const handleDownloadReport = (executionId: string) => {
    downloadReport.mutate(executionId, {
      onError: () => {
        showToast({
          type: 'error',
          title: t('common.error'),
          message: t('reports.execution.downloadFailed', 'Failed to download report.'),
        });
      },
    });
  };

  const handleRetryExecution = async (executionId: string) => {
    try {
      await retryExecution.mutateAsync(executionId);
      showToast({
        type: 'success',
        title: t('common.success'),
        message: t('reports.execution.retryQueued', 'Execution queued for retry.'),
      });
    } catch {
      showToast({
        type: 'error',
        title: t('common.error'),
        message: t('reports.execution.retryFailed', 'Failed to retry execution.'),
      });
      throw new Error('retry failed');
    }
  };

  return (
    <ReportsPage
      organizationId={organizationId}
      dataSources={[]}
      reports={[]}
      schedules={schedulesData?.schedules ?? []}
      isLoading={schedulesLoading}
      kpis={[]}
      buildings={[]}
      trendAnalysis={
        {
          metric: '',
          period: 'monthly',
          current_value: 0,
          previous_value: 0,
          change: 0,
          change_percentage: 0,
          trend: 'stable' as const,
          anomalies: [],
        } as TrendAnalysis
      }
      trendLines={[]}
      periodComparison={
        {
          metric: '',
          periods: [],
          difference: 0,
          difference_percentage: 0,
        } as PeriodComparison
      }
      onCreateSchedule={handleCreateSchedule}
      onUpdateSchedule={handleUpdateSchedule}
      executions={executions}
      executionsLoading={executionsLoading}
      executionsHasMore={hasMore}
      onFetchExecutions={handleFetchExecutions}
      onLoadMoreExecutions={handleLoadMoreExecutions}
      onDownloadReport={handleDownloadReport}
      onRetryExecution={handleRetryExecution}
    />
  );
}

/**
 * Schedule detail page route — renders execution history table inline (Story 81.2).
 *
 * Route: /reports/schedules/:scheduleId
 * Uses useReportExecutionHistory to fetch paginated executions and renders them
 * via ScheduleDetailPage. Falls back to MSW stub data when api-server is absent.
 */
function ScheduleDetailPageRoute() {
  const { scheduleId = '' } = useParams<{ scheduleId: string }>();
  const navigate = useNavigate();
  const { showToast } = useToast();
  const { t } = useTranslation();

  const EXECUTION_PAGE_SIZE = 20;
  const [executionOffset, setExecutionOffset] = useState(0);

  const {
    data: executionHistoryData,
    isLoading: executionsLoading,
    isError: executionsError,
  } = useReportExecutionHistory(
    { scheduleId },
    {
      limit: EXECUTION_PAGE_SIZE,
      offset: executionOffset,
      enabled: !!scheduleId,
      refetchInterval: 10_000,
    }
  );

  const downloadReport = useDownloadReport();
  const retryExecution = useRetryReportExecution();

  const executions = executionHistoryData?.executions ?? [];
  const hasMore = executionHistoryData?.hasMore ?? false;

  const handleLoadMore = () => setExecutionOffset((prev) => prev + EXECUTION_PAGE_SIZE);

  const handleDownloadReport = (executionId: string) => {
    downloadReport.mutate(executionId, {
      onError: () => {
        showToast({
          type: 'error',
          title: t('common.error'),
          message: t('reports.execution.downloadFailed', 'Failed to download report.'),
        });
      },
    });
  };

  const handleRetryExecution = async (executionId: string) => {
    try {
      await retryExecution.mutateAsync(executionId);
      showToast({
        type: 'success',
        title: t('common.success'),
        message: t('reports.execution.retryQueued', 'Execution queued for retry.'),
      });
    } catch {
      showToast({
        type: 'error',
        title: t('common.error'),
        message: t('reports.execution.retryFailed', 'Failed to retry execution.'),
      });
      throw new Error('retry failed');
    }
  };

  // Stub schedule object — schedule metadata will be enriched once the
  // GET /api/v1/reports/schedules/:id endpoint is wired in a future story.
  const stubSchedule: ReportSchedule = {
    id: scheduleId,
    report_id: '',
    organization_id: '',
    name: `Schedule ${scheduleId}`,
    frequency: 'monthly',
    time: '08:00',
    timezone: 'UTC',
    format: 'pdf',
    recipients: [],
    is_active: true,
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  };

  return (
    <ScheduleDetailPage
      schedule={stubSchedule}
      executions={executions}
      isLoading={executionsLoading}
      isError={executionsError}
      hasMore={hasMore}
      onLoadMore={handleLoadMore}
      onDownload={handleDownloadReport}
      onRetry={handleRetryExecution}
      onBack={() => navigate('/reports')}
    />
  );
}

/** Reports routes (Epic 81). */
export function reportRoutes() {
  return (
    <>
      <Route
        path="/reports"
        element={
          <ProtectedRoute>
            <ReportsPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/reports/schedules/:scheduleId"
        element={
          <ProtectedRoute>
            <ScheduleDetailPageRoute />
          </ProtectedRoute>
        }
      />
    </>
  );
}
