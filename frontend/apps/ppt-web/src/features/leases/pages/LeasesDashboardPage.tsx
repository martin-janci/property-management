/**
 * LeasesDashboardPage - Overview with expiring leases and statistics.
 * Epic 19: Lease Management & Tenant Screening
 */

import { useTranslation } from 'react-i18next';
import { LeaseCard } from '../components/LeaseCard';
import type { ExpirationOverview, LeaseStatistics, LeaseSummary } from '../types';

interface LeasesDashboardPageProps {
  statistics: LeaseStatistics;
  expirationOverview: ExpirationOverview;
  isLoading?: boolean;
  onNavigateToLease: (id: string) => void;
  onNavigateToLeases: () => void;
  onNavigateToApplications: () => void;
  onNavigateToCreateLease: () => void;
  onRenewLease?: (id: string) => void;
  onTerminateLease?: (id: string) => void;
}

interface StatCardProps {
  title: string;
  value: string | number;
  subtitle?: string;
  className?: string;
  icon?: React.ReactNode;
}

function StatCard({ title, value, subtitle, className = '', icon }: StatCardProps) {
  return (
    <div className={`bg-white rounded-lg shadow p-4 ${className}`}>
      <div className="flex items-center justify-between">
        <div>
          <p className="text-sm text-gray-500">{title}</p>
          <p className="text-2xl font-semibold text-gray-900">{value}</p>
          {subtitle && <p className="text-xs text-gray-500 mt-1">{subtitle}</p>}
        </div>
        {icon && <div className="text-gray-400">{icon}</div>}
      </div>
    </div>
  );
}

