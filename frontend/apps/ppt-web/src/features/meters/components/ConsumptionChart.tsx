/**
 * ConsumptionChart component - chart showing consumption history.
 * Meters feature: Self-readings and consumption tracking.
 */

import { useTranslation } from 'react-i18next';
import type { ConsumptionHistory, MeterUnit } from '../types';

interface ConsumptionChartProps {
  data: ConsumptionHistory;
  height?: number;
  isLoading?: boolean;
  showTrend?: boolean;
}

function formatValue(value: number, unit: MeterUnit): string {
  if (value >= 1000) {
    return `${(value / 1000).toFixed(1)}k ${unit}`;
  }
  return `${value.toFixed(1)} ${unit}`;
}

function formatShortValue(value: number): string {
  if (value >= 1000) {
    return `${(value / 1000).toFixed(1)}k`;
  }
  return value.toFixed(0);
}

export function ConsumptionChart({
  data,
  height = 250,
  isLoading,
  showTrend = true,
}: ConsumptionChartProps) {
  const { t, i18n } = useTranslation();

  if (isLoading) {
    return (
      <div className="bg-white rounded-lg shadow p-6">
        <div className="h-6 bg-gray-200 rounded w-1/3 mb-4 animate-pulse" />
        <div className="animate-pulse bg-gray-200 rounded" style={{ height: `${height}px` }} />
      </div>
    );
  }

  if (data.data.length === 0) {
    return (
      <div className="bg-white rounded-lg shadow p-6">
        <h3 className="text-lg font-medium text-gray-900 mb-4">{t('meters.consumptionHistory')}</h3>
        <div
          className="flex items-center justify-center text-gray-500"
          style={{ height: `${height}px` }}
        >
          {t('meters.noConsumptionData')}
        </div>
      </div>
    );
  }

  const maxValue = Math.max(...data.data.map((d) => d.value), 1);
  const chartWidth = 100;

  // Y-axis labels
  const yAxisLabels = [0, 0.25, 0.5, 0.75, 1].map((ratio) => ({
    value: maxValue * ratio,
    position: 100 - ratio * 100,
  }));

  // Trend indicator
  const trendColor =
    data.trend === 'up'
      ? 'text-red-600'
      : data.trend === 'down'
        ? 'text-green-600'
        : 'text-gray-500';

  const trendIcon =
    data.trend === 'up'
      ? 'M5 10l7-7m0 0l7 7m-7-7v18'
      : data.trend === 'down'
        ? 'M19 14l-7 7m0 0l-7-7m7 7V3'
        : 'M4 12h16';

  return (
    <div className="bg-white rounded-lg shadow p-6">
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-lg font-medium text-gray-900">{t('meters.consumptionHistory')}</h3>
        {showTrend && data.trend && data.changePercentage !== undefined && (
          <div className={`flex items-center gap-1 ${trendColor}`}>
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d={trendIcon} />
            </svg>
            <span className="text-sm font-medium">
              {data.changePercentage > 0 ? '+' : ''}
              {data.changePercentage.toFixed(1)}%
            </span>
          </div>
        )}
      </div>

      {/* Stats */}
      <div className="grid grid-cols-2 gap-4 mb-4">
        <div className="bg-gray-50 rounded-lg p-3">
          <p className="text-xs text-gray-500">{t('meters.totalConsumption')}</p>
          <p className="text-lg font-bold text-gray-900">
            {formatValue(data.totalConsumption, data.unit)}
          </p>
        </div>
        <div className="bg-gray-50 rounded-lg p-3">
          <p className="text-xs text-gray-500">{t('meters.averageConsumption')}</p>
          <p className="text-lg font-bold text-gray-900">
            {formatValue(data.averageConsumption, data.unit)}
          </p>
        </div>
      </div>

      {/* Chart */}
      <div className="relative" style={{ height: `${height}px` }}>
        {/* Y-axis */}
        <div className="absolute left-0 top-0 bottom-0 w-10 flex flex-col justify-between text-xs text-gray-400">
          {yAxisLabels.map((label) => (
            <span key={label.position} style={{ transform: 'translateY(-50%)' }}>
              {formatShortValue(label.value)}
            </span>
          ))}
        </div>

        {/* Chart Area */}
        <div className="ml-12 relative h-full">
          {/* Grid lines */}
          <div className="absolute inset-0">
            {yAxisLabels.map((label) => (
              <div
                key={label.position}
                className="absolute left-0 right-0 border-t border-gray-100"
                style={{ top: `${label.position}%` }}
              />
            ))}
          </div>

          {/* Bars */}
          <svg className="w-full h-full" preserveAspectRatio="xMidYMid meet" aria-hidden="true">
            {data.data.map((point, index) => {
              const barWidth = (chartWidth / data.data.length) * 0.7;
              const barX = (index / data.data.length) * chartWidth + barWidth * 0.2;
              const barHeight = (point.value / maxValue) * 100;

              return (
                <g key={point.date}>
                  <rect
                    x={`${barX}%`}
                    y={`${100 - barHeight}%`}
                    width={`${barWidth}%`}
                    height={`${barHeight}%`}
                    fill="#3b82f6"
                    opacity={0.8}
                    rx="2"
                    className="hover:opacity-100 transition-opacity"
                  >
                    <title>
                      {point.label || point.date}: {formatValue(point.value, data.unit)}
                    </title>
                  </rect>
                </g>
              );
            })}
          </svg>

          {/* X-axis labels */}
          <div className="absolute bottom-0 left-0 right-0 flex justify-between text-xs text-gray-400 transform translate-y-full pt-2">
            {data.data
              .filter(
                (_, i, arr) =>
                  i === 0 || i === arr.length - 1 || i % Math.ceil(arr.length / 6) === 0
              )
              .map((point) => (
                <span key={point.date}>
                  {point.label ||
                    new Date(point.date).toLocaleDateString(i18n.language, {
                      month: 'short',
                    })}
                </span>
              ))}
          </div>
        </div>
      </div>
    </div>
  );
}
