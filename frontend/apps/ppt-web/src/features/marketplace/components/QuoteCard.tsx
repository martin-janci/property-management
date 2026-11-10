/**
 * QuoteCard component - displays a provider quote summary.
 * Epic 68: Service Provider Marketplace (Story 68.3)
 */

import type { TFunction } from 'i18next';
import { useTranslation } from 'react-i18next';

export type QuoteStatus =
  | 'pending'
  | 'submitted'
  | 'accepted'
  | 'rejected'
  | 'withdrawn'
  | 'expired';

export interface QuoteSummary {
  id: string;
  rfqId: string;
  providerId: string;
  providerName: string;
  providerLogoUrl?: string;
  providerRating?: number;
  providerIsVerified?: boolean;
  price: number;
  currency: string;
  estimatedStartDate?: string;
  estimatedEndDate?: string;
  estimatedDurationDays?: number;
  warrantyPeriodDays?: number;
  status: QuoteStatus;
  validUntil?: string;
  submittedAt?: string;
  notes?: string;
}

interface QuoteCardProps {
  quote: QuoteSummary;
  isSelected?: boolean;
  onView?: (id: string) => void;
  onAccept?: (id: string) => void;
  onReject?: (id: string) => void;
  onSelect?: (id: string) => void;
  onViewProvider?: (providerId: string) => void;
}

const statusColors: Record<QuoteStatus, string> = {
  pending: 'bg-yellow-100 text-yellow-800',
  submitted: 'bg-blue-100 text-blue-800',
  accepted: 'bg-green-100 text-green-800',
  rejected: 'bg-red-100 text-red-800',
  withdrawn: 'bg-gray-100 text-gray-800',
  expired: 'bg-orange-100 text-orange-800',
};

function formatPrice(price: number, currency: string): string {
  return new Intl.NumberFormat('en-EU', {
    style: 'currency',
    currency,
  }).format(price);
}

function formatDate(t: TFunction, dateString?: string): string {
  if (!dateString) return t('marketplace.quoteCard.notSpecified');
  return new Date(dateString).toLocaleDateString('en-EU', {
    month: 'short',
    day: 'numeric',
    year: 'numeric',
  });
}

function getValidityStatus(
  t: TFunction,
  validUntil?: string
): { text: string; isExpired: boolean } {
  if (!validUntil) return { text: t('marketplace.quoteCard.noExpiry'), isExpired: false };

  const expiryDate = new Date(validUntil);
  const now = new Date();
  const diffMs = expiryDate.getTime() - now.getTime();
  const diffDays = Math.ceil(diffMs / (1000 * 60 * 60 * 24));

  if (diffDays < 0) {
    return { text: t('marketplace.quoteCard.expired'), isExpired: true };
  }
  if (diffDays === 0) {
    return { text: t('marketplace.quoteCard.expiresToday'), isExpired: false };
  }
  if (diffDays === 1) {
    return { text: t('marketplace.quoteCard.expiresTomorrow'), isExpired: false };
  }
  return { text: t('marketplace.quoteCard.validForDays', { count: diffDays }), isExpired: false };
}

