// Epic 144: Portfolio Performance Analytics - Performance Alert Component
import type React from 'react';
import { useTranslation } from 'react-i18next';

interface PerformanceAlertProps {
  id: string;
  alertType: string;
  severity: string;
  title: string;
  message: string;
  metricName?: string;
  currentValue?: number;
  thresholdValue?: number;
  isRead: boolean;
  isResolved: boolean;
  createdAt: string;
  onMarkRead?: () => void;
  onResolve?: () => void;
}

export const PerformanceAlert: React.FC<PerformanceAlertProps> = ({
  alertType,
  severity,
  title,
  message,
  metricName,
  currentValue,
  thresholdValue,
  isRead,
  isResolved,
  createdAt,
  onMarkRead,
  onResolve,
}) => {
  const { t } = useTranslation();

  const getSeverityStyles = () => {
    switch (severity.toLowerCase()) {
      case 'critical':
        return {
          bg: 'bg-red-50',
          border: 'border-red-400',
          icon: 'text-red-600',
          badge: 'bg-red-100 text-red-800',
        };
      case 'warning':
        return {
          bg: 'bg-yellow-50',
          border: 'border-yellow-400',
          icon: 'text-yellow-600',
          badge: 'bg-yellow-100 text-yellow-800',
        };
      case 'info':
        return {
          bg: 'bg-blue-50',
          border: 'border-blue-400',
          icon: 'text-blue-600',
          badge: 'bg-blue-100 text-blue-800',
        };
      default:
        return {
          bg: 'bg-gray-50',
          border: 'border-gray-400',
          icon: 'text-gray-600',
          badge: 'bg-gray-100 text-gray-800',
        };
    }
  };

  const styles = getSeverityStyles();

  const formatDate = (dateStr: string) => {
    const date = new Date(dateStr);
    return date.toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  const formatAlertType = (type: string) => {
    return type.replace(/_/g, ' ').replace(/\b\w/g, (l) => l.toUpperCase());
  };

  return (
    <div
      className={`${styles.bg} border-l-4 ${styles.border} p-4 rounded-r-lg ${
        !isRead ? 'ring-2 ring-offset-2 ring-blue-500' : ''
      } ${isResolved ? 'opacity-60' : ''}`}
    >
      <div className="flex justify-between items-start">
        <div className="flex items-start space-x-3">
          {/* Icon */}
          <div className={`${styles.icon} mt-0.5`}>
            {severity === 'critical' ? (
              <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
                <path
                  fillRule="evenodd"
                  d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z"
                  clipRule="evenodd"
                />
              </svg>
            ) : severity === 'warning' ? (
              <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
                <path
                  fillRule="evenodd"
                  d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z"
                  clipRule="evenodd"
                />
              </svg>
            ) : (
              <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
                <path
                  fillRule="evenodd"
                  d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z"
                  clipRule="evenodd"
                />
              </svg>
            )}
          </div>

          {/* Content */}
          <div className="flex-1">
            <div className="flex items-center space-x-2">
              <h4 className="text-sm font-semibold text-gray-900">{title}</h4>
              <span className={`px-2 py-0.5 text-xs rounded-full ${styles.badge}`}>
                {formatAlertType(alertType)}
              </span>
              {isResolved && (
                <span className="px-2 py-0.5 text-xs rounded-full bg-green-100 text-green-800">
                  {t('portfolio.alert.resolved')}
                </span>
              )}
            </div>
            <p className="text-sm text-gray-600 mt-1">{message}</p>

            {/* Metric details */}
            {metricName && (
              <div className="flex items-center space-x-4 mt-2 text-xs text-gray-500">
                <span>{t('portfolio.alert.metric', { name: metricName })}</span>
                {currentValue !== undefined && (
                  <span>{t('portfolio.alert.current', { value: currentValue.toFixed(2) })}</span>
                )}
                {thresholdValue !== undefined && (
                  <span>
                    {t('portfolio.alert.threshold', { value: thresholdValue.toFixed(2) })}
                  </span>
                )}
              </div>
            )}

            <p className="text-xs text-gray-400 mt-2">{formatDate(createdAt)}</p>
          </div>
        </div>

        {/* Actions */}
        {(!isRead || !isResolved) && (
          <div className="flex items-center space-x-2 ml-4">
            {!isRead && onMarkRead && (
              <button
                type="button"
                onClick={onMarkRead}
                className="text-xs text-blue-600 hover:text-blue-800 font-medium"
              >
                {t('portfolio.alert.markRead')}
              </button>
            )}
            {!isResolved && onResolve && (
              <button
                type="button"
                onClick={onResolve}
                className="text-xs text-green-600 hover:text-green-800 font-medium"
              >
                {t('portfolio.alert.resolve')}
              </button>
            )}
          </div>
        )}
      </div>
    </div>
  );
};
