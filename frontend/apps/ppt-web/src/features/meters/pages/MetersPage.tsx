/**
 * MetersPage - main page listing all meters for building/unit.
 * Meters feature: Self-readings and consumption tracking.
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { MeterCard } from '../components/MeterCard';
import type { ExportFormat, ExportOptions, Meter, MeterType } from '../types';

type ExportStep = 'closed' | 'format' | 'dateRange' | 'confirm';

interface MetersPageProps {
  meters: Meter[];
  buildingName?: string;
  unitDesignation?: string;
  isLoading?: boolean;
  isExporting?: boolean;
  onNavigateToDetail: (meterId: string) => void;
  onNavigateToSubmitReading: (meterId: string) => void;
  onNavigateToComparison?: () => void;
  onExport?: (options: ExportOptions) => void;
}

export function MetersPage({
  meters,
  buildingName,
  unitDesignation,
  isLoading,
  isExporting,
  onNavigateToDetail,
  onNavigateToSubmitReading,
  onNavigateToComparison,
  onExport,
}: MetersPageProps) {
  const { t, i18n } = useTranslation();
  const [typeFilter, setTypeFilter] = useState<MeterType | ''>('');
  const [showInactive, setShowInactive] = useState(false);

  // Export modal state
  const [exportStep, setExportStep] = useState<ExportStep>('closed');
  const [exportFormat, setExportFormat] = useState<ExportFormat>('csv');
  const [exportStartDate, setExportStartDate] = useState(() => {
    const date = new Date();
    date.setMonth(date.getMonth() - 1);
    return date.toISOString().split('T')[0];
  });
  const [exportEndDate, setExportEndDate] = useState(() => {
    return new Date().toISOString().split('T')[0];
  });

  const filteredMeters = meters.filter((meter) => {
    if (typeFilter && meter.meterType !== typeFilter) return false;
    if (!showInactive && !meter.isActive) return false;
    return true;
  });

  const meterTypes: MeterType[] = [
    'electricity',
    'gas',
    'water',
    'heat',
    'cold_water',
    'hot_water',
  ];

  const handleExportClick = () => {
    setExportStep('format');
  };

  const handleExportFormatSelect = (format: ExportFormat) => {
    setExportFormat(format);
    setExportStep('dateRange');
  };

  const handleExportDateConfirm = () => {
    setExportStep('confirm');
  };

  const handleExportConfirm = () => {
    if (onExport) {
      onExport({
        format: exportFormat,
        startDate: exportStartDate,
        endDate: exportEndDate,
        meterIds: filteredMeters.map((m) => m.id),
        includeConsumption: true,
      });
    }
    setExportStep('closed');
  };

  const handleExportCancel = () => {
    setExportStep('closed');
  };

  return (
    <div className="max-w-4xl mx-auto px-4 py-8">
      {/* Header */}
      <div className="mb-6 flex items-start justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">{t('meters.title')}</h1>
          {buildingName && (
            <p className="text-gray-600">
              {buildingName}
              {unitDesignation && ` - ${unitDesignation}`}
            </p>
          )}
        </div>
        <div className="flex items-center gap-2">
          {onNavigateToComparison && (
            <button
              type="button"
              onClick={onNavigateToComparison}
              className="px-3 py-2 text-sm border border-gray-300 rounded-lg hover:bg-gray-50 flex items-center gap-2"
            >
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"
                />
              </svg>
              {t('meters.compare')}
            </button>
          )}
          {onExport && (
            <button
              type="button"
              onClick={handleExportClick}
              disabled={isExporting || meters.length === 0}
              className="px-3 py-2 text-sm bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 flex items-center gap-2"
            >
              {isExporting ? (
                <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-white" />
              ) : (
                <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
                  />
                </svg>
              )}
              {t('meters.export.button')}
            </button>
          )}
        </div>
      </div>

      {/* Filters */}
      <div className="mb-6 flex flex-wrap items-center gap-4">
        <div className="flex items-center gap-2">
          <label htmlFor="typeFilter" className="text-sm font-medium text-gray-700">
            {t('meters.type')}:
          </label>
          <select
            id="typeFilter"
            value={typeFilter}
            onChange={(e) => setTypeFilter(e.target.value as MeterType | '')}
            className="rounded-md border border-gray-300 px-3 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
          >
            <option value="">{t('common.all')}</option>
            {meterTypes.map((type) => (
              <option key={type} value={type}>
                {t(`meters.types.${type}`)}
              </option>
            ))}
          </select>
        </div>

        <label className="flex items-center gap-2 text-sm text-gray-600">
          <input
            type="checkbox"
            checked={showInactive}
            onChange={(e) => setShowInactive(e.target.checked)}
            className="rounded border-gray-300"
          />
          {t('meters.showInactive')}
        </label>
      </div>

      {/* Loading state */}
      {isLoading && (
        <div className="flex justify-center py-12">
          <div className="animate-spin rounded-full h-10 w-10 border-b-2 border-blue-600" />
        </div>
      )}

      {/* Empty state */}
      {!isLoading && filteredMeters.length === 0 && (
        <div className="text-center py-12 bg-gray-50 rounded-lg">
          <svg
            className="mx-auto h-12 w-12 text-gray-400"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"
            />
          </svg>
          <p className="mt-2 text-gray-500">{t('meters.noMeters')}</p>
        </div>
      )}

      {/* Meters Grid */}
      {!isLoading && filteredMeters.length > 0 && (
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {filteredMeters.map((meter) => (
            <MeterCard
              key={meter.id}
              meter={meter}
              onView={onNavigateToDetail}
              onSubmitReading={onNavigateToSubmitReading}
            />
          ))}
        </div>
      )}

      {/* Summary */}
      {!isLoading && meters.length > 0 && (
        <div className="mt-6 text-sm text-gray-500">
          {t('common.showing')} {filteredMeters.length} {t('common.of')} {meters.length}{' '}
          {t('meters.metersLabel')}
        </div>
      )}

      {/* Export Modal */}
      {exportStep !== 'closed' && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-50">
          <div className="bg-white rounded-lg shadow-xl max-w-md w-full mx-4 p-6">
            {/* Format Selection Step */}
            {exportStep === 'format' && (
              <>
                <h2 className="text-lg font-semibold text-gray-900 mb-4">
                  {t('meters.export.selectFormat')}
                </h2>
                <div className="grid grid-cols-2 gap-4">
                  <button
                    type="button"
                    onClick={() => handleExportFormatSelect('csv')}
                    className={`p-4 border-2 rounded-lg text-center hover:border-blue-500 ${
                      exportFormat === 'csv' ? 'border-blue-500 bg-blue-50' : 'border-gray-200'
                    }`}
                  >
                    <svg
                      className="w-8 h-8 mx-auto mb-2 text-green-600"
                      fill="none"
                      stroke="currentColor"
                      viewBox="0 0 24 24"
                    >
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
                      />
                    </svg>
                    <span className="font-medium">CSV</span>
                    <p className="text-xs text-gray-500 mt-1">{t('meters.export.csvDesc')}</p>
                  </button>
                  <button
                    type="button"
                    onClick={() => handleExportFormatSelect('excel')}
                    className={`p-4 border-2 rounded-lg text-center hover:border-blue-500 ${
                      exportFormat === 'excel' ? 'border-blue-500 bg-blue-50' : 'border-gray-200'
                    }`}
                  >
                    <svg
                      className="w-8 h-8 mx-auto mb-2 text-green-700"
                      fill="none"
                      stroke="currentColor"
                      viewBox="0 0 24 24"
                    >
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M3 10h18M3 14h18m-9-4v8m-7 0h14a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z"
                      />
                    </svg>
                    <span className="font-medium">Excel</span>
                    <p className="text-xs text-gray-500 mt-1">{t('meters.export.excelDesc')}</p>
                  </button>
                </div>
                <div className="mt-6 flex justify-end">
                  <button
                    type="button"
                    onClick={handleExportCancel}
                    className="px-4 py-2 text-gray-700 hover:bg-gray-100 rounded-lg"
                  >
                    {t('common.cancel')}
                  </button>
                </div>
              </>
            )}

            {/* Date Range Step */}
            {exportStep === 'dateRange' && (
              <>
                <h2 className="text-lg font-semibold text-gray-900 mb-4">
                  {t('meters.export.selectDateRange')}
                </h2>
                <div className="space-y-4">
                  <div>
                    <label
                      htmlFor="exportStartDate"
                      className="block text-sm font-medium text-gray-700 mb-1"
                    >
                      {t('meters.export.startDate')}
                    </label>
                    <input
                      type="date"
                      id="exportStartDate"
                      value={exportStartDate}
                      onChange={(e) => setExportStartDate(e.target.value)}
                      className="w-full rounded-md border border-gray-300 px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
                    />
                  </div>
                  <div>
                    <label
                      htmlFor="exportEndDate"
                      className="block text-sm font-medium text-gray-700 mb-1"
                    >
                      {t('meters.export.endDate')}
                    </label>
                    <input
                      type="date"
                      id="exportEndDate"
                      value={exportEndDate}
                      onChange={(e) => setExportEndDate(e.target.value)}
                      max={new Date().toISOString().split('T')[0]}
                      className="w-full rounded-md border border-gray-300 px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
                    />
                  </div>
                </div>
                <div className="mt-6 flex justify-between">
                  <button
                    type="button"
                    onClick={() => setExportStep('format')}
                    className="px-4 py-2 text-gray-700 hover:bg-gray-100 rounded-lg"
                  >
                    {t('common.back')}
                  </button>
                  <button
                    type="button"
                    onClick={handleExportDateConfirm}
                    disabled={!exportStartDate || !exportEndDate}
                    className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50"
                  >
                    {t('common.next')}
                  </button>
                </div>
              </>
            )}

            {/* Confirmation Step */}
            {exportStep === 'confirm' && (
              <>
                <h2 className="text-lg font-semibold text-gray-900 mb-4">
                  {t('meters.export.confirm')}
                </h2>
                <div className="bg-gray-50 rounded-lg p-4 space-y-2 text-sm">
                  <div className="flex justify-between">
                    <span className="text-gray-600">{t('meters.export.format')}:</span>
                    <span className="font-medium">{exportFormat.toUpperCase()}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-600">{t('meters.export.dateRange')}:</span>
                    <span className="font-medium">
                      {new Date(exportStartDate).toLocaleDateString(i18n.language)} -{' '}
                      {new Date(exportEndDate).toLocaleDateString(i18n.language)}
                    </span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-600">{t('meters.export.metersCount')}:</span>
                    <span className="font-medium">{filteredMeters.length}</span>
                  </div>
                </div>
                <div className="mt-6 flex justify-between">
                  <button
                    type="button"
                    onClick={() => setExportStep('dateRange')}
                    className="px-4 py-2 text-gray-700 hover:bg-gray-100 rounded-lg"
                  >
                    {t('common.back')}
                  </button>
                  <button
                    type="button"
                    onClick={handleExportConfirm}
                    className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 flex items-center gap-2"
                  >
                    <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
                      />
                    </svg>
                    {t('meters.export.download')}
                  </button>
                </div>
              </>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
