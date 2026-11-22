/**
 * QuoteComparisonCard component - displays an individual insurance quote.
 * Insurance Management Feature (UC-35)
 */

import { useTranslation } from 'react-i18next';
import { getActiveLocale } from '../../../lib/locale';
import type { InsuranceQuote } from '../types';

interface QuoteComparisonCardProps {
  quote: InsuranceQuote;
  showAnnual: boolean;
  onSelect: (quoteId: string) => void;
  isSelected?: boolean;
}

export function QuoteComparisonCard({
  quote,
  showAnnual,
  onSelect,
  isSelected,
}: QuoteComparisonCardProps) {
  const { t } = useTranslation();

  const formatCurrency = (amount: number, currency: string) => {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: currency,
    }).format(amount);
  };

  const premium = showAnnual ? quote.premiumAnnual : quote.premiumMonthly;
  const periodLabel = showAnnual ? t('insurance.quotes.perYear') : t('insurance.quotes.perMonth');

  return (
    <div
      className={`relative bg-white rounded-lg shadow-md p-6 transition-all ${
        isSelected ? 'ring-2 ring-blue-500 shadow-lg' : 'hover:shadow-lg'
      } ${quote.isBestValue || quote.isLowestPrice ? 'border-t-4' : ''} ${
        quote.isBestValue ? 'border-green-500' : ''
      } ${quote.isLowestPrice && !quote.isBestValue ? 'border-blue-500' : ''}`}
    >
      {/* Badges */}
      <div className="absolute -top-3 left-4 flex gap-2">
        {quote.isBestValue && (
          <span className="px-2 py-1 text-xs font-semibold rounded-full bg-green-100 text-green-800">
            {t('insurance.quotes.bestValue')}
          </span>
        )}
        {quote.isLowestPrice && (
          <span className="px-2 py-1 text-xs font-semibold rounded-full bg-blue-100 text-blue-800">
            {t('insurance.quotes.lowestPrice')}
          </span>
        )}
      </div>

      {/* Provider Info */}
      <div className="flex items-center gap-3 mt-2">
        <div className="w-12 h-12 rounded-full bg-gray-100 flex items-center justify-center">
          {quote.provider.logoUrl ? (
            <img
              src={quote.provider.logoUrl}
              alt={quote.provider.name}
              className="w-10 h-10 rounded-full object-cover"
            />
          ) : (
            <span className="text-lg font-bold text-gray-500">{quote.provider.name.charAt(0)}</span>
          )}
        </div>
        <div>
          <h3 className="font-semibold text-gray-900">{quote.provider.name}</h3>
          {quote.provider.rating !== undefined && (
            <div className="flex items-center gap-1 text-sm text-gray-500">
              <svg className="w-4 h-4 text-yellow-400 fill-current" viewBox="0 0 20 20">
                <path d="M10 15l-5.878 3.09 1.123-6.545L.489 6.91l6.572-.955L10 0l2.939 5.955 6.572.955-4.756 4.635 1.123 6.545z" />
              </svg>
              <span>{quote.provider.rating.toFixed(1)}</span>
              {quote.provider.reviewCount !== undefined && (
                <span className="text-gray-400">
                  ({quote.provider.reviewCount} {t('insurance.quotes.reviews')})
                </span>
              )}
            </div>
          )}
        </div>
      </div>

      {/* Premium */}
      <div className="mt-4 text-center py-4 border-t border-b">
        <p className="text-3xl font-bold text-gray-900">
          {formatCurrency(premium, quote.currency)}
        </p>
        <p className="text-sm text-gray-500">{periodLabel}</p>
        {showAnnual && (
          <p className="text-xs text-gray-400 mt-1">
            ({formatCurrency(quote.premiumMonthly, quote.currency)}/{t('insurance.quotes.perMonth')}
            )
          </p>
        )}
      </div>

      {/* Coverage Details */}
      <div className="mt-4 space-y-3">
        <div className="flex justify-between text-sm">
          <span className="text-gray-600">{t('insurance.coverage')}:</span>
          <span className="font-medium">
            {formatCurrency(quote.coverageAmount, quote.currency)}
          </span>
        </div>
        <div className="flex justify-between text-sm">
          <span className="text-gray-600">{t('insurance.deductible')}:</span>
          <span className="font-medium">{formatCurrency(quote.deductible, quote.currency)}</span>
        </div>
      </div>

      {/* Coverage Limits */}
      {quote.coverageLimits.length > 0 && (
        <div className="mt-4">
          <h4 className="text-sm font-medium text-gray-700 mb-2">
            {t('insurance.quotes.coverageLimits')}
          </h4>
          <ul className="space-y-1">
            {quote.coverageLimits.map((limit, idx) => (
              <li key={idx} className="flex justify-between text-xs text-gray-600">
                <span>{limit.label}</span>
                <span className="font-medium">{limit.value}</span>
              </li>
            ))}
          </ul>
        </div>
      )}

      {/* Key Features */}
      {quote.features.length > 0 && (
        <div className="mt-4">
          <h4 className="text-sm font-medium text-gray-700 mb-2">
            {t('insurance.quotes.keyFeatures')}
          </h4>
          <ul className="space-y-1">
            {quote.features.slice(0, 4).map((feature, idx) => (
              <li key={idx} className="flex items-start gap-2 text-sm text-gray-600">
                <svg
                  className="w-4 h-4 text-green-500 flex-shrink-0 mt-0.5"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M5 13l4 4L19 7"
                  />
                </svg>
                <span>{feature}</span>
              </li>
            ))}
            {quote.features.length > 4 && (
              <li className="text-xs text-blue-600">
                +{quote.features.length - 4} {t('insurance.quotes.moreFeatures')}
              </li>
            )}
          </ul>
        </div>
      )}

      {/* Exclusions */}
      {quote.exclusions.length > 0 && (
        <div className="mt-4">
          <h4 className="text-sm font-medium text-gray-700 mb-2">
            {t('insurance.quotes.exclusions')}
          </h4>
          <ul className="space-y-1">
            {quote.exclusions.slice(0, 3).map((exclusion, idx) => (
              <li key={idx} className="flex items-start gap-2 text-sm text-gray-500">
                <svg
                  className="w-4 h-4 text-red-400 flex-shrink-0 mt-0.5"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M6 18L18 6M6 6l12 12"
                  />
                </svg>
                <span>{exclusion}</span>
              </li>
            ))}
            {quote.exclusions.length > 3 && (
              <li className="text-xs text-gray-400">
                +{quote.exclusions.length - 3} {t('insurance.quotes.moreExclusions')}
              </li>
            )}
          </ul>
        </div>
      )}

      {/* Valid Until */}
      <p className="mt-4 text-xs text-gray-400 text-center">
        {t('insurance.quotes.validUntil')}:{' '}
        {new Date(quote.validUntil).toLocaleDateString(getActiveLocale())}
      </p>

      {/* Select Button */}
      <button
        type="button"
        onClick={() => onSelect(quote.id)}
        className={`mt-4 w-full px-4 py-2 rounded-lg font-medium transition-colors ${
          isSelected ? 'bg-blue-600 text-white' : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
        }`}
      >
        {isSelected ? t('insurance.quotes.selected') : t('insurance.quotes.selectQuote')}
      </button>
    </div>
  );
}
