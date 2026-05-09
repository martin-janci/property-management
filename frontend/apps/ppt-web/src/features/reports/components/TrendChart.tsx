/**
 * TrendChart component - Story 53.4
 *
 * Time series chart for trend analysis with period comparison.
 */

import type { TrendAnalysis, TrendLine } from '@ppt/api-client';
import { useMemo } from 'react';

interface TrendChartProps {
  title: string;
  analysis: TrendAnalysis;
  trendLines: TrendLine[];
  onPeriodChange?: (period: 'daily' | 'weekly' | 'monthly' | 'quarterly' | 'yearly') => void;
  selectedPeriod?: string;
  isLoading?: boolean;
}

function formatValue(value: number): string {
  if (value >= 1000000) {
    return `${(value / 1000000).toFixed(1)}M`;
  }
  if (value >= 1000) {
    return `${(value / 1000).toFixed(1)}K`;
  }
  return value.toFixed(0);
}

const PERIODS = [
  { value: 'daily', label: 'Daily' },
  { value: 'weekly', label: 'Weekly' },
  { value: 'monthly', label: 'Monthly' },
  { value: 'quarterly', label: 'Quarterly' },
  { value: 'yearly', label: 'Yearly' },
];

const COLORS = ['#3b82f6', '#10b981', '#f59e0b', '#ef4444'];

