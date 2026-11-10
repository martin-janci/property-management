/**
 * ReadingLineChart component - SVG line chart showing readings over time.
 * Meters feature: Self-readings and consumption tracking.
 */

import { useTranslation } from 'react-i18next';
import type { MeterUnit } from '../types';

interface DataPoint {
  date: string;
  value: number;
  consumption?: number;
}

interface ReadingLineChartProps {
  data: DataPoint[];
  unit: MeterUnit;
  height?: number;
  showConsumption?: boolean;
}

export function ReadingLineChart({
  data,
  unit,
  height = 300,
  showConsumption = false,
}: ReadingLineChartProps) {
  const { t, i18n } = useTranslation();

  if (data.length === 0) {
    return (
      <div
        className="flex items-center justify-center text-gray-500"
        style={{ height: `${height}px` }}
      >
        {t('meters.noReadings')}
      </div>
    );
  }

  // Chart dimensions
  const padding = { top: 20, right: 20, bottom: 40, left: 60 };
  const chartHeight = height - padding.top - padding.bottom;

  // Calculate min/max values
  const values = data.map((d) => d.value);
  const minValue = Math.min(...values);
  const maxValue = Math.max(...values);
  const valueRange = maxValue - minValue || 1;

  // Add 10% padding to value range
  const paddedMin = minValue - valueRange * 0.1;
  const paddedMax = maxValue + valueRange * 0.1;
  const paddedRange = paddedMax - paddedMin;

  // Create points for SVG polyline
  const points = data.map((d, i) => {
    const x = (i / (data.length - 1)) * 100;
    const y = 100 - ((d.value - paddedMin) / paddedRange) * 100;
    return { x, y, ...d };
  });

  // Y-axis labels (5 labels)
  const yLabels = Array.from({ length: 5 }, (_, i) => {
    const value = paddedMin + (paddedRange * i) / 4;
    return {
      value,
      y: 100 - (i / 4) * 100,
    };
  });

  // X-axis labels (show max 6 dates)
  const xLabelIndices: number[] = [];
  if (data.length <= 6) {
    xLabelIndices.push(...data.map((_, i) => i));
  } else {
    const step = (data.length - 1) / 5;
    for (let i = 0; i < 6; i++) {
      xLabelIndices.push(Math.round(i * step));
    }
  }

  const formatValue = (value: number): string => {
    if (value >= 10000) {
      return `${(value / 1000).toFixed(0)}k`;
    }
    if (value >= 1000) {
      return `${(value / 1000).toFixed(1)}k`;
    }
    return value.toFixed(0);
  };

  const formatDate = (dateStr: string): string => {
    const date = new Date(dateStr);
    return date.toLocaleDateString(i18n.language, { month: 'short', day: 'numeric' });
  };

  // Create SVG path for the line
  const linePath = points.map((p, i) => `${i === 0 ? 'M' : 'L'} ${p.x} ${p.y}`).join(' ');

  // Create SVG path for the area (gradient fill)
  const areaPath = `${linePath} L ${points[points.length - 1].x} 100 L ${points[0].x} 100 Z`;

  return (
    <div className="relative" style={{ height: `${height}px` }}>
      {/* Y-axis labels */}
      <div
        className="absolute left-0 top-0 flex flex-col justify-between text-xs text-gray-400"
        style={{
          height: `${chartHeight}px`,
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

      {/* Chart Area */}
      <div
        className="absolute"
        style={{
          left: `${padding.left}px`,
          top: `${padding.top}px`,
          right: `${padding.right}px`,
          height: `${chartHeight}px`,
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

        {/* Line chart */}
        <svg
          className="absolute inset-0 w-full h-full"
          viewBox="0 0 100 100"
          preserveAspectRatio="none"
        >
          {/* Gradient definition */}
          <defs>
            <linearGradient id="lineGradient" x1="0%" y1="0%" x2="0%" y2="100%">
              <stop offset="0%" stopColor="#3b82f6" stopOpacity="0.3" />
              <stop offset="100%" stopColor="#3b82f6" stopOpacity="0" />
            </linearGradient>
          </defs>

          {/* Area fill */}
          <path d={areaPath} fill="url(#lineGradient)" />

          {/* Line */}
          <path
            d={linePath}
            fill="none"
            stroke="#3b82f6"
            strokeWidth="2"
            vectorEffect="non-scaling-stroke"
          />
        </svg>

        {/* Data points */}
        <svg
          className="absolute inset-0 w-full h-full"
          viewBox="0 0 100 100"
          preserveAspectRatio="none"
        >
          {points.map((point, i) => (
            <g key={i}>
              <circle
                cx={`${point.x}`}
                cy={`${point.y}`}
                r="1.5"
                fill="#3b82f6"
                className="cursor-pointer"
                vectorEffect="non-scaling-stroke"
              >
                <title>
                  {formatDate(point.date)}: {point.value.toLocaleString()} {unit}
                  {point.consumption !== undefined &&
                    point.consumption >= 0 &&
                    `\n${t('meters.consumption')}: +${point.consumption.toLocaleString()} ${unit}`}
                </title>
              </circle>
            </g>
          ))}
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
        {xLabelIndices.map((index) => (
          <span key={index}>{formatDate(data[index].date)}</span>
        ))}
      </div>

      {/* Legend */}
      <div className="absolute top-0 right-0 flex items-center gap-4 text-xs text-gray-500">
        <div className="flex items-center gap-1">
          <div className="w-3 h-0.5 bg-blue-500" />
          <span>{t('meters.value')}</span>
        </div>
        {showConsumption && (
          <div className="flex items-center gap-1">
            <div className="w-3 h-0.5 bg-green-500" />
            <span>{t('meters.consumption')}</span>
          </div>
        )}
      </div>
    </div>
  );
}
