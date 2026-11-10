/**
 * QuoteComparisonTable component - side-by-side quote comparison.
 * Epic 68: Service Provider Marketplace (Story 68.3)
 */

import { useTranslation } from 'react-i18next';
import type { QuoteSummary } from './QuoteCard';

interface QuoteComparisonTableProps {
  quotes: QuoteSummary[];
  onAccept?: (quoteId: string) => void;
  onViewProvider?: (providerId: string) => void;
  onViewQuote?: (quoteId: string) => void;
}

function formatPrice(price: number, currency: string): string {
  return new Intl.NumberFormat('en-EU', {
    style: 'currency',
    currency,
  }).format(price);
}

function formatDate(dateString?: string): string {
  if (!dateString) return '-';
  return new Date(dateString).toLocaleDateString('en-EU', {
    month: 'short',
    day: 'numeric',
    year: 'numeric',
  });
}

export function QuoteComparisonTable({
  quotes,
  onAccept,
  onViewProvider,
  onViewQuote,
}: QuoteComparisonTableProps) {
  const { t } = useTranslation();

  if (quotes.length === 0) {
    return (
      <div className="bg-white rounded-lg shadow p-8 text-center">
        <svg
          className="mx-auto w-12 h-12 text-gray-400"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <title>{t('marketplace.comparisonTable.noQuotes')}</title>
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
          />
        </svg>
        <p className="mt-2 text-gray-500">{t('marketplace.comparisonTable.noQuotesToCompare')}</p>
      </div>
    );
  }

  // Find the best values for highlighting
  const prices = quotes.map((q) => q.price);
  const lowestPrice = Math.min(...prices);
  const ratings = quotes.filter((q) => q.providerRating).map((q) => q.providerRating!);
  const highestRating = ratings.length > 0 ? Math.max(...ratings) : null;
  const warranties = quotes.filter((q) => q.warrantyPeriodDays).map((q) => q.warrantyPeriodDays!);
  const longestWarranty = warranties.length > 0 ? Math.max(...warranties) : null;

  const comparisonRows: {
    label: string;
    getValue: (q: QuoteSummary) => string;
    isBest: (q: QuoteSummary) => boolean;
    highlight: boolean;
  }[] = [
    {
      label: t('marketplace.comparisonTable.price'),
      getValue: (q: QuoteSummary) => formatPrice(q.price, q.currency),
      isBest: (q: QuoteSummary) => q.price === lowestPrice,
      highlight: true,
    },
    {
      label: t('marketplace.comparisonTable.providerRating'),
      getValue: (q: QuoteSummary) =>
        q.providerRating
          ? t('marketplace.comparisonTable.ratingOutOf', { rating: q.providerRating.toFixed(1) })
          : '-',
      isBest: (q: QuoteSummary) => q.providerRating === highestRating,
      highlight: true,
    },
    {
      label: t('marketplace.comparisonTable.verified'),
      getValue: (q: QuoteSummary) =>
        q.providerIsVerified
          ? t('marketplace.comparisonTable.yes')
          : t('marketplace.comparisonTable.no'),
      isBest: (q: QuoteSummary) => q.providerIsVerified === true,
      highlight: false,
    },
    {
      label: t('marketplace.comparisonTable.startDate'),
      getValue: (q: QuoteSummary) => formatDate(q.estimatedStartDate),
      isBest: () => false,
      highlight: false,
    },
    {
      label: t('marketplace.comparisonTable.endDate'),
      getValue: (q: QuoteSummary) => formatDate(q.estimatedEndDate),
      isBest: () => false,
      highlight: false,
    },
    {
      label: t('marketplace.comparisonTable.duration'),
      getValue: (q: QuoteSummary) =>
        q.estimatedDurationDays
          ? t('marketplace.comparisonTable.durationDays', { count: q.estimatedDurationDays })
          : '-',
      isBest: () => false,
      highlight: false,
    },
    {
      label: t('marketplace.comparisonTable.warranty'),
      getValue: (q: QuoteSummary) =>
        q.warrantyPeriodDays
          ? t('marketplace.comparisonTable.durationDays', { count: q.warrantyPeriodDays })
          : '-',
      isBest: (q: QuoteSummary) => q.warrantyPeriodDays === longestWarranty,
      highlight: true,
    },
    {
      label: t('marketplace.comparisonTable.validUntil'),
      getValue: (q: QuoteSummary) => formatDate(q.validUntil),
      isBest: () => false,
      highlight: false,
    },
  ];

  return (
    <div className="bg-white rounded-lg shadow overflow-hidden">
      <div className="overflow-x-auto">
        <table className="min-w-full divide-y divide-gray-200">
          <thead className="bg-gray-50">
            <tr>
              <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider w-40">
                {t('marketplace.comparisonTable.criteria')}
              </th>
              {quotes.map((quote) => (
                <th key={quote.id} className="px-6 py-3 text-center min-w-[200px]">
                  <div className="flex flex-col items-center">
                    {quote.providerLogoUrl ? (
                      <img
                        src={quote.providerLogoUrl}
                        alt={`${quote.providerName} logo`}
                        className="w-10 h-10 rounded-lg object-cover mb-2"
                      />
                    ) : (
                      <div className="w-10 h-10 rounded-lg bg-gray-200 flex items-center justify-center mb-2">
                        <svg
                          className="w-5 h-5 text-gray-400"
                          fill="none"
                          stroke="currentColor"
                          viewBox="0 0 24 24"
                        >
                          <title>{t('marketplace.common.company')}</title>
                          <path
                            strokeLinecap="round"
                            strokeLinejoin="round"
                            strokeWidth={2}
                            d="M19 21V5a2 2 0 00-2-2H7a2 2 0 00-2 2v16m14 0h2m-2 0h-5m-9 0H3m2 0h5M9 7h1m-1 4h1m4-4h1m-1 4h1m-5 10v-5a1 1 0 011-1h2a1 1 0 011 1v5m-4 0h4"
                          />
                        </svg>
                      </div>
                    )}
                    <button
                      type="button"
                      onClick={() => onViewProvider?.(quote.providerId)}
                      className="text-sm font-semibold text-gray-900 hover:text-blue-600"
                    >
                      {quote.providerName}
                    </button>
                    {quote.providerIsVerified && (
                      <span className="mt-1 inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-blue-100 text-blue-800">
                        {t('marketplace.common.verified')}
                      </span>
                    )}
                  </div>
                </th>
              ))}
            </tr>
          </thead>
          <tbody className="bg-white divide-y divide-gray-200">
            {comparisonRows.map((row) => (
              <tr key={row.label}>
                <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">
                  {row.label}
                </td>
                {quotes.map((quote) => {
                  const isBest = row.isBest(quote);
                  return (
                    <td
                      key={quote.id}
                      className={`px-6 py-4 whitespace-nowrap text-sm text-center ${
                        isBest && row.highlight
                          ? 'bg-green-50 text-green-700 font-semibold'
                          : 'text-gray-600'
                      }`}
                    >
                      {row.getValue(quote)}
                      {isBest && row.highlight && (
                        <span className="ml-2 text-xs text-green-600">
                          {t('marketplace.comparisonTable.best')}
                        </span>
                      )}
                    </td>
                  );
                })}
              </tr>
            ))}
            {/* Notes Row */}
            <tr>
              <td className="px-6 py-4 text-sm font-medium text-gray-900">
                {t('marketplace.comparisonTable.notes')}
              </td>
              {quotes.map((quote) => (
                <td key={quote.id} className="px-6 py-4 text-sm text-gray-600">
                  {quote.notes ? (
                    <p className="max-w-xs mx-auto text-left line-clamp-3">{quote.notes}</p>
                  ) : (
                    <span className="text-gray-400">-</span>
                  )}
                </td>
              ))}
            </tr>
            {/* Actions Row */}
            <tr className="bg-gray-50">
              <td className="px-6 py-4 text-sm font-medium text-gray-900">
                {t('marketplace.comparisonTable.actions')}
              </td>
              {quotes.map((quote) => (
                <td key={quote.id} className="px-6 py-4 text-center">
                  <div className="flex flex-col items-center gap-2">
                    <button
                      type="button"
                      onClick={() => onViewQuote?.(quote.id)}
                      className="text-sm font-medium text-blue-600 hover:text-blue-800"
                    >
                      {t('marketplace.common.viewDetails')}
                    </button>
                    {quote.status === 'submitted' && (
                      <button
                        type="button"
                        onClick={() => onAccept?.(quote.id)}
                        className="px-4 py-2 bg-green-600 text-white text-sm font-medium rounded-lg hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-green-500 focus:ring-offset-2"
                      >
                        {t('marketplace.comparisonTable.acceptQuote')}
                      </button>
                    )}
                  </div>
                </td>
              ))}
            </tr>
          </tbody>
        </table>
      </div>

      {/* Legend */}
      <div className="px-6 py-3 bg-gray-50 border-t text-sm text-gray-500">
        <span className="inline-flex items-center gap-2">
          <span className="w-3 h-3 bg-green-100 rounded" />
          <span>{t('marketplace.comparisonTable.bestValue')}</span>
        </span>
      </div>
    </div>
  );
}
