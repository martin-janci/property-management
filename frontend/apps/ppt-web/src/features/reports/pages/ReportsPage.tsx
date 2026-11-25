/**
 * ReportsPage - Reports & schedules page.
 *
 * Combines on-demand generation of the built-in reports (gap audit 3.1) with
 * the Epic 81 schedule-management / execution-history components. The former
 * free-form report builder targeted `/reports/definitions` + `/preview`
 * routes that never existed (there is no report-definitions table, #2198), so
 * the Reports tab now generates the four built-in reports directly.
 *
 * NOTE: the former "Dashboard" analytics tab (KPIs, trend charts, building
 * metrics, period comparison) was removed — it was a Potemkin surface fed by
 * hardcoded empty objects from the route container, backed by
 * `/api/v1/reports/analytics/*` + `/trends/*` endpoints that the reports router
 * never registered (code-review-ppt-web-core-reports-route-stub-analytics).
 * Re-add it only alongside a real analytics endpoint + api-client hooks.
 */

import type {
  CreateReportSchedule,
  CronScheduleUpdateRequest,
  ReportExecution,
  ReportSchedule,
} from '@ppt/api-client';
import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useConfirmDialog } from '../../../components/useConfirmDialog';
import {
  EditScheduleModal,
  ExecutionHistory,
  GenerateReportForm,
  type GenerateReportValues,
  ScheduleForm,
  ScheduleList,
} from '../components';

interface ReportsPageProps {
  schedules: ReportSchedule[];
  isLoading?: boolean;
  /** Generate + download a built-in report (POST /api/v1/reports/export). */
  onGenerateReport: (values: GenerateReportValues) => Promise<void> | void;
  isGeneratingReport?: boolean;
  onCreateSchedule?: (data: CreateReportSchedule) => Promise<void>;
  /** gap-81-1: cron-based update (cron_expression, recipients, enabled) */
  onUpdateSchedule?: (id: string, data: CronScheduleUpdateRequest) => Promise<void>;
  onDeleteSchedule?: (id: string) => Promise<void>;
  onToggleSchedule?: (id: string, isActive: boolean) => Promise<void>;
  onRunScheduleNow?: (id: string) => Promise<void>;
  // Execution history props
  executions?: ReportExecution[];
  executionsLoading?: boolean;
  executionsHasMore?: boolean;
  onLoadMoreExecutions?: () => void;
  onDownloadReport?: (executionId: string) => void;
  onRetryExecution?: (executionId: string) => Promise<void>;
  onFetchExecutions?: (scheduleId: string) => void;
}

type Tab = 'reports' | 'schedules';