export function QuoteCard({
  quote,
  isSelected,
  onView,
  onAccept,
  onReject,
  onSelect,
  onViewProvider,
}: QuoteCardProps) {
  const { t } = useTranslation();
  const canAccept = quote.status === 'submitted';
  const canReject = quote.status === 'submitted';
  const validity = getValidityStatus(t, quote.validUntil);

  return (
    <div
      className={`bg-white rounded-lg shadow p-6 transition-all ${
        isSelected ? 'ring-2 ring-blue-500' : 'hover:shadow-md'
      }`}
    >
      {/* Provider Info */}
      <div className="flex items-start justify-between">
        <div className="flex items-center gap-3">
          {quote.providerLogoUrl ? (
            <img
              src={quote.providerLogoUrl}
              alt={`${quote.providerName} logo`}
              className="w-12 h-12 rounded-lg object-cover"
            />
          ) : (
            <div className="w-12 h-12 rounded-lg bg-gray-200 flex items-center justify-center">
              <svg
                className="w-6 h-6 text-gray-400"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <title>{t('marketplace.common.companyPlaceholder')}</title>
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M19 21V5a2 2 0 00-2-2H7a2 2 0 00-2 2v16m14 0h2m-2 0h-5m-9 0H3m2 0h5M9 7h1m-1 4h1m4-4h1m-1 4h1m-5 10v-5a1 1 0 011-1h2a1 1 0 011 1v5m-4 0h4"
                />
              </svg>
            </div>
          )}
          <div>
            <div className="flex items-center gap-2">
              <button
                type="button"
                onClick={() => onViewProvider?.(quote.providerId)}
                className="text-lg font-semibold text-gray-900 hover:text-blue-600"
              >
                {quote.providerName}
              </button>
              {quote.providerIsVerified && (
                <svg className="w-5 h-5 text-blue-500" fill="currentColor" viewBox="0 0 20 20">
                  <title>{t('marketplace.common.verified')}</title>
                  <path
                    fillRule="evenodd"
                    d="M6.267 3.455a3.066 3.066 0 001.745-.723 3.066 3.066 0 013.976 0 3.066 3.066 0 001.745.723 3.066 3.066 0 012.812 2.812c.051.643.304 1.254.723 1.745a3.066 3.066 0 010 3.976 3.066 3.066 0 00-.723 1.745 3.066 3.066 0 01-2.812 2.812 3.066 3.066 0 00-1.745.723 3.066 3.066 0 01-3.976 0 3.066 3.066 0 00-1.745-.723 3.066 3.066 0 01-2.812-2.812 3.066 3.066 0 00-.723-1.745 3.066 3.066 0 010-3.976 3.066 3.066 0 00.723-1.745 3.066 3.066 0 012.812-2.812zm7.44 5.252a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
                    clipRule="evenodd"
                  />
                </svg>
              )}
            </div>
            {quote.providerRating && (
              <div className="flex items-center gap-1 mt-0.5">
                <svg className="w-4 h-4 text-yellow-400" fill="currentColor" viewBox="0 0 20 20">
                  <title>{t('marketplace.common.ratingStar')}</title>
                  <path d="M9.049 2.927c.3-.921 1.603-.921 1.902 0l1.07 3.292a1 1 0 00.95.69h3.462c.969 0 1.371 1.24.588 1.81l-2.8 2.034a1 1 0 00-.364 1.118l1.07 3.292c.3.921-.755 1.688-1.54 1.118l-2.8-2.034a1 1 0 00-1.175 0l-2.8 2.034c-.784.57-1.838-.197-1.539-1.118l1.07-3.292a1 1 0 00-.364-1.118L2.98 8.72c-.783-.57-.38-1.81.588-1.81h3.461a1 1 0 00.951-.69l1.07-3.292z" />
                </svg>
                <span className="text-sm text-gray-600">{quote.providerRating.toFixed(1)}</span>
              </div>
            )}
          </div>
        </div>
        <span className={`px-2 py-1 text-xs font-medium rounded ${statusColors[quote.status]}`}>
          {t(`marketplace.quoteStatus.${quote.status}`)}
        </span>
      </div>

      {/* Price */}
      <div className="mt-4 text-center py-4 bg-gray-50 rounded-lg">
        <p className="text-3xl font-bold text-gray-900">
          {formatPrice(quote.price, quote.currency)}
        </p>
        {quote.warrantyPeriodDays && (
          <p className="text-sm text-gray-500 mt-1">
            {t('marketplace.quoteCard.warrantyIncluded', { count: quote.warrantyPeriodDays })}
          </p>
        )}
      </div>

      {/* Details */}
      <div className="mt-4 grid grid-cols-2 gap-4 text-sm">
        <div>
          <span className="text-gray-500">{t('marketplace.quoteCard.startDate')}</span>{' '}
          <span className="font-medium text-gray-900">
            {formatDate(t, quote.estimatedStartDate)}
          </span>
        </div>
        <div>
          <span className="text-gray-500">{t('marketplace.quoteCard.endDate')}</span>{' '}
          <span className="font-medium text-gray-900">{formatDate(t, quote.estimatedEndDate)}</span>
        </div>
        {quote.estimatedDurationDays && (
          <div>
            <span className="text-gray-500">{t('marketplace.quoteCard.duration')}</span>{' '}
            <span className="font-medium text-gray-900">
              {t('marketplace.quoteCard.durationDays', { count: quote.estimatedDurationDays })}
            </span>
          </div>
        )}
        <div>
          <span className="text-gray-500">{t('marketplace.quoteCard.validity')}</span>{' '}
          <span className={`font-medium ${validity.isExpired ? 'text-red-600' : 'text-gray-900'}`}>
            {validity.text}
          </span>
        </div>
      </div>

      {/* Notes */}
      {quote.notes && (
        <div className="mt-4 p-3 bg-gray-50 rounded-lg">
          <p className="text-sm text-gray-600 line-clamp-2">{quote.notes}</p>
        </div>
      )}

      <p className="mt-2 text-xs text-gray-400">
        {t('marketplace.quoteCard.submittedOn', { date: formatDate(t, quote.submittedAt) })}
      </p>

      {/* Actions */}
      <div className="mt-4 flex items-center gap-3 pt-4 border-t">
        <button
          type="button"
          onClick={() => onView?.(quote.id)}
          className="text-sm font-medium text-blue-600 hover:text-blue-800"
        >
          {t('marketplace.common.viewDetails')}
        </button>
        {onSelect && (
          <button
            type="button"
            onClick={() => onSelect(quote.id)}
            className={`text-sm font-medium ${
              isSelected ? 'text-blue-600 hover:text-blue-800' : 'text-gray-600 hover:text-gray-800'
            }`}
          >
            {isSelected
              ? t('marketplace.common.selected')
              : t('marketplace.quoteCard.selectToCompare')}
          </button>
        )}
        {canAccept && (
          <button
            type="button"
            onClick={() => onAccept?.(quote.id)}
            className="text-sm font-medium text-green-600 hover:text-green-800"
          >
            {t('marketplace.quoteCard.acceptQuote')}
          </button>
        )}
        {canReject && (
          <button
            type="button"
            onClick={() => onReject?.(quote.id)}
            className="text-sm font-medium text-red-600 hover:text-red-800"
          >
            {t('marketplace.quoteCard.reject')}
          </button>
        )}
      </div>
    </div>
  );
}
