/**
 * LeaseCard component - displays a lease summary card in lists.
 * Epic 19: Lease Management & Tenant Screening
 */

import { useTranslation } from 'react-i18next';
import type { LeaseStatus, LeaseSummary } from '../types';
import { SignatureStatusBadge } from './SignatureStatusBadge';

interface LeaseCardProps {
  lease: LeaseSummary;
  onView?: (id: string) => void;
  onRenew?: (id: string) => void;
  onTerminate?: (id: string) => void;
}

const statusColors: Record<LeaseStatus, string> = {
  draft: 'bg-gray-100 text-gray-800',
  pending_signatures: 'bg-yellow-100 text-yellow-800',
  active: 'bg-green-100 text-green-800',
  expiring_soon: 'bg-orange-100 text-orange-800',
  expired: 'bg-red-100 text-red-800',
  terminated: 'bg-gray-100 text-gray-800',
  renewed: 'bg-blue-100 text-blue-800',
};

export function LeaseCard({ lease, onView, onRenew, onTerminate }: LeaseCardProps) {
  const { t } = useTranslation();

  const canRenew = lease.status === 'active' || lease.status === 'expiring_soon';
  const canTerminate = lease.status === 'active' || lease.status === 'expiring_soon';

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString();
  };

  const formatCurrency = (amount: number, currency: string) => {
    return new Intl.NumberFormat(undefined, {
      style: 'currency',
      currency,
    }).format(amount);
  };

  return (
    <div className="bg-white rounded-lg shadow p-4 hover:shadow-md transition-shadow">
      <div className="flex items-start justify-between">
        <div className="flex-1">
          <div className="flex items-center gap-2">
            {lease.status === 'expiring_soon' && (
              <span className="text-orange-500" title={t('leases.expiringSoon')}>
                <svg
                  className="w-4 h-4"
                  fill="currentColor"
                  viewBox="0 0 20 20"
                  aria-label={t('leases.expiringSoon')}
                >
                  <title>{t('leases.expiringSoon')}</title>
                  <path
                    fillRule="evenodd"
                    d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z"
                    clipRule="evenodd"
                  />
                </svg>
              </span>
            )}
            <h3 className="text-lg font-semibold text-gray-900">
              {lease.buildingName} - {lease.unitNumber}
            </h3>
          </div>
          <p className="text-sm text-gray-600 mt-1">{lease.tenantName}</p>
          <p className="text-xs text-gray-500">{lease.tenantEmail}</p>
          <div className="mt-2 flex items-center gap-2 flex-wrap">
            <span className={`px-2 py-1 text-xs font-medium rounded ${statusColors[lease.status]}`}>
              {t(`leases.status.${lease.status}`)}
            </span>
            <span className="text-sm font-medium text-gray-900">
              {formatCurrency(lease.rentAmount, lease.currency)}
              {t('leases.perMonth')}
            </span>
            {lease.signatureStatus && <SignatureStatusBadge status={lease.signatureStatus} />}
          </div>
          <div className="mt-2 text-xs text-gray-500">
            <span>{formatDate(lease.startDate)}</span>
            <span className="mx-1">-</span>
            <span>{formatDate(lease.endDate)}</span>
            {lease.daysUntilExpiry !== undefined && lease.daysUntilExpiry <= 90 && (
              <span className="ml-2 text-orange-600">
                ({lease.daysUntilExpiry} {t('leases.daysRemaining')})
              </span>
            )}
          </div>
        </div>
      </div>

      <div className="mt-4 flex items-center gap-2 border-t pt-3">
        <button
          type="button"
          onClick={() => onView?.(lease.id)}
          className="text-sm text-blue-600 hover:text-blue-800"
        >
          {t('common.view')}
        </button>
        {canRenew && (
          <button
            type="button"
            onClick={() => onRenew?.(lease.id)}
            className="text-sm text-green-600 hover:text-green-800"
          >
            {t('leases.renew')}
          </button>
        )}
        {canTerminate && (
          <button
            type="button"
            onClick={() => onTerminate?.(lease.id)}
            className="text-sm text-red-600 hover:text-red-800"
          >
            {t('leases.terminate')}
          </button>
        )}
      </div>
    </div>
  );
}
