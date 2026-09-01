/**
 * ReadingComparisonPage - compare readings across meters and time.
 * Meters feature: Self-readings and consumption tracking.
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { ComparisonParams, ExportOptions, Meter, ReadingChartData } from '../types';

type ChartView = 'sideBySide' | 'overlay';

// Chart colors for different meters
const CHART_COLORS = [
  '#3b82f6', // blue
  '#10b981', // green
  '#f59e0b', // amber
  '#ef4444', // red
  '#8b5cf6', // violet
  '#ec4899', // pink
];

interface ReadingComparisonPageProps {
  meters: Meter[];
  comparisonData: ReadingChartData[];
  isLoading?: boolean;
  isExporting?: boolean;
  onBack: () => void;
  onCompare: (params: ComparisonParams) => void;
  onExport?: (options: ExportOptions) => void;
}

export function ReadingComparisonPage({
  meters,
  comparisonData,
  isLoading,
  isExporting,
  onBack,
  onCompare,
  onExport,
}: ReadingComparisonPageProps) {
  const { t, i18n } = useTranslation();

  const [selectedMeterIds, setSelectedMeterIds] = useState<string[]>([]);
  const [startDate, setStartDate] = useState(() => {
    const date = new Date();
    date.setMonth(date.getMonth() - 6);
    return date.toISOString().split('T')[0];
  });
  const [endDate, setEndDate] = useState(() => {
    return new Date().toISOString().split('T')[0];
  });
  const [chartView, setChartView] = useState<ChartView>('overlay');

  const handleMeterToggle = (meterId: string) => {
    setSelectedMeterIds((prev) =>
      prev.includes(meterId) ? prev.filter((id) => id !== meterId) : [...prev, meterId]
    );
  };

  const handleCompare = () => {
    if (selectedMeterIds.length > 0) {
      onCompare({
        meterIds: selectedMeterIds,
        startDate,
        endDate,
      });
    }
  };

  const handleExport = () => {
    if (onExport && selectedMeterIds.length > 0) {
      onExport({
        format: 'csv',
        startDate,
        endDate,
        meterIds: selectedMeterIds,
        includeConsumption: true,
      });
    }
  };

  // Calculate chart dimensions
  const chartHeight = 300;
  const padding = { top: 20, right: 20, bottom: 40, left: 60 };

  // Find global min/max across all comparison data
  const allValues = comparisonData.flatMap((d) => d.dataPoints.map((p) => p.value));
  const minValue = allValues.length > 0 ? Math.min(...allValues) : 0;
  const maxValue = allValues.length > 0 ? Math.max(...allValues) : 100;
  const valueRange = maxValue - minValue || 1;
  const paddedMin = minValue - valueRange * 0.1;
  const paddedMax = maxValue + valueRange * 0.1;
  const paddedRange = paddedMax - paddedMin;

  // Find all unique dates
  const allDates = [
    ...new Set(comparisonData.flatMap((d) => d.dataPoints.map((p) => p.date))),
  ].sort();

  const formatValue = (value: number): string => {
    if (value >= 10000) return `${(value / 1000).toFixed(0)}k`;
    if (value >= 1000) return `${(value / 1000).toFixed(1)}k`;
    return value.toFixed(0);
  };

  const formatDate = (dateStr: string): string => {
    const date = new Date(dateStr);
    return date.toLocaleDateString(i18n.language, { month: 'short', day: 'numeric' });
  };

  // Y-axis labels
  const yLabels = Array.from({ length: 5 }, (_, i) => ({
    value: paddedMin + (paddedRange * i) / 4,
    y: 100 - (i / 4) * 100,
  }));

  return (
    <div className="max-w-6xl mx-auto px-4 py-8">
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
            <h1 className="text-2xl font-bold text-gray-900">{t('meters.comparison.title')}</h1>
            <p className="text-gray-600 mt-1">{t('meters.comparison.subtitle')}</p>
          </div>
          {onExport && comparisonData.length > 0 && (
            <button
              type="button"
              onClick={handleExport}
              disabled={isExporting}
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
              {t('meters.comparison.exportData')}
            </button>
          )}
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-4 gap-6">
        {/* Filters Sidebar */}
        <div className="lg:col-span-1 space-y-6">
          {/* Meter Selection */}
          <div className="bg-white rounded-lg shadow p-4">
            <h3 className="text-sm font-medium text-gray-900 mb-3">
              {t('meters.comparison.selectMeters')}
            </h3>
            <div className="space-y-2 max-h-64 overflow-y-auto">
              {meters.map((meter, index) => (
                <label
                  key={meter.id}
                  className="flex items-center gap-3 p-2 rounded hover:bg-gray-50 cursor-pointer"
                >
                  <input
                    type="checkbox"
                    checked={selectedMeterIds.includes(meter.id)}
                    onChange={() => handleMeterToggle(meter.id)}
                    className="rounded border-gray-300 text-blue-600 focus:ring-blue-500"
                  />
                  <div
                    className="w-3 h-3 rounded-full"
                    style={{ backgroundColor: CHART_COLORS[index % CHART_COLORS.length] }}
                  />
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium text-gray-900 truncate">
                      {t(`meters.types.${meter.meterType}`)}
                    </p>
                    <p className="text-xs text-gray-500 truncate">{meter.serialNumber}</p>
                  </div>
                </label>
              ))}
            </div>
            {meters.length === 0 && (
              <p className="text-sm text-gray-500 text-center py-4">{t('meters.noMeters')}</p>
            )}
          </div>

          {/* Date Range */}
          <div className="bg-white rounded-lg shadow p-4">
            <h3 className="text-sm font-medium text-gray-900 mb-3">
              {t('meters.comparison.dateRange')}
            </h3>
            <div className="space-y-3">
              <div>
                <label htmlFor="startDate" className="block text-xs text-gray-500 mb-1">
                  {t('meters.export.startDate')}
                </label>
                <input
                  type="date"
                  id="startDate"
                  value={startDate}
                  onChange={(e) => setStartDate(e.target.value)}
                  className="w-full rounded-md border border-gray-300 px-3 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
              </div>
              <div>
                <label htmlFor="endDate" className="block text-xs text-gray-500 mb-1">
                  {t('meters.export.endDate')}
                </label>
                <input
                  type="date"
                  id="endDate"
                  value={endDate}
                  onChange={(e) => setEndDate(e.target.value)}
                  max={new Date().toISOString().split('T')[0]}
                  className="w-full rounded-md border border-gray-300 px-3 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
              </div>
            </div>
          </div>

          {/* Compare Button */}
          <button
            type="button"
            onClick={handleCompare}
            disabled={selectedMeterIds.length === 0 || isLoading}
            className="w-full px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
          >
            {isLoading && (
              <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-white" />
            )}
            {t('meters.comparison.compare')}
          </button>
        </div>

        {/* Chart Area */}
        <div className="lg:col-span-3">
          <div className="bg-white rounded-lg shadow p-6">
            {/* Chart Controls */}
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-lg font-semibold text-gray-900">
                {t('meters.comparison.chartTitle')}
              </h2>
              <div className="flex items-center gap-1 bg-gray-100 rounded-lg p-1">
                <button
                  type="button"
                  onClick={() => setChartView('overlay')}
                  className={`px-3 py-1.5 text-sm rounded-md transition-colors ${
                    chartView === 'overlay'
                      ? 'bg-white shadow text-gray-900'
                      : 'text-gray-600 hover:text-gray-900'
                  }`}
                >
                  {t('meters.comparison.overlay')}
                </button>
                <button
                  type="button"
                  onClick={() => setChartView('sideBySide')}
                  className={`px-3 py-1.5 text-sm rounded-md transition-colors ${
                    chartView === 'sideBySide'
                      ? 'bg-white shadow text-gray-900'
                      : 'text-gray-600 hover:text-gray-900'
                  }`}
                >
                  {t('meters.comparison.sideBySide')}
                </button>
              </div>
            </div>

            {/* Loading State */}
            {isLoading && (
              <div
                className="flex items-center justify-center"
                style={{ height: `${chartHeight}px` }}
              >
                <div className="animate-spin rounded-full h-10 w-10 border-b-2 border-blue-600" />
              </div>
            )}

            {/* Empty State */}
            {!isLoading && comparisonData.length === 0 && (
              <div
                className="flex flex-col items-center justify-center text-gray-500"
                style={{ height: `${chartHeight}px` }}
              >
                <svg
                  className="w-12 h-12 mb-2 text-gray-300"
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
                <p>{t('meters.comparison.selectToCompare')}</p>
              </div>
            )}

            {/* Overlay Chart */}
            {!isLoading && comparisonData.length > 0 && chartView === 'overlay' && (
              <div className="relative" style={{ height: `${chartHeight}px` }}>
                {/* Y-axis labels */}
                <div
                  className="absolute left-0 top-0 flex flex-col justify-between text-xs text-gray-400"
                  style={{
                    height: `${chartHeight - padding.top - padding.bottom}px`,
                    paddingTop: `${padding.top}px`,
                    width: `${padding.left}px`,
                  }}
                >
                  {yLabels.reverse().map((label, i) => (
                    <span key={i} className="text-right pr-2">
                      {formatValue(label.value)}
                    </span>
                  ))}
                </div>

                {/* Chart */}
                <div
                  className="absolute"
                  style={{
                    left: `${padding.left}px`,
                    top: `${padding.top}px`,
                    right: `${padding.right}px`,
                    height: `${chartHeight - padding.top - padding.bottom}px`,
                  }}
                >
                  {/* Grid lines */}
                  <svg className="absolute inset-0 w-full h-full" preserveAspectRatio="none">
                    {yLabels.map((label, i) => (
                      <line
                        key={i}
                        x1="0%"
                        y1={`${label.y}%`}
                        x2="100%"
                        y2={`${label.y}%`}
                        stroke="var(--ppt-border-default)"
                        strokeWidth="1"
                      />
                    ))}
                  </svg>

                  {/* Lines for each meter */}
                  <svg
                    className="absolute inset-0 w-full h-full"
                    viewBox="0 0 100 100"
                    preserveAspectRatio="none"
                  >
                    {comparisonData.map((meterData, meterIndex) => {
                      const points = meterData.dataPoints.map((d) => {
                        const dateIndex = allDates.indexOf(d.date);
                        const x =
                          allDates.length > 1 ? (dateIndex / (allDates.length - 1)) * 100 : 50;
                        const y = 100 - ((d.value - paddedMin) / paddedRange) * 100;
                        return { x, y };
                      });

                      const linePath = points
                        .map((p, i) => `${i === 0 ? 'M' : 'L'} ${p.x} ${p.y}`)
                        .join(' ');

                      return (
                        <path
                          key={meterData.meterId}
                          d={linePath}
                          fill="none"
                          stroke={meterData.color || CHART_COLORS[meterIndex % CHART_COLORS.length]}
                          strokeWidth="2"
                          vectorEffect="non-scaling-stroke"
                        />
                      );
                    })}
                  </svg>
                </div>

                {/* X-axis labels */}
                <div
                  className="absolute flex justify-between text-xs text-gray-400"
                  style={{
                    left: `${padding.left}px`,
                    right: `${padding.right}px`,
                    bottom: '8px',
                  }}
                >
                  {allDates.length <= 6
                    ? allDates.map((date) => <span key={date}>{formatDate(date)}</span>)
                    : [0, Math.floor(allDates.length / 2), allDates.length - 1].map((i) => (
                        <span key={i}>{formatDate(allDates[i])}</span>
                      ))}
                </div>
              </div>
            )}

            {/* Side by Side Charts */}
            {!isLoading && comparisonData.length > 0 && chartView === 'sideBySide' && (
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                {comparisonData.map((meterData, meterIndex) => {
                  const meterValues = meterData.dataPoints.map((d) => d.value);
                  const meterMin = Math.min(...meterValues);
                  const meterMax = Math.max(...meterValues);
                  const meterRange = meterMax - meterMin || 1;
                  const meterPaddedMin = meterMin - meterRange * 0.1;
                  const meterPaddedMax = meterMax + meterRange * 0.1;
                  const meterPaddedRange = meterPaddedMax - meterPaddedMin;

                  const points = meterData.dataPoints.map((d, i) => {
                    const x =
                      meterData.dataPoints.length > 1
                        ? (i / (meterData.dataPoints.length - 1)) * 100
                        : 50;
                    const y = 100 - ((d.value - meterPaddedMin) / meterPaddedRange) * 100;
                    return { x, y };
                  });

                  const linePath = points
                    .map((p, i) => `${i === 0 ? 'M' : 'L'} ${p.x} ${p.y}`)
                    .join(' ');

                  const color = meterData.color || CHART_COLORS[meterIndex % CHART_COLORS.length];

                  return (
                    <div key={meterData.meterId} className="border rounded-lg p-4">
                      <div className="flex items-center gap-2 mb-2">
                        <div className="w-3 h-3 rounded-full" style={{ backgroundColor: color }} />
                        <span className="text-sm font-medium text-gray-900">
                          {meterData.meterName}
                        </span>
                      </div>
                      <div className="relative" style={{ height: '150px' }}>
                        <svg
                          className="w-full h-full"
                          viewBox="0 0 100 100"
                          preserveAspectRatio="none"
                        >
                          <defs>
                            <linearGradient
                              id={`gradient-${meterData.meterId}`}
                              x1="0%"
                              y1="0%"
                              x2="0%"
                              y2="100%"
                            >
                              <stop offset="0%" stopColor={color} stopOpacity="0.3" />
                              <stop offset="100%" stopColor={color} stopOpacity="0" />
                            </linearGradient>
                          </defs>
                          <path
                            d={`${linePath} L ${points[points.length - 1].x} 100 L ${points[0].x} 100 Z`}
                            fill={`url(#gradient-${meterData.meterId})`}
                          />
                          <path
                            d={linePath}
                            fill="none"
                            stroke={color}
                            strokeWidth="2"
                            vectorEffect="non-scaling-stroke"
                          />
                        </svg>
                      </div>
                      <div className="mt-2 text-xs text-gray-500">
                        {meterData.dataPoints.length} {t('meters.readings')} | {meterData.unit}
                      </div>
                    </div>
                  );
                })}
              </div>
            )}

            {/* Legend */}
            {!isLoading && comparisonData.length > 0 && (
              <div className="mt-4 pt-4 border-t flex flex-wrap gap-4">
                {comparisonData.map((meterData, index) => (
                  <div key={meterData.meterId} className="flex items-center gap-2">
                    <div
                      className="w-3 h-3 rounded-full"
                      style={{
                        backgroundColor:
                          meterData.color || CHART_COLORS[index % CHART_COLORS.length],
                      }}
                    />
                    <span className="text-sm text-gray-600">
                      {meterData.meterName} ({meterData.unit})
                    </span>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
