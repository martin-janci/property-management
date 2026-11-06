/**
 * RfqCard component - displays an RFQ summary card.
 * Epic 68: Service Provider Marketplace (Story 68.3)
 */

import type { TFunction } from 'i18next';
import { useTranslation } from 'react-i18next';
import type { ServiceCategory } from './ProviderCard';

export type RfqStatus = 'draft' | 'sent' | 'quotes_received' | 'awarded' | 'cancelled' | 'expired';

export interface RfqSummary {
  id: string;
  title: string;
  description: string;
  serviceCategory: ServiceCategory;
  status: RfqStatus;
  isUrgent?: boolean;
  budgetMin?: number;
  budgetMax?: number;
  currency?: string;
  preferredStartDate?: string;
  quoteDeadline?: string;
  quotesCount: number;
  invitedProvidersCount: number;
  createdAt: string;
}

interface RfqCardProps {
  rfq: RfqSummary;
  onView?: (id: string) => void;
  onEdit?: (id: string) => void;
  onCompareQuotes?: (id: string) => void;
  onCancel?: (id: string) => void;
}

const statusColors: Record<RfqStatus, string> = {
  draft: 'bg-gray-100 text-gray-800',
  sent: 'bg-blue-100 text-blue-800',
  quotes_received: 'bg-green-100 text-green-800',
  awarded: 'bg-purple-100 text-purple-800',
  cancelled: 'bg-red-100 text-red-800',
  expired: 'bg-orange-100 text-orange-800',
};

function formatBudget(t: TFunction, min?: number, max?: number, currency = 'EUR'): string {
  if (!min && !max) return t('marketplace.rfqCard.noBudget');

  const formatter = new Intl.NumberFormat('en-EU', {
    style: 'currency',
    currency,
    minimumFractionDigits: 0,
    maximumFractionDigits: 0,
  });

  if (min && max) {
    return t('marketplace.rfqCard.budgetRange', {
      min: formatter.format(min),
      max: formatter.format(max),
    });
  }
  if (min) {
    return t('marketplace.rfqCard.budgetFrom', { min: formatter.format(min) });
  }
  return t('marketplace.rfqCard.budgetUpTo', { max: formatter.format(max!) });
}

function formatDate(t: TFunction, dateString?: string): string {
  if (!dateString) return t('marketplace.rfqCard.notSpecified');
  return new Date(dateString).toLocaleDateString('en-EU', {
    month: 'short',
    day: 'numeric',
    year: 'numeric',
  });
}

function getDeadlineStatus(t: TFunction, deadline?: string): { text: string; urgent: boolean } {
  if (!deadline) return { text: t('marketplace.rfqCard.noDeadline'), urgent: false };

  const deadlineDate = new Date(deadline);
  const now = new Date();
  const diffMs = deadlineDate.getTime() - now.getTime();
  const diffDays = Math.ceil(diffMs / (1000 * 60 * 60 * 24));

  if (diffDays < 0) {
    return { text: t('marketplace.rfqCard.expired'), urgent: true };
  }
  if (diffDays === 0) {
    return { text: t('marketplace.rfqCard.dueToday'), urgent: true };
  }
  if (diffDays === 1) {
    return { text: t('marketplace.rfqCard.dueTomorrow'), urgent: true };
  }
  if (diffDays <= 3) {
    return { text: t('marketplace.rfqCard.daysLeft', { count: diffDays }), urgent: true };
  }
  return { text: t('marketplace.rfqCard.daysLeft', { count: diffDays }), urgent: false };
}

