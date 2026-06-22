/**
 * FaultReportsPage — fault analytics / reports surface (Story 4.7, BIT-186).
 *
 * Manager-only. Lets a manager pick a building + date range, then renders the
 * top-line KPIs (count, avg resolution time, avg rating) and status/category/
 * priority breakdown charts with drill-down into the faults list. Supports CSV
 * and PDF (print) export.
 *
 * Presentational: the building/date filters are controlled by the route
 * wrapper (FaultReportsPageRoute), which drives `useFaultStatistics`.
 */

import type { FaultStatistics } from '@ppt/api-client';
import { useTranslation } from 'react-i18next';
import { FaultBreakdownChart, type FaultBreakdownDatum } from '../components/FaultBreakdownChart';
import { buildStatisticsCsv, downloadCsv, type FaultReportMeta } from '../utils/faultReportExport';

export interface FaultReportFilters {
  buildingId?: string;
  dateFrom?: string;
  dateTo?: string;
}

interface FaultReportsPageProps {
  /** Manager-equivalent access (mirrors isManagerRole). */
  isManager: boolean;
  stats?: FaultStatistics;
  isLoading?: boolean;
  isError?: boolean;
  onRetry?: () => void;
  buildings: Array<{ id: string; name: string }>;
  filters: FaultReportFilters;
  onFiltersChange: (filters: FaultReportFilters) => void;
  /** Drill down into the faults list filtered by a status/category/priority. */
  onDrillDown: (params: { status?: string; category?: string; priority?: string }) => void;
}

/** Humanise a snake_case enum value for display (`waiting_parts` → `Waiting parts`). */
function humanise(value: string): string {
  const s = value.replace(/_/g, ' ');
  return s.charAt(0).toUpperCase() + s.slice(1);
}

function toData(buckets: Array<{ key: string; count: number }>): FaultBreakdownDatum[] {
  return buckets.map((b) => ({ key: b.key, label: humanise(b.key), count: b.count }));
}