export function ReportsPage({
  schedules,
  isLoading,
  onGenerateReport,
  isGeneratingReport,
  onCreateSchedule,
  onUpdateSchedule,
  onDeleteSchedule,
  onToggleSchedule,
  onRunScheduleNow,
  executions,
  executionsLoading,
  executionsHasMore,
  onLoadMoreExecutions,
  onDownloadReport,
  onRetryExecution,
  onFetchExecutions,
}: ReportsPageProps) {
  const { t } = useTranslation();
  const { confirm, confirmDialog } = useConfirmDialog();
  const [activeTab, setActiveTab] = useState<Tab>('schedules');
  const [showScheduleForm, setShowScheduleForm] = useState(false);

  // Epic 81: Schedule editing state
  const [editingSchedule, setEditingSchedule] = useState<ReportSchedule | null>(null);
  const [isEditModalOpen, setIsEditModalOpen] = useState(false);
  const [isUpdatingSchedule, setIsUpdatingSchedule] = useState(false);

  // Epic 81: Execution history state
  const [viewingHistorySchedule, setViewingHistorySchedule] = useState<ReportSchedule | null>(null);
  const [isHistoryOpen, setIsHistoryOpen] = useState(false);

  // Epic 81.1: Handle schedule editing
  const handleEditSchedule = useCallback((schedule: ReportSchedule) => {
    setEditingSchedule(schedule);
    setIsEditModalOpen(true);
  }, []);

  const handleCloseEditModal = useCallback(() => {
    setIsEditModalOpen(false);
    setEditingSchedule(null);
  }, []);

  const handleSaveSchedule = useCallback(
    async (id: string, data: CronScheduleUpdateRequest) => {
      if (!onUpdateSchedule) return;
      setIsUpdatingSchedule(true);
      try {
        await onUpdateSchedule(id, data);
        handleCloseEditModal();
      } finally {
        setIsUpdatingSchedule(false);
      }
    },
    [onUpdateSchedule, handleCloseEditModal]
  );

  // Epic 81.2: Handle execution history
  const handleViewHistory = useCallback(
    (scheduleId: string) => {
      const schedule = schedules.find((s) => s.id === scheduleId);
      if (schedule) {
        setViewingHistorySchedule(schedule);
        setIsHistoryOpen(true);
        onFetchExecutions?.(scheduleId);
      }
    },
    [schedules, onFetchExecutions]
  );

  const handleCloseHistory = useCallback(() => {
    setIsHistoryOpen(false);
    setViewingHistorySchedule(null);
  }, []);

  const tabs = [
    { id: 'reports' as Tab, label: t('reports.page.tabs.reports'), icon: '📋' },
    { id: 'schedules' as Tab, label: t('reports.page.tabs.schedules'), icon: '⏰' },
  ];

  return (
    <div className="min-h-screen bg-gray-100">
      {/* Header */}
      <div className="bg-white shadow">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
          <div className="flex items-center justify-between">
            <h1 className="text-2xl font-bold text-gray-900">{t('reports.page.title')}</h1>
            <div className="flex gap-3">
              {activeTab === 'schedules' && (
                <button
                  type="button"
                  onClick={() => setShowScheduleForm(true)}
                  className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700"
                >
                  {t('reports.page.newSchedule')}
                </button>
              )}
            </div>
          </div>

          {/* Tabs */}
          <div className="mt-4 border-b border-gray-200">
            <nav className="-mb-px flex gap-8">
              {tabs.map((tab) => (
                <button
                  key={tab.id}
                  type="button"
                  onClick={() => setActiveTab(tab.id)}
                  className={`py-4 px-1 border-b-2 font-medium text-sm transition-colors ${
                    activeTab === tab.id
                      ? 'border-blue-500 text-blue-600'
                      : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
                  }`}
                >
                  <span className="mr-2">{tab.icon}</span>
                  {tab.label}
                </button>
              ))}
            </nav>
          </div>
        </div>
      </div>

      {/* Content */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {/* Reports Tab: on-demand generation of the built-in reports */}
        {activeTab === 'reports' && (
          <GenerateReportForm isGenerating={isGeneratingReport} onGenerate={onGenerateReport} />
        )}

        {/* Schedules Tab */}
        {activeTab === 'schedules' && (
          <div>
            {showScheduleForm ? (
              <div className="bg-white rounded-lg shadow p-6">
                <h2 className="text-lg font-medium text-gray-900 mb-6">
                  {t('reports.page.createSchedule')}
                </h2>
                <ScheduleForm
                  onSubmit={async (data) => {
                    await onCreateSchedule?.(data);
                    setShowScheduleForm(false);
                  }}
                  onCancel={() => setShowScheduleForm(false)}
                />
              </div>
            ) : (
              <ScheduleList
                schedules={schedules}
                isLoading={isLoading}
                onEdit={handleEditSchedule}
                onDelete={async (id) => {
                  const schedule = schedules.find((s) => s.id === id);
                  const confirmed = await confirm({
                    title: t('reports.page.deleteSchedule.title'),
                    message: t('reports.page.deleteSchedule.message', {
                      name: schedule?.name ?? '',
                    }),
                    confirmLabel: t('reports.page.deleteSchedule.confirm'),
                    variant: 'danger',
                  });
                  if (confirmed) {
                    await onDeleteSchedule?.(id);
                  }
                }}
                onToggle={async (id, isActive) => {
                  await onToggleSchedule?.(id, isActive);
                }}
                onRunNow={async (id) => {
                  await onRunScheduleNow?.(id);
                }}
                onViewHistory={handleViewHistory}
              />
            )}
          </div>
        )}
      </div>

      {/* Epic 81.1: Edit Schedule Modal */}
      {editingSchedule && (
        <EditScheduleModal
          schedule={editingSchedule}
          isOpen={isEditModalOpen}
          isSubmitting={isUpdatingSchedule}
          onClose={handleCloseEditModal}
          onSave={handleSaveSchedule}
        />
      )}

      {/* Epic 81.2: Execution History Modal */}
      {viewingHistorySchedule && isHistoryOpen && (
        <ExecutionHistory
          scheduleId={viewingHistorySchedule.id}
          scheduleName={viewingHistorySchedule.name}
          executions={executions ?? []}
          isLoading={executionsLoading}
          hasMore={executionsHasMore}
          onLoadMore={onLoadMoreExecutions}
          onDownload={onDownloadReport}
          onRetry={onRetryExecution}
          onClose={handleCloseHistory}
        />
      )}

      {confirmDialog}
    </div>
  );
}
