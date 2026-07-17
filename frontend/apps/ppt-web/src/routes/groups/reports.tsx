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
  ApiError,
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
 * Map a backend `ErrorResponse.code` from the create-schedule endpoint to a
 * localized message key (issue #2370). Piping the raw `error.message` into the
 * toast leaked English into the 5 non-English locales, since the backend
 * messages are English literals. Unknown codes fall back to the generic
 * `reports.schedule.createFailed` copy.
 */
const CREATE_SCHEDULE_ERROR_KEYS: Record<string, string> = {
  EMPTY_NAME: 'reports.schedule.createErrors.emptyName',
  INVALID_FREQUENCY: 'reports.schedule.createErrors.invalidFrequency',
  INVALID_RECIPIENT_EMAIL: 'reports.schedule.createErrors.invalidRecipientEmail',
  INVALID_TIMEZONE: 'reports.schedule.createErrors.invalidTimezone',
  // issue #2403: the backend `create_schedule` handler emits these codes too, but
  // they were unmapped and fell back to the generic "Failed to create schedule".
  TOO_MANY_RECIPIENTS: 'reports.schedule.createErrors.tooManyRecipients',
  INVALID_TIME: 'reports.schedule.createErrors.invalidTime',
  INVALID_FORMAT: 'reports.schedule.createErrors.invalidFormat',
  INVALID_DAY_OF_WEEK: 'reports.schedule.createErrors.invalidDayOfWeek',
  INVALID_DAY_OF_MONTH: 'reports.schedule.createErrors.invalidDayOfMonth',
};

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
      // Surface *why* creation failed (issue #2324) but via a localized message
      // keyed on the backend error `code`, not the raw English `error.message`
      // which leaked untranslated copy into non-English locales (issue #2370).
      const code = e instanceof ApiError ? e.code : undefined;
      const messageKey = code ? CREATE_SCHEDULE_ERROR_KEYS[code] : undefined;
      showToast({
        type: 'error',
        title: t('common.error'),
        message: messageKey
          ? t(messageKey)
          : t('reports.schedule.createFailed', 'Failed to create schedule.'),
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
