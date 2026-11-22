// Epic 144: Portfolio Performance Analytics - Metrics Summary Card Component
import type React from 'react';
import { getActiveLocale } from '../../../lib/locale';

interface MetricsSummaryCardProps {
  title: string;
  value: number | string;
  subtitle?: string;
  change?: number;
  changeLabel?: string;
  format?: 'currency' | 'percent' | 'number' | 'ratio';
  currency?: string;
  icon?: React.ReactNode;
  trend?: 'up' | 'down' | 'neutral';
}

export const MetricsSummaryCard: React.FC<MetricsSummaryCardProps> = ({
  title,
  value,
  subtitle,
  change,
  changeLabel,
  format = 'number',
  currency = 'EUR',
  icon,
  trend,
}) => {
  const formatValue = (val: number | string) => {
    if (typeof val === 'string') return val;

    switch (format) {
      case 'currency':
        return new Intl.NumberFormat('en-US', {
          style: 'currency',
          currency: currency,
          minimumFractionDigits: 0,
          maximumFractionDigits: 0,
        }).format(val);
      case 'percent':
        return `${val.toFixed(2)}%`;
      case 'ratio':
        return `${val.toFixed(2)}x`;
      default:
        return val.toLocaleString(getActiveLocale());
    }
  };

  const getTrendColor = () => {
    if (!trend) return 'text-gray-500';
    switch (trend) {
      case 'up':
        return 'text-green-600';
      case 'down':
        return 'text-red-600';
      default:
        return 'text-gray-500';
    }
  };

  const getTrendIcon = () => {
    if (!trend) return null;
    switch (trend) {
      case 'up':
        return (
          <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
            <path
              fillRule="evenodd"
              d="M5.293 9.707a1 1 0 010-1.414l4-4a1 1 0 011.414 0l4 4a1 1 0 01-1.414 1.414L11 7.414V15a1 1 0 11-2 0V7.414L6.707 9.707a1 1 0 01-1.414 0z"
              clipRule="evenodd"
            />
          </svg>
        );
      case 'down':
        return (
          <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
            <path
              fillRule="evenodd"
              d="M14.707 10.293a1 1 0 010 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 111.414-1.414L9 12.586V5a1 1 0 012 0v7.586l2.293-2.293a1 1 0 011.414 0z"
              clipRule="evenodd"
            />
          </svg>
        );
      default:
        return (
          <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
            <path
              fillRule="evenodd"
              d="M5 10a1 1 0 011-1h8a1 1 0 110 2H6a1 1 0 01-1-1z"
              clipRule="evenodd"
            />
          </svg>
        );
    }
  };

  return (
    <div className="bg-white rounded-lg shadow-md p-6">
      <div className="flex items-center justify-between mb-2">
        <p className="text-sm font-medium text-gray-500">{title}</p>
        {icon && <div className="text-gray-400">{icon}</div>}
      </div>

      <p className="text-2xl font-bold text-gray-900">{formatValue(value)}</p>

      {subtitle && <p className="text-sm text-gray-500 mt-1">{subtitle}</p>}

      {change !== undefined && (
        <div className={`flex items-center mt-2 ${getTrendColor()}`}>
          {getTrendIcon()}
          <span className="text-sm font-medium ml-1">
            {change >= 0 ? '+' : ''}
            {change.toFixed(2)}%
          </span>
          {changeLabel && <span className="text-sm text-gray-500 ml-1">{changeLabel}</span>}
        </div>
      )}
    </div>
  );
};
