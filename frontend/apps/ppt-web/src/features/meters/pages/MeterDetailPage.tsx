/**
 * MeterDetailPage - view meter details and reading history.
 * Meters feature: Self-readings and consumption tracking.
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ConsumptionChart } from '../components/ConsumptionChart';
import { ReadingLineChart } from '../components/ReadingLineChart';
import type { ConsumptionHistory, Meter, MeterReading } from '../types';

type ViewMode = 'list' | 'chart';

interface MeterDetailPageProps {
  meter: Meter;
  readings: MeterReading[];
  consumption?: ConsumptionHistory;
  isLoading?: boolean;
  onBack: () => void;
  onSubmitReading: () => void;
  onViewReading?: (readingId: string) => void;
  onEditReading?: (readingId: string) => void;
}

const statusColors: Record<string, string> = {
  pending: 'bg-yellow-100 text-yellow-800',
  validated: 'bg-green-100 text-green-800',
  rejected: 'bg-red-100 text-red-800',
  corrected: 'bg-blue-100 text-blue-800',
};

export function MeterDetailPage({
  meter,
  readings,
  consumption,
  isLoading,
  onBack,
  onSubmitReading,
  onViewReading,
  onEditReading,
}: MeterDetailPageProps) {
  const { t, i18n } = useTranslation();
  const [viewMode, setViewMode] = useState<ViewMode>('list');

  if (isLoading) {
    return (
      <div className="flex justify-center py-12">
        <div className="animate-spin rounded-full h-10 w-10 border-b-2 border-blue-600" />
      </div>
    );
  }

  // Calculate average consumption and trend
  const calculateStats = () => {
    if (readings.length < 2) {
      return { average: 0, trend: 'stable' as const, trendPercentage: 0 };
    }

    const consumptionValues: number[] = [];
    for (let i = 0; i < readings.length - 1; i++) {
      const diff = readings[i].value - readings[i + 1].value;
      if (diff >= 0) {
        consumptionValues.push(diff);
      }
    }

    if (consumptionValues.length === 0) {
      return { average: 0, trend: 'stable' as const, trendPercentage: 0 };
    }

    const average = consumptionValues.reduce((a, b) => a + b, 0) / consumptionValues.length;

    // Calculate trend from recent vs older readings
    const recentHalf = consumptionValues.slice(0, Math.floor(consumptionValues.length / 2));
    const olderHalf = consumptionValues.slice(Math.floor(consumptionValues.length / 2));

    if (recentHalf.length === 0 || olderHalf.length === 0) {
      return { average, trend: 'stable' as const, trendPercentage: 0 };
    }

    const recentAvg = recentHalf.reduce((a, b) => a + b, 0) / recentHalf.length;
    const olderAvg = olderHalf.reduce((a, b) => a + b, 0) / olderHalf.length;

    const trendPercentage = olderAvg !== 0 ? ((recentAvg - olderAvg) / olderAvg) * 100 : 0;

    let trend: 'up' | 'down' | 'stable' = 'stable';
    if (trendPercentage > 5) trend = 'up';
    else if (trendPercentage < -5) trend = 'down';

    return { average, trend, trendPercentage };
  };

  const stats = calculateStats();

  // Prepare data for line chart
  const chartData = readings
    .slice()
    .reverse()
    .map((reading, index, arr) => ({
      date: reading.readingDate,
      value: reading.value,
      consumption: index > 0 ? reading.value - arr[index - 1].value : undefined,
    }));

  return (
    <div className="max-w-4xl mx-auto px-4 py-8">
      {/* Header */}
      <div className="mb-6">
        <button
          type="button"
          onClick={onBack}
          className="text-sm text-blue-600 hover:text-blue-800 flex items-center gap-1 mb-4"
        >
          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M15 19l-7-7 7-7"
            />
          </svg>
          {t('meters.backToMeters')}
        </button>
        <div className="flex items-start justify-between">
          <div>
            <h1 className="text-2xl font-bold text-gray-900">
              {t(`meters.types.${meter.meterType}`)}
            </h1>
            <p className="text-gray-600">{meter.serialNumber}</p>
          </div>
          {meter.isActive && (
            <button
              type="button"
              onClick={onSubmitReading}
              className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700"
            >
              {t('meters.submitReading')}
            </button>
          )}
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Main Content */}
        <div className="lg:col-span-2 space-y-6">
          {/* Stats Cards */}
          <div className="grid grid-cols-2 gap-4">
            <div className="bg-white rounded-lg shadow p-4">
              <p className="text-sm text-gray-500">{t('meters.averageConsumption')}</p>
              <p className="text-2xl font-bold text-gray-900">
                {stats.average.toFixed(1)} {meter.unit}
              </p>
              <p className="text-xs text-gray-400">{t('meters.perPeriod')}</p>
            </div>
            <div className="bg-white rounded-lg shadow p-4">
              <p className="text-sm text-gray-500">{t('meters.trend')}</p>
              <div className="flex items-center gap-2">
                {stats.trend === 'up' && (
                  <svg
                    className="w-6 h-6 text-red-500"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M5 10l7-7m0 0l7 7m-7-7v18"
                    />
                  </svg>
                )}
                {stats.trend === 'down' && (
                  <svg
                    className="w-6 h-6 text-green-500"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M19 14l-7 7m0 0l-7-7m7 7V3"
                    />
                  </svg>
                )}
                {stats.trend === 'stable' && (
                  <svg
                    className="w-6 h-6 text-gray-500"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M4 12h16"
                    />
                  </svg>
                )}
                <span
                  className={`text-2xl font-bold ${
                    stats.trend === 'up'
                      ? 'text-red-600'
                      : stats.trend === 'down'
                        ? 'text-green-600'
                        : 'text-gray-600'
                  }`}
                >
                  {stats.trendPercentage > 0 ? '+' : ''}
                  {stats.trendPercentage.toFixed(1)}%
                </span>
              </div>
              <p className="text-xs text-gray-400">
                {stats.trend === 'up' && t('meters.trendUp')}
                {stats.trend === 'down' && t('meters.trendDown')}
                {stats.trend === 'stable' && t('meters.trendStable')}
              </p>
            </div>
          </div>

          {/* Consumption Chart */}
          {consumption && <ConsumptionChart data={consumption} height={250} showTrend={true} />}

          {/* Reading History with View Toggle */}
          <div className="bg-white rounded-lg shadow p-6">
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-lg font-semibold text-gray-900">{t('meters.readingHistory')}</h2>
              <div className="flex items-center gap-1 bg-gray-100 rounded-lg p-1">
                <button
                  type="button"
                  onClick={() => setViewMode('list')}
                  className={`px-3 py-1.5 text-sm rounded-md transition-colors ${
                    viewMode === 'list'
                      ? 'bg-white shadow text-gray-900'
                      : 'text-gray-600 hover:text-gray-900'
                  }`}
                >
                  <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M4 6h16M4 10h16M4 14h16M4 18h16"
                    />
                  </svg>
                </button>
                <button
                  type="button"
                  onClick={() => setViewMode('chart')}
                  className={`px-3 py-1.5 text-sm rounded-md transition-colors ${
                    viewMode === 'chart'
                      ? 'bg-white shadow text-gray-900'
                      : 'text-gray-600 hover:text-gray-900'
                  }`}
                >
                  <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M7 12l3-3 3 3 4-4M8 21l4-4 4 4M3 4h18M4 4h16v12a1 1 0 01-1 1H5a1 1 0 01-1-1V4z"
                    />
                  </svg>
                </button>
              </div>
            </div>

            {readings.length === 0 ? (
              <p className="text-gray-500 text-center py-8">{t('meters.noReadings')}</p>
            ) : viewMode === 'chart' ? (
              <ReadingLineChart data={chartData} unit={meter.unit} height={300} />
            ) : (
              <div className="overflow-x-auto">
                <table className="min-w-full divide-y divide-gray-200">
                  <thead className="bg-gray-50">
                    <tr>
                      <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">
                        {t('meters.readingDate')}
                      </th>
                      <th className="px-4 py-3 text-right text-xs font-medium text-gray-500 uppercase">
                        {t('meters.value')}
                      </th>
                      <th className="px-4 py-3 text-center text-xs font-medium text-gray-500 uppercase">
                        {t('meters.status.title')}
                      </th>
                      <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">
                        {t('meters.submittedBy')}
                      </th>
                      <th className="px-4 py-3 text-right text-xs font-medium text-gray-500 uppercase">
                        {t('common.actions')}
                      </th>
                    </tr>
                  </thead>
                  <tbody className="bg-white divide-y divide-gray-200">
                    {readings.map((reading, index) => {
                      const previousReading = readings[index + 1];
                      const consumptionVal = previousReading
                        ? reading.value - previousReading.value
                        : null;

                      return (
                        <tr key={reading.id} className="hover:bg-gray-50">
                          <td className="px-4 py-3 text-sm text-gray-900">
                            {new Date(reading.readingDate).toLocaleDateString(i18n.language)}
                          </td>
                          <td className="px-4 py-3 text-sm text-right font-medium text-gray-900">
                            {reading.value.toLocaleString()} {meter.unit}
                            {consumptionVal !== null && consumptionVal >= 0 && (
                              <span className="block text-xs text-gray-500">
                                (+{consumptionVal.toLocaleString()})
                              </span>
                            )}
                          </td>
                          <td className="px-4 py-3 text-sm text-center">
                            <span
                              className={`px-2 py-1 text-xs font-medium rounded ${statusColors[reading.status]}`}
                            >
                              {t(`meters.status.${reading.status}`)}
                            </span>
                          </td>
                          <td className="px-4 py-3 text-sm text-gray-600">
                            {reading.submittedByName || t('common.unknown')}
                          </td>
                          <td className="px-4 py-3 text-sm text-right">
                            <div className="flex items-center justify-end gap-2">
                              {onViewReading && (
                                <button
                                  type="button"
                                  onClick={() => onViewReading(reading.id)}
                                  className="text-blue-600 hover:text-blue-800"
                                  title={t('common.view')}
                                >
                                  <svg
                                    className="w-4 h-4"
                                    fill="none"
                                    stroke="currentColor"
                                    viewBox="0 0 24 24"
                                  >
                                    <path
                                      strokeLinecap="round"
                                      strokeLinejoin="round"
                                      strokeWidth={2}
                                      d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
                                    />
                                    <path
                                      strokeLinecap="round"
                                      strokeLinejoin="round"
                                      strokeWidth={2}
                                      d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"
                                    />
                                  </svg>
                                </button>
                              )}
                              {onEditReading && reading.status === 'pending' && (
                                <button
                                  type="button"
                                  onClick={() => onEditReading(reading.id)}
                                  className="text-gray-600 hover:text-gray-800"
                                  title={t('common.edit')}
                                >
                                  <svg
                                    className="w-4 h-4"
                                    fill="none"
                                    stroke="currentColor"
                                    viewBox="0 0 24 24"
                                  >
                                    <path
                                      strokeLinecap="round"
                                      strokeLinejoin="round"
                                      strokeWidth={2}
                                      d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"
                                    />
                                  </svg>
                                </button>
                              )}
                            </div>
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            )}
          </div>
        </div>

        {/* Sidebar */}
        <div className="space-y-6">
          {/* Meter Details */}
          <div className="bg-white rounded-lg shadow p-6">
            <h3 className="text-lg font-semibold text-gray-900 mb-4">{t('meters.meterInfo')}</h3>
            <dl className="space-y-3 text-sm">
              <div>
                <dt className="text-gray-500">{t('meters.type')}</dt>
                <dd className="font-medium">{t(`meters.types.${meter.meterType}`)}</dd>
              </div>
              <div>
                <dt className="text-gray-500">{t('meters.serialNumber')}</dt>
                <dd className="font-medium">{meter.serialNumber}</dd>
              </div>
              <div>
                <dt className="text-gray-500">{t('meters.unitOfMeasurement')}</dt>
                <dd className="font-medium">{meter.unit}</dd>
              </div>
              {meter.location && (
                <div>
                  <dt className="text-gray-500">{t('meters.location')}</dt>
                  <dd className="font-medium">{meter.location}</dd>
                </div>
              )}
              {meter.buildingName && (
                <div>
                  <dt className="text-gray-500">{t('buildings.title')}</dt>
                  <dd className="font-medium">{meter.buildingName}</dd>
                </div>
              )}
              {meter.unitDesignation && (
                <div>
                  <dt className="text-gray-500">{t('meters.unit')}</dt>
                  <dd className="font-medium">{meter.unitDesignation}</dd>
                </div>
              )}
              {meter.installationDate && (
                <div>
                  <dt className="text-gray-500">{t('meters.installationDate')}</dt>
                  <dd className="font-medium">
                    {new Date(meter.installationDate).toLocaleDateString(i18n.language)}
                  </dd>
                </div>
              )}
              <div>
                <dt className="text-gray-500">{t('meters.status.title')}</dt>
                <dd className="font-medium">
                  <span className={meter.isActive ? 'text-green-600' : 'text-gray-500'}>
                    {meter.isActive ? t('meters.active') : t('meters.inactive')}
                  </span>
                </dd>
              </div>
            </dl>
          </div>

          {/* Last Reading */}
          {meter.lastReadingValue !== undefined && (
            <div className="bg-blue-50 rounded-lg p-6 border border-blue-100">
              <h3 className="text-lg font-semibold text-blue-900 mb-2">
                {t('meters.lastReading')}
              </h3>
              <p className="text-3xl font-bold text-blue-900">
                {meter.lastReadingValue.toLocaleString()} {meter.unit}
              </p>
              {meter.lastReadingDate && (
                <p className="text-sm text-blue-700 mt-1">
                  {new Date(meter.lastReadingDate).toLocaleDateString(i18n.language)}
                </p>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