export function LeasesDashboardPage({
  statistics,
  expirationOverview,
  isLoading,
  onNavigateToLease,
  onNavigateToLeases,
  onNavigateToApplications,
  onNavigateToCreateLease,
  onRenewLease,
  onTerminateLease,
}: LeasesDashboardPageProps) {
  const { t, i18n } = useTranslation();

  const formatCurrency = (amount: number, currency: string) => {
    return new Intl.NumberFormat(i18n.language, {
      style: 'currency',
      currency,
    }).format(amount);
  };

  // Locale-aware percent (sk → "0,0 %", en → "0.0%"); `value` is a 0..1 ratio.
  const formatPercentage = (value: number) => {
    return new Intl.NumberFormat(i18n.language, {
      style: 'percent',
      minimumFractionDigits: 1,
      maximumFractionDigits: 1,
    }).format(value);
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-12">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
      </div>
    );
  }

  const urgentLeases = [...expirationOverview.expired, ...expirationOverview.expiringIn30Days];

  return (
    <div className="min-h-screen bg-gray-100">
      {/* Header */}
      <div className="bg-white shadow">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
          <div className="flex flex-col lg:flex-row lg:items-center lg:justify-between gap-4">
            <div>
              <h1 className="text-2xl font-bold text-gray-900">{t('leases.dashboard.title')}</h1>
              <p className="mt-1 text-sm text-gray-500">{t('leases.dashboard.subtitle')}</p>
            </div>
            <div className="flex gap-3">
              <button
                type="button"
                onClick={onNavigateToApplications}
                className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50"
              >
                {t('leases.dashboard.viewApplications')}
              </button>
              <button
                type="button"
                onClick={onNavigateToCreateLease}
                className="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-md"
              >
                {t('leases.dashboard.createLease')}
              </button>
            </div>
          </div>
        </div>
      </div>

      {/* Main Content */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {/* Statistics Grid */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 mb-8">
          <StatCard
            title={t('leases.dashboard.stats.totalLeases')}
            value={statistics.totalLeases}
            icon={
              <svg className="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
                />
              </svg>
            }
          />
          <StatCard
            title={t('leases.dashboard.stats.activeLeases')}
            value={statistics.activeLeases}
            subtitle={t('leases.dashboard.stats.occupancyRate', {
              rate: formatPercentage(statistics.occupancyRate),
            })}
            icon={
              <svg className="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"
                />
              </svg>
            }
          />
          <StatCard
            title={t('leases.dashboard.stats.expiringLeases')}
            value={statistics.expiringLeases}
            className={statistics.expiringLeases > 0 ? 'border-l-4 border-orange-500' : ''}
            icon={
              <svg
                className={`w-8 h-8 ${statistics.expiringLeases > 0 ? 'text-orange-400' : ''}`}
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"
                />
              </svg>
            }
          />
          <StatCard
            title={t('leases.dashboard.stats.monthlyRent')}
            value={formatCurrency(statistics.totalMonthlyRent, statistics.currency)}
            icon={
              <svg className="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                />
              </svg>
            }
          />
        </div>

        {/* Secondary Stats Row */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-8">
          <StatCard
            title={t('leases.dashboard.stats.pendingApplications')}
            value={statistics.pendingApplications}
            className={statistics.pendingApplications > 0 ? 'border-l-4 border-blue-500' : ''}
          />
          <StatCard
            title={t('leases.dashboard.stats.expiredLeases')}
            value={statistics.expiredLeases}
            className={statistics.expiredLeases > 0 ? 'border-l-4 border-red-500' : ''}
          />
          <StatCard
            title={t('leases.dashboard.stats.overduePayments')}
            value={statistics.overduePayments}
            className={statistics.overduePayments > 0 ? 'border-l-4 border-red-500' : ''}
          />
        </div>

        {/* Urgent Leases Section */}
        {urgentLeases.length > 0 && (
          <div className="mb-8">
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-lg font-semibold text-gray-900">
                {t('leases.dashboard.urgentLeases')}
              </h2>
              <button
                type="button"
                onClick={onNavigateToLeases}
                className="text-sm text-blue-600 hover:text-blue-800"
              >
                {t('leases.dashboard.viewAll')}
              </button>
            </div>
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
              {urgentLeases.slice(0, 6).map((lease) => (
                <LeaseCard
                  key={lease.id}
                  lease={lease}
                  onView={onNavigateToLease}
                  onRenew={onRenewLease}
                  onTerminate={onTerminateLease}
                />
              ))}
            </div>
          </div>
        )}

        {/* Expiration Buckets */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
          {/* Expiring in 30 Days */}
          <ExpirationBucket
            title={t('leases.dashboard.expiring30Days')}
            leases={expirationOverview.expiringIn30Days}
            emptyMessage={t('leases.dashboard.noLeasesExpiring')}
            onNavigateToLease={onNavigateToLease}
            onRenewLease={onRenewLease}
            urgencyLevel="high"
          />

          {/* Expiring in 60 Days */}
          <ExpirationBucket
            title={t('leases.dashboard.expiring60Days')}
            leases={expirationOverview.expiringIn60Days}
            emptyMessage={t('leases.dashboard.noLeasesExpiring')}
            onNavigateToLease={onNavigateToLease}
            onRenewLease={onRenewLease}
            urgencyLevel="medium"
          />
        </div>
      </div>
    </div>
  );
}

interface ExpirationBucketProps {
  title: string;
  leases: LeaseSummary[];
  emptyMessage: string;
  onNavigateToLease: (id: string) => void;
  onRenewLease?: (id: string) => void;
  urgencyLevel: 'high' | 'medium' | 'low';
}

function ExpirationBucket({
  title,
  leases,
  emptyMessage,
  onNavigateToLease,
  onRenewLease,
  urgencyLevel,
}: ExpirationBucketProps) {
  const { t } = useTranslation();
  const urgencyColors = {
    high: 'border-red-500',
    medium: 'border-orange-500',
    low: 'border-yellow-500',
  };

  return (
    <div className={`bg-white rounded-lg shadow border-t-4 ${urgencyColors[urgencyLevel]}`}>
      <div className="px-4 py-3 border-b">
        <h3 className="font-semibold text-gray-900">
          {title} ({leases.length})
        </h3>
      </div>
      <div className="p-4">
        {leases.length === 0 ? (
          <p className="text-sm text-gray-500 text-center py-4">{emptyMessage}</p>
        ) : (
          <div className="space-y-3">
            {leases.slice(0, 5).map((lease) => (
              <div
                key={lease.id}
                className="flex items-center justify-between p-3 bg-gray-50 rounded-lg"
              >
                <div>
                  <p className="font-medium text-gray-900">
                    {lease.buildingName} - {lease.unitNumber}
                  </p>
                  <p className="text-sm text-gray-500">{lease.tenantName}</p>
                </div>
                <div className="flex items-center gap-2">
                  {lease.daysUntilExpiry !== undefined && (
                    <span className="text-sm text-gray-500">
                      {t('leases.daysShort', { count: lease.daysUntilExpiry })}
                    </span>
                  )}
                  <button
                    type="button"
                    onClick={() => onNavigateToLease(lease.id)}
                    className="text-sm text-blue-600 hover:text-blue-800"
                  >
                    {t('common.view')}
                  </button>
                  {onRenewLease && (
                    <button
                      type="button"
                      onClick={() => onRenewLease(lease.id)}
                      className="text-sm text-green-600 hover:text-green-800"
                    >
                      {t('leases.renew')}
                    </button>
                  )}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