export function FaultReportsPage({
  isManager,
  stats,
  isLoading,
  isError,
  onRetry,
  buildings,
  filters,
  onFiltersChange,
  onDrillDown,
}: FaultReportsPageProps) {
  const { t } = useTranslation();

  if (!isManager) {
    return (
      <div className="max-w-5xl mx-auto px-4 py-8">
        <div
          role="alert"
          className="border border-amber-200 bg-amber-50 text-amber-800 rounded-lg p-6 text-center"
        >
          <p className="font-medium">
            {t('faults.reports.forbidden', {
              defaultValue: 'You do not have permission to view fault reports.',
            })}
          </p>
        </div>
      </div>
    );
  }

  const buildingLabel =
    buildings.find((b) => b.id === filters.buildingId)?.name ??
    t('faults.reports.allBuildings', { defaultValue: 'All buildings' });

  const handleExportCsv = () => {
    if (!stats) return;
    const meta: FaultReportMeta = {
      buildingLabel,
      dateFrom: filters.dateFrom,
      dateTo: filters.dateTo,
    };
    downloadCsv('fault-report.csv', buildStatisticsCsv(stats, meta));
  };

  const handleExportPdf = () => {
    // Browser print pipeline — the print stylesheet below hides chrome so the
    // user can "Save as PDF" from the dialog. Dependency-free.
    window.print();
  };

  return (
    <div className="max-w-5xl mx-auto px-4 py-8 fault-reports">
      {/* Print stylesheet: hide interactive controls so the PDF shows data only. */}
      <style>{`@media print {
        .fault-reports__no-print { display: none !important; }
        .fault-reports { max-width: none; padding: 0; }
      }`}</style>

      <header className="mb-6 flex flex-wrap items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">
            {t('faults.reports.title', { defaultValue: 'Fault Reports' })}
          </h1>
          <p className="text-sm text-gray-500">
            {t('faults.reports.subtitle', {
              defaultValue: 'Fault analytics across buildings and time.',
            })}
          </p>
        </div>
        <div className="fault-reports__no-print flex gap-2">
          <button
            type="button"
            onClick={handleExportCsv}
            disabled={!stats}
            className="px-3 py-2 text-sm font-medium border border-gray-300 rounded-lg text-gray-700 hover:bg-gray-50 disabled:opacity-50"
          >
            {t('faults.reports.exportCsv', { defaultValue: 'Export CSV' })}
          </button>
          <button
            type="button"
            onClick={handleExportPdf}
            disabled={!stats}
            className="px-3 py-2 text-sm font-medium border border-gray-300 rounded-lg text-gray-700 hover:bg-gray-50 disabled:opacity-50"
          >
            {t('faults.reports.exportPdf', { defaultValue: 'Export PDF' })}
          </button>
        </div>
      </header>

      {/* Filters */}
      <div className="fault-reports__no-print mb-6 grid grid-cols-1 sm:grid-cols-3 gap-4 p-4 bg-gray-50 border border-gray-200 rounded-lg">
        <label className="flex flex-col gap-1 text-sm">
          <span className="font-medium text-gray-700">
            {t('faults.reports.building', { defaultValue: 'Building' })}
          </span>
          <select
            value={filters.buildingId ?? ''}
            onChange={(e) =>
              onFiltersChange({ ...filters, buildingId: e.target.value || undefined })
            }
            className="border border-gray-300 rounded-lg px-3 py-2"
          >
            <option value="">
              {t('faults.reports.allBuildings', { defaultValue: 'All buildings' })}
            </option>
            {buildings.map((b) => (
              <option key={b.id} value={b.id}>
                {b.name}
              </option>
            ))}
          </select>
        </label>
        <label className="flex flex-col gap-1 text-sm">
          <span className="font-medium text-gray-700">
            {t('faults.reports.dateFrom', { defaultValue: 'From' })}
          </span>
          <input
            type="date"
            value={filters.dateFrom ?? ''}
            max={filters.dateTo || undefined}
            onChange={(e) => onFiltersChange({ ...filters, dateFrom: e.target.value || undefined })}
            className="border border-gray-300 rounded-lg px-3 py-2"
          />
        </label>
        <label className="flex flex-col gap-1 text-sm">
          <span className="font-medium text-gray-700">
            {t('faults.reports.dateTo', { defaultValue: 'To' })}
          </span>
          <input
            type="date"
            value={filters.dateTo ?? ''}
            min={filters.dateFrom || undefined}
            onChange={(e) => onFiltersChange({ ...filters, dateTo: e.target.value || undefined })}
            className="border border-gray-300 rounded-lg px-3 py-2"
          />
        </label>
      </div>

      {/* Loading */}
      {isLoading && (
        <div className="flex justify-center py-16" aria-live="polite" aria-busy="true">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
        </div>
      )}

      {/* Error */}
      {!isLoading && isError && (
        <div
          role="alert"
          className="text-center py-8 border border-red-200 bg-red-50 rounded-lg text-red-700"
        >
          <p className="font-medium">
            {t('faults.reports.failedToLoad', {
              defaultValue: 'Failed to load fault statistics',
            })}
          </p>
          {onRetry && (
            <button
              type="button"
              onClick={onRetry}
              className="mt-3 px-3 py-1 border border-red-300 rounded hover:bg-red-100"
            >
              {t('common.retry', { defaultValue: 'Retry' })}
            </button>
          )}
        </div>
      )}

      {/* Empty */}
      {!isLoading && !isError && stats && stats.total_count === 0 && (
        <div className="text-center py-12 text-gray-500">
          {t('faults.reports.empty', {
            defaultValue: 'No faults match the selected filters.',
          })}
        </div>
      )}

      {/* Content */}
      {!isLoading && !isError && stats && stats.total_count > 0 && (
        <>
          {/* KPIs */}
          <div className="grid grid-cols-2 lg:grid-cols-5 gap-4 mb-8">
            <Kpi
              label={t('faults.reports.kpiTotal', { defaultValue: 'Total faults' })}
              value={stats.total_count}
            />
            <Kpi
              label={t('faults.reports.kpiOpen', { defaultValue: 'Open' })}
              value={stats.open_count}
              valueClassName="text-blue-600"
            />
            <Kpi
              label={t('faults.reports.kpiClosed', { defaultValue: 'Closed' })}
              value={stats.closed_count}
              valueClassName="text-gray-400"
            />
            <Kpi
              label={t('faults.reports.kpiResolution', { defaultValue: 'Avg resolution (h)' })}
              value={
                stats.average_resolution_time_hours == null
                  ? '—'
                  : stats.average_resolution_time_hours.toFixed(1)
              }
            />
            <Kpi
              label={t('faults.reports.kpiRating', { defaultValue: 'Avg rating' })}
              value={stats.average_rating == null ? '—' : `${stats.average_rating.toFixed(2)} ★`}
              valueClassName="text-amber-500"
            />
          </div>

          {/* Breakdown charts */}
          <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
            <FaultBreakdownChart
              title={t('faults.reports.byStatus', { defaultValue: 'By status' })}
              data={toData(stats.by_status.map((s) => ({ key: s.status, count: s.count })))}
              barClassName="bg-blue-500"
              onSelect={(status) => onDrillDown({ status })}
              emptyLabel={t('faults.reports.noData', { defaultValue: 'No data available' })}
            />
            <FaultBreakdownChart
              title={t('faults.reports.byCategory', { defaultValue: 'By category' })}
              data={toData(stats.by_category.map((c) => ({ key: c.category, count: c.count })))}
              barClassName="bg-emerald-500"
              onSelect={(category) => onDrillDown({ category })}
              emptyLabel={t('faults.reports.noData', { defaultValue: 'No data available' })}
            />
            <FaultBreakdownChart
              title={t('faults.reports.byPriority', { defaultValue: 'By priority' })}
              data={toData(stats.by_priority.map((p) => ({ key: p.priority, count: p.count })))}
              barClassName="bg-amber-500"
              onSelect={(priority) => onDrillDown({ priority })}
              emptyLabel={t('faults.reports.noData', { defaultValue: 'No data available' })}
            />
          </div>
        </>
      )}
    </div>
  );
}

function Kpi({
  label,
  value,
  valueClassName = 'text-gray-900',
}: {
  label: string;
  value: string | number;
  valueClassName?: string;
}) {
  return (
    <div className="bg-white rounded-lg shadow p-4">
      <p className="text-sm font-medium text-gray-500">{label}</p>
      <p className={`mt-2 text-2xl font-bold ${valueClassName}`}>{value}</p>
    </div>
  );
}