export function TrendChart({
  title,
  analysis,
  trendLines,
  onPeriodChange,
  selectedPeriod = 'monthly',
  isLoading,
}: TrendChartProps) {
  if (isLoading) {
    return (
      <div className="bg-white rounded-lg shadow p-6">
        <div className="animate-pulse">
          <div className="h-6 bg-gray-200 rounded w-1/3 mb-4" />
          <div className="h-64 bg-gray-200 rounded" />
        </div>
      </div>
    );
  }

  // Memoize maxValue calculation to prevent recalculation on every render
  const maxValue = useMemo(() => {
    let max = 1;
    for (const line of trendLines) {
      for (const point of line.data) {
        if (point.value > max) {
          max = point.value;
        }
      }
    }
    return max;
  }, [trendLines]);

  const hasAnomalies = analysis.anomalies.length > 0;

  return (
    <div className="bg-white rounded-lg shadow overflow-hidden">
      {/* Header */}
      <div className="px-6 py-4 border-b border-gray-200">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-lg font-medium text-gray-900">{title}</h3>
            <p className="text-sm text-gray-500 mt-1">
              {analysis.metric} - {analysis.period}
            </p>
          </div>

          {/* Period Selector */}
          {onPeriodChange && (
            <div className="flex gap-1">
              {PERIODS.map((period) => (
                <button
                  key={period.value}
                  type="button"
                  onClick={() =>
                    onPeriodChange(
                      period.value as 'daily' | 'weekly' | 'monthly' | 'quarterly' | 'yearly'
                    )
                  }
                  className={`px-3 py-1 text-sm rounded-md transition-colors ${
                    selectedPeriod === period.value
                      ? 'bg-blue-100 text-blue-700'
                      : 'text-gray-600 hover:bg-gray-100'
                  }`}
                >
                  {period.label}
                </button>
              ))}
            </div>
          )}
        </div>
      </div>

      {/* Summary Stats */}
      <div className="grid grid-cols-4 divide-x divide-gray-100 border-b border-gray-100">
        <div className="p-4 text-center">
          <p className="text-sm text-gray-500">Current</p>
          <p className="text-xl font-bold text-gray-900">{formatValue(analysis.current_value)}</p>
        </div>
        <div className="p-4 text-center">
          <p className="text-sm text-gray-500">Previous</p>
          <p className="text-xl font-bold text-gray-900">{formatValue(analysis.previous_value)}</p>
        </div>
        <div className="p-4 text-center">
          <p className="text-sm text-gray-500">Change</p>
          <p
            className={`text-xl font-bold ${
              analysis.change >= 0 ? 'text-green-600' : 'text-red-600'
            }`}
          >
            {analysis.change >= 0 ? '+' : ''}
            {analysis.change_percentage.toFixed(1)}%
          </p>
        </div>
        <div className="p-4 text-center">
          <p className="text-sm text-gray-500">Forecast</p>
          <p className="text-xl font-bold text-gray-900">
            {analysis.forecast ? formatValue(analysis.forecast) : '-'}
          </p>
        </div>
      </div>

      {/* Chart */}
      <div className="p-6">
        <div className="relative h-64">
          {/* Y-axis labels */}
          <div className="absolute left-0 top-0 bottom-0 w-12 flex flex-col justify-between text-xs text-gray-400">
            {[1, 0.75, 0.5, 0.25, 0].map((ratio) => (
              <span key={ratio}>{formatValue(maxValue * ratio)}</span>
            ))}
          </div>

          {/* Chart area */}
          <div className="ml-14 relative h-full">
            {/* Grid */}
            <div className="absolute inset-0">
              {[0, 25, 50, 75, 100].map((pos) => (
                <div
                  key={pos}
                  className="absolute left-0 right-0 border-t border-gray-100"
                  style={{ top: `${pos}%` }}
                />
              ))}
            </div>

            {/* SVG */}
            <svg
              className="w-full h-full"
              preserveAspectRatio="none"
              role="img"
              aria-label={`${title} trend chart showing ${trendLines.map((line) => line.name).join(', ')}`}
            >
              {trendLines.map((line, lineIndex) => {
                const color = line.color || COLORS[lineIndex % COLORS.length];
                const dataLength = line.data.length;
                const points = line.data.map((point, i) => ({
                  x: dataLength > 1 ? (i / (dataLength - 1)) * 100 : 50,
                  y: 100 - (point.value / maxValue) * 100,
                  ...point,
                }));

                // Trend line path
                const pathD = points.reduce(
                  (path, p, i) => (i === 0 ? `M ${p.x}% ${p.y}%` : `${path} L ${p.x}% ${p.y}%`),
                  ''
                );

                // Area fill
                const areaD = `${pathD} L 100% 100% L 0% 100% Z`;

                return (
                  <g key={line.id}>
                    <path d={areaD} fill={color} fillOpacity={0.1} />
                    <path d={pathD} fill="none" stroke={color} strokeWidth="2" />
                    {points.map((p, i) => (
                      <circle
                        key={`${line.id}-${i}`}
                        cx={`${p.x}%`}
                        cy={`${p.y}%`}
                        r="3"
                        fill={color}
                      >
                        <title>{`${p.date}: ${formatValue(p.value)}`}</title>
                      </circle>
                    ))}
                  </g>
                );
              })}

              {/* Anomaly markers */}
              {analysis.anomalies.map((anomaly) => {
                const lineData = trendLines[0]?.data || [];
                const index = lineData.findIndex((p) => p.date === anomaly.date);
                if (index === -1) return null;

                const x = (index / (lineData.length - 1 || 1)) * 100;
                const y = 100 - (anomaly.value / maxValue) * 100;

                return (
                  <g key={`anomaly-${anomaly.date}`}>
                    <circle
                      cx={`${x}%`}
                      cy={`${y}%`}
                      r="6"
                      fill="none"
                      stroke="var(--ppt-color-danger)"
                      strokeWidth="2"
                    />
                    <circle cx={`${x}%`} cy={`${y}%`} r="3" fill="var(--ppt-color-danger)">
                      <title>Anomaly: {anomaly.date}</title>
                    </circle>
                  </g>
                );
              })}
            </svg>

            {/* X-axis labels */}
            <div className="absolute -bottom-6 left-0 right-0 flex justify-between text-xs text-gray-400">
              {trendLines[0]?.data
                .filter(
                  (_, i, arr) =>
                    i === 0 || i === arr.length - 1 || i % Math.ceil(arr.length / 5) === 0
                )
                .map((p) => (
                  <span key={p.date}>
                    {new Date(p.date).toLocaleDateString('en-US', {
                      month: 'short',
                      day: 'numeric',
                    })}
                  </span>
                ))}
            </div>
          </div>
        </div>

        {/* Legend */}
        <div className="mt-8 flex items-center justify-center gap-6">
          {trendLines.map((line, index) => (
            <div key={line.id} className="flex items-center gap-2">
              <span
                className="w-3 h-3 rounded-full"
                style={{ backgroundColor: line.color || COLORS[index % COLORS.length] }}
              />
              <span className="text-sm text-gray-600">{line.name}</span>
            </div>
          ))}
          {hasAnomalies && (
            <div className="flex items-center gap-2">
              <span className="w-3 h-3 rounded-full border-2 border-red-500" />
              <span className="text-sm text-gray-600">Anomalies ({analysis.anomalies.length})</span>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