export function RfqCard({ rfq, onView, onEdit, onCompareQuotes, onCancel }: RfqCardProps) {
  const { t } = useTranslation();
  const canEdit = rfq.status === 'draft';
  const canCompare = rfq.quotesCount > 0;
  const canCancel = rfq.status === 'draft' || rfq.status === 'sent';
  const deadlineStatus = getDeadlineStatus(t, rfq.quoteDeadline);

  return (
    <div className="bg-white rounded-lg shadow p-6 hover:shadow-md transition-shadow">
      <div className="flex items-start justify-between">
        <div className="flex-1">
          <div className="flex items-center gap-2">
            {rfq.isUrgent && (
              <span className="text-red-500" title={t('marketplace.rfqCard.urgent')}>
                <svg
                  className="w-5 h-5"
                  fill="currentColor"
                  viewBox="0 0 20 20"
                  aria-label={t('aria.urgent')}
                >
                  <title>{t('marketplace.rfqCard.urgent')}</title>
                  <path
                    fillRule="evenodd"
                    d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z"
                    clipRule="evenodd"
                  />
                </svg>
              </span>
            )}
            <h3 className="text-lg font-semibold text-gray-900">{rfq.title}</h3>
          </div>
          <p className="mt-1 text-sm text-gray-600 line-clamp-2">{rfq.description}</p>
        </div>
        <span className={`px-2 py-1 text-xs font-medium rounded ${statusColors[rfq.status]}`}>
          {t(`marketplace.rfqStatus.${rfq.status}`)}
        </span>
      </div>

      <div className="mt-4 grid grid-cols-2 gap-4 text-sm">
        <div>
          <span className="text-gray-500">{t('marketplace.rfqCard.category')}</span>{' '}
          <span className="font-medium text-gray-900">
            {t(`marketplace.serviceCategory.${rfq.serviceCategory}`)}
          </span>
        </div>
        <div>
          <span className="text-gray-500">{t('marketplace.rfqCard.budget')}</span>{' '}
          <span className="font-medium text-gray-900">
            {formatBudget(t, rfq.budgetMin, rfq.budgetMax, rfq.currency)}
          </span>
        </div>
        <div>
          <span className="text-gray-500">{t('marketplace.rfqCard.startDate')}</span>{' '}
          <span className="font-medium text-gray-900">{formatDate(t, rfq.preferredStartDate)}</span>
        </div>
        <div>
          <span className="text-gray-500">{t('marketplace.rfqCard.quoteDeadline')}</span>{' '}
          <span
            className={`font-medium ${deadlineStatus.urgent ? 'text-red-600' : 'text-gray-900'}`}
          >
            {deadlineStatus.text}
          </span>
        </div>
      </div>

      {/* Quote Stats */}
      <div className="mt-4 flex items-center gap-4">
        <div className="flex items-center gap-1">
          <svg
            className="w-5 h-5 text-gray-400"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <title>{t('marketplace.rfqCard.providersInvited')}</title>
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z"
            />
          </svg>
          <span className="text-sm text-gray-600">
            {t('marketplace.rfqCard.invited', { count: rfq.invitedProvidersCount })}
          </span>
        </div>
        <div className="flex items-center gap-1">
          <svg
            className="w-5 h-5 text-gray-400"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <title>{t('marketplace.rfqCard.quotesReceived')}</title>
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
            />
          </svg>
          <span className="text-sm text-gray-600">
            {rfq.quotesCount === 1
              ? t('marketplace.rfqCard.quotesCountOne', { count: rfq.quotesCount })
              : t('marketplace.rfqCard.quotesCountOther', { count: rfq.quotesCount })}
          </span>
        </div>
      </div>

      <p className="mt-2 text-xs text-gray-400">
        {t('marketplace.rfqCard.created', { date: formatDate(t, rfq.createdAt) })}
      </p>

      {/* Actions */}
      <div className="mt-4 flex items-center gap-3 pt-4 border-t">
        <button
          type="button"
          onClick={() => onView?.(rfq.id)}
          className="text-sm font-medium text-blue-600 hover:text-blue-800"
        >
          {t('marketplace.common.viewDetails')}
        </button>
        {canEdit && (
          <button
            type="button"
            onClick={() => onEdit?.(rfq.id)}
            className="text-sm font-medium text-gray-600 hover:text-gray-800"
          >
            {t('marketplace.rfqCard.edit')}
          </button>
        )}
        {canCompare && (
          <button
            type="button"
            onClick={() => onCompareQuotes?.(rfq.id)}
            className="text-sm font-medium text-green-600 hover:text-green-800"
          >
            {t('marketplace.rfqCard.compareQuotes')}
          </button>
        )}
        {canCancel && (
          <button
            type="button"
            onClick={() => onCancel?.(rfq.id)}
            className="text-sm font-medium text-red-600 hover:text-red-800"
          >
            {t('marketplace.common.cancel')}
          </button>
        )}
      </div>
    </div>
  );
}
