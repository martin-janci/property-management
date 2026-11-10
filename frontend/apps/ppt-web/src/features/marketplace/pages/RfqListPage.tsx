/**
 * RfqListPage - list of RFQs for the organization.
 * Epic 68: Service Provider Marketplace (Story 68.3)
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { RfqCard, type RfqStatus, type RfqSummary } from '../components/RfqCard';

interface RfqListPageProps {
  rfqs: RfqSummary[];
  totalRfqs: number;
  isLoading?: boolean;
  onCreateRfq: () => void;
  onViewRfq: (id: string) => void;
  onEditRfq: (id: string) => void;
  onCompareQuotes: (id: string) => void;
  onCancelRfq: (id: string) => void;
  onFilterChange: (status?: RfqStatus) => void;
  onLoadMore?: () => void;
  hasMore?: boolean;
}

const statusTabs: { value: RfqStatus | 'all'; labelKey: string }[] = [
  { value: 'all', labelKey: 'marketplace.rfqListPage.tabAll' },
  { value: 'draft', labelKey: 'marketplace.rfqListPage.tabDrafts' },
  { value: 'sent', labelKey: 'marketplace.rfqListPage.tabSent' },
  { value: 'quotes_received', labelKey: 'marketplace.rfqListPage.tabQuotesReceived' },
  { value: 'awarded', labelKey: 'marketplace.rfqListPage.tabAwarded' },
  { value: 'cancelled', labelKey: 'marketplace.rfqListPage.tabCancelled' },
];

export function RfqListPage({
  rfqs,
  totalRfqs,
  isLoading,
  onCreateRfq,
  onViewRfq,
  onEditRfq,
  onCompareQuotes,
  onCancelRfq,
  onFilterChange,
  onLoadMore,
  hasMore,
}: RfqListPageProps) {
  const { t } = useTranslation();
  const [activeStatus, setActiveStatus] = useState<RfqStatus | 'all'>('all');

  const handleStatusChange = (status: RfqStatus | 'all') => {
    setActiveStatus(status);
    onFilterChange(status === 'all' ? undefined : status);
  };

  // Count RFQs by status for badges
  const statusCounts = rfqs.reduce(
    (acc, rfq) => {
      acc[rfq.status] = (acc[rfq.status] || 0) + 1;
      return acc;
    },
    {} as Record<RfqStatus, number>
  );

  return (
    <div className="max-w-5xl mx-auto px-4 py-8">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">{t('marketplace.rfqListPage.title')}</h1>
          <p className="mt-1 text-gray-600">{t('marketplace.rfqListPage.subtitle')}</p>
        </div>
        <button
          type="button"
          onClick={onCreateRfq}
          className="px-4 py-2 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2"
        >
          {t('marketplace.rfqListPage.createNewRfq')}
        </button>
      </div>

      {/* Status Tabs */}
      <div className="border-b border-gray-200 mb-6">
        <nav className="flex gap-4 overflow-x-auto">
          {statusTabs.map((tab) => {
            const count = tab.value === 'all' ? totalRfqs : statusCounts[tab.value] || 0;
            return (
              <button
                key={tab.value}
                type="button"
                onClick={() => handleStatusChange(tab.value)}
                className={`flex items-center gap-2 py-4 px-1 text-sm font-medium border-b-2 -mb-px whitespace-nowrap ${
                  activeStatus === tab.value
                    ? 'border-blue-500 text-blue-600'
                    : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
                }`}
              >
                {t(tab.labelKey)}
                {count > 0 && (
                  <span
                    className={`px-2 py-0.5 text-xs font-medium rounded-full ${
                      activeStatus === tab.value
                        ? 'bg-blue-100 text-blue-600'
                        : 'bg-gray-100 text-gray-600'
                    }`}
                  >
                    {count}
                  </span>
                )}
              </button>
            );
          })}
        </nav>
      </div>

      {/* RFQ List */}
      {isLoading && rfqs.length === 0 ? (
        <div className="space-y-4">
          {[...Array(3)].map((_, i) => (
            <div key={`skeleton-${i}`} className="bg-white rounded-lg shadow p-6 animate-pulse">
              <div className="flex items-start justify-between">
                <div className="flex-1">
                  <div className="h-6 bg-gray-200 rounded w-1/3 mb-2" />
                  <div className="h-4 bg-gray-200 rounded w-2/3" />
                </div>
                <div className="h-6 bg-gray-200 rounded w-20" />
              </div>
              <div className="mt-4 grid grid-cols-2 gap-4">
                <div className="h-4 bg-gray-200 rounded" />
                <div className="h-4 bg-gray-200 rounded" />
              </div>
            </div>
          ))}
        </div>
      ) : rfqs.length === 0 ? (
        <div className="text-center py-12 bg-gray-50 rounded-lg">
          <svg
            className="mx-auto w-12 h-12 text-gray-400"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <title>{t('marketplace.rfqListPage.noRfqs')}</title>
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
            />
          </svg>
          <h3 className="mt-4 text-lg font-medium text-gray-900">
            {activeStatus === 'all'
              ? t('marketplace.rfqListPage.noRfqsTitle')
              : t('marketplace.rfqListPage.noStatusRfqsTitle', {
                  status: t(`marketplace.rfqStatus.${activeStatus}`),
                })}
          </h3>
          <p className="mt-2 text-gray-500">
            {activeStatus === 'all'
              ? t('marketplace.rfqListPage.noRfqsAllHint')
              : t('marketplace.rfqListPage.noRfqsStatusHint')}
          </p>
          {activeStatus === 'all' && (
            <button
              type="button"
              onClick={onCreateRfq}
              className="mt-4 px-4 py-2 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700"
            >
              {t('marketplace.rfqListPage.createNewRfq')}
            </button>
          )}
        </div>
      ) : (
        <>
          <div className="space-y-4">
            {rfqs.map((rfq) => (
              <RfqCard
                key={rfq.id}
                rfq={rfq}
                onView={onViewRfq}
                onEdit={onEditRfq}
                onCompareQuotes={onCompareQuotes}
                onCancel={onCancelRfq}
              />
            ))}
          </div>

          {/* Load More */}
          {hasMore && (
            <div className="mt-8 text-center">
              <button
                type="button"
                onClick={onLoadMore}
                disabled={isLoading}
                className="px-6 py-2 border border-gray-300 text-gray-700 font-medium rounded-lg hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 disabled:opacity-50"
              >
                {isLoading
                  ? t('marketplace.rfqListPage.loading')
                  : t('marketplace.rfqListPage.loadMore')}
              </button>
            </div>
          )}
        </>
      )}

      {/* Quick Stats */}
      {totalRfqs > 0 && (
        <div className="mt-8 grid grid-cols-2 md:grid-cols-4 gap-4">
          <div className="bg-white rounded-lg shadow p-4 text-center">
            <p className="text-2xl font-bold text-gray-900">{totalRfqs}</p>
            <p className="text-sm text-gray-500">{t('marketplace.rfqListPage.totalRfqs')}</p>
          </div>
          <div className="bg-white rounded-lg shadow p-4 text-center">
            <p className="text-2xl font-bold text-blue-600">{statusCounts.sent || 0}</p>
            <p className="text-sm text-gray-500">{t('marketplace.rfqListPage.activeRfqs')}</p>
          </div>
          <div className="bg-white rounded-lg shadow p-4 text-center">
            <p className="text-2xl font-bold text-green-600">{statusCounts.quotes_received || 0}</p>
            <p className="text-sm text-gray-500">{t('marketplace.rfqListPage.withQuotes')}</p>
          </div>
          <div className="bg-white rounded-lg shadow p-4 text-center">
            <p className="text-2xl font-bold text-purple-600">{statusCounts.awarded || 0}</p>
            <p className="text-sm text-gray-500">{t('marketplace.rfqListPage.awarded')}</p>
          </div>
        </div>
      )}
    </div>
  );
}
