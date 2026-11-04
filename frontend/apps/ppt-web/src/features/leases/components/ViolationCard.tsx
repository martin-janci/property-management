/**
 * ViolationCard component - displays a violation summary card in lists.
 * UC-34: Lease Violations Tracking
 */

import { useTranslation } from 'react-i18next';
import type { ViolationSeverity, ViolationStatus, ViolationSummary } from '../types';

interface ViolationCardProps {
  violation: ViolationSummary;
  onView?: (id: string) => void;
  onResolve?: (id: string) => void;
  onDispute?: (id: string) => void;
}

const statusColors: Record<ViolationStatus, string> = {
  open: 'bg-red-100 text-red-800',
  resolved: 'bg-green-100 text-green-800',
  disputed: 'bg-yellow-100 text-yellow-800',
  escalated: 'bg-purple-100 text-purple-800',
  dismissed: 'bg-gray-100 text-gray-800',
};

const severityColors: Record<ViolationSeverity, string> = {
  minor: 'bg-blue-100 text-blue-800',
  moderate: 'bg-orange-100 text-orange-800',
  severe: 'bg-red-100 text-red-800',
};

const severityIcons: Record<ViolationSeverity, React.ReactNode> = {
  minor: (
    <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20" aria-hidden="true">
      <path
        fillRule="evenodd"
        d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z"
        clipRule="evenodd"
      />
    </svg>
  ),
  moderate: (
    <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20" aria-hidden="true">
      <path
        fillRule="evenodd"
        d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z"
        clipRule="evenodd"
      />
    </svg>
  ),
  severe: (
    <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20" aria-hidden="true">
      <path
        fillRule="evenodd"
        d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z"
        clipRule="evenodd"
      />
    </svg>
  ),
};

export function ViolationCard({ violation, onView, onResolve, onDispute }: ViolationCardProps) {
  const { t } = useTranslation();

  const canResolve = violation.status === 'open' || violation.status === 'disputed';
  const canDispute = violation.status === 'open';

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString();
  };

  return (
    <div className="bg-white rounded-lg shadow p-4 hover:shadow-md transition-shadow">
      <div className="flex items-start justify-between">
        <div className="flex-1">
          <div className="flex items-center gap-2">
            <span
              className={severityColors[violation.severity]}
              title={t(`leases.violations.severity.${violation.severity}`)}
            >
              {severityIcons[violation.severity]}
            </span>
            <h3 className="text-lg font-semibold text-gray-900">
              {t(`leases.violations.type.${violation.violationType}`)}
            </h3>
          </div>
          <p className="text-sm text-gray-600 mt-1">
            {violation.buildingName} - {violation.unitNumber}
          </p>
          <p className="text-xs text-gray-500">{violation.tenantName}</p>
          <div className="mt-2 flex items-center gap-2 flex-wrap">
            <span
              className={`px-2 py-1 text-xs font-medium rounded ${statusColors[violation.status]}`}
            >
              {t(`leases.violations.status.${violation.status}`)}
            </span>
            <span
              className={`px-2 py-1 text-xs font-medium rounded ${severityColors[violation.severity]}`}
            >
              {t(`leases.violations.severity.${violation.severity}`)}
            </span>
          </div>
          <div className="mt-2 text-xs text-gray-500">
            <span>
              {t('leases.violations.violationDate')}: {formatDate(violation.violationDate)}
            </span>
            <span className="mx-2">|</span>
            <span>
              {t('leases.violations.reportedAt')}: {formatDate(violation.reportedAt)}
            </span>
          </div>
        </div>
      </div>

      <div className="mt-4 flex items-center gap-2 border-t pt-3">
        <button
          type="button"
          onClick={() => onView?.(violation.id)}
          className="text-sm text-blue-600 hover:text-blue-800"
        >
          {t('common.view')}
        </button>
        {canResolve && (
          <button
            type="button"
            onClick={() => onResolve?.(violation.id)}
            className="text-sm text-green-600 hover:text-green-800"
          >
            {t('leases.violations.resolve')}
          </button>
        )}
        {canDispute && (
          <button
            type="button"
            onClick={() => onDispute?.(violation.id)}
            className="text-sm text-yellow-600 hover:text-yellow-800"
          >
            {t('leases.violations.dispute')}
          </button>
        )}
      </div>
    </div>
  );
}
