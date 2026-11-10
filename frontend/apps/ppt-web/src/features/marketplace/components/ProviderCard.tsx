/**
 * ProviderCard component - displays a service provider summary in marketplace listings.
 * Epic 68: Service Provider Marketplace (Story 68.1, 68.2)
 */

import type { TFunction } from 'i18next';
import { useTranslation } from 'react-i18next';

export type ServiceCategory =
  | 'plumbing'
  | 'electrical'
  | 'hvac'
  | 'cleaning'
  | 'landscaping'
  | 'security'
  | 'painting'
  | 'roofing'
  | 'carpentry'
  | 'locksmith'
  | 'pest_control'
  | 'general_maintenance'
  | 'elevator_maintenance'
  | 'fire_safety'
  | 'waste_management'
  | 'other';

export type BadgeType =
  | 'verified_business'
  | 'insured'
  | 'certified'
  | 'top_rated'
  | 'fast_responder'
  | 'preferred';

export interface ProviderSummary {
  id: string;
  companyName: string;
  description?: string;
  logoUrl?: string;
  serviceCategories: ServiceCategory[];
  city?: string;
  pricingType: 'hourly' | 'project' | 'fixed' | 'quote_required';
  hourlyRateMin?: number;
  hourlyRateMax?: number;
  currency?: string;
  averageRating?: number;
  totalReviews?: number;
  totalJobsCompleted?: number;
  isVerified?: boolean;
  badges?: BadgeType[];
  emergencyAvailable?: boolean;
  responseTimeHours?: number;
}

interface ProviderCardProps {
  provider: ProviderSummary;
  onView?: (id: string) => void;
  onRequestQuote?: (id: string) => void;
  onContact?: (id: string) => void;
}

const badgeColors: Record<BadgeType, string> = {
  verified_business: 'bg-green-100 text-green-800',
  insured: 'bg-blue-100 text-blue-800',
  certified: 'bg-purple-100 text-purple-800',
  top_rated: 'bg-yellow-100 text-yellow-800',
  fast_responder: 'bg-cyan-100 text-cyan-800',
  preferred: 'bg-orange-100 text-orange-800',
};

function formatPriceRange(
  t: TFunction,
  min?: number,
  max?: number,
  currency = 'EUR',
  pricingType?: string
): string {
  if (pricingType === 'quote_required') {
    return t('marketplace.providerCard.quoteRequired');
  }
  if (!min && !max) {
    return t('marketplace.providerCard.contactForPricing');
  }
  const formatter = new Intl.NumberFormat('en-EU', {
    style: 'currency',
    currency,
  });
  if (min && max) {
    return t('marketplace.providerCard.perHourRange', {
      min: formatter.format(min),
      max: formatter.format(max),
    });
  }
  if (min) {
    return t('marketplace.providerCard.perHourFrom', { min: formatter.format(min) });
  }
  return t('marketplace.providerCard.perHourUpTo', { max: formatter.format(max!) });
}

function RatingDisplay({ rating, reviewCount }: { rating?: number; reviewCount?: number }) {
  const { t } = useTranslation();

  if (!rating) {
    return <span className="text-sm text-gray-500">{t('marketplace.providerCard.noReviews')}</span>;
  }

  const fullStars = Math.floor(rating);
  const hasHalfStar = rating % 1 >= 0.5;

  return (
    <div className="flex items-center gap-1">
      <div className="flex" aria-label={t('marketplace.common.starsAria', { rating })}>
        {[...Array(5)].map((_, i) => (
          <svg
            key={`star-${i}`}
            className={`w-4 h-4 ${
              i < fullStars
                ? 'text-yellow-400'
                : i === fullStars && hasHalfStar
                  ? 'text-yellow-300'
                  : 'text-gray-300'
            }`}
            fill="currentColor"
            viewBox="0 0 20 20"
          >
            <title>
              {i < rating ? t('marketplace.common.filledStar') : t('marketplace.common.emptyStar')}
            </title>
            <path d="M9.049 2.927c.3-.921 1.603-.921 1.902 0l1.07 3.292a1 1 0 00.95.69h3.462c.969 0 1.371 1.24.588 1.81l-2.8 2.034a1 1 0 00-.364 1.118l1.07 3.292c.3.921-.755 1.688-1.54 1.118l-2.8-2.034a1 1 0 00-1.175 0l-2.8 2.034c-.784.57-1.838-.197-1.539-1.118l1.07-3.292a1 1 0 00-.364-1.118L2.98 8.72c-.783-.57-.38-1.81.588-1.81h3.461a1 1 0 00.951-.69l1.07-3.292z" />
          </svg>
        ))}
      </div>
      <span className="text-sm font-medium text-gray-700">{rating.toFixed(1)}</span>
      {reviewCount !== undefined && (
        <span className="text-sm text-gray-500">
          {reviewCount === 1
            ? t('marketplace.providerCard.reviewsCountOne', { count: reviewCount })
            : t('marketplace.providerCard.reviewsCountOther', { count: reviewCount })}
        </span>
      )}
    </div>
  );
}

export function ProviderCard({ provider, onView, onRequestQuote, onContact }: ProviderCardProps) {
  const { t } = useTranslation();
  const displayedCategories = provider.serviceCategories.slice(0, 3);
  const remainingCount = provider.serviceCategories.length - 3;

  return (
    <div className="bg-white rounded-lg shadow p-6 hover:shadow-lg transition-shadow">
      <div className="flex items-start gap-4">
        {/* Logo */}
        <div className="flex-shrink-0">
          {provider.logoUrl ? (
            <img
              src={provider.logoUrl}
              alt={`${provider.companyName} logo`}
              className="w-16 h-16 rounded-lg object-cover"
            />
          ) : (
            <div className="w-16 h-16 rounded-lg bg-gray-200 flex items-center justify-center">
              <svg
                className="w-8 h-8 text-gray-400"
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
        </div>

        {/* Content */}
        <div className="flex-1 min-w-0">
          <div className="flex items-start justify-between">
            <div>
              <div className="flex items-center gap-2">
                <h3 className="text-lg font-semibold text-gray-900 truncate">
                  {provider.companyName}
                </h3>
                {provider.isVerified && (
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
              {provider.city && <p className="text-sm text-gray-500 mt-0.5">{provider.city}</p>}
            </div>
            {provider.emergencyAvailable && (
              <span className="inline-flex items-center px-2 py-1 rounded text-xs font-medium bg-red-100 text-red-800">
                {t('marketplace.providerCard.emergency247')}
              </span>
            )}
          </div>

          {/* Description */}
          {provider.description && (
            <p className="mt-2 text-sm text-gray-600 line-clamp-2">{provider.description}</p>
          )}

          {/* Service Categories */}
          <div className="mt-3 flex flex-wrap gap-1">
            {displayedCategories.map((cat) => (
              <span
                key={cat}
                className="inline-block px-2 py-0.5 text-xs font-medium bg-gray-100 text-gray-700 rounded"
              >
                {t(`marketplace.serviceCategory.${cat}`)}
              </span>
            ))}
            {remainingCount > 0 && (
              <span className="inline-block px-2 py-0.5 text-xs font-medium bg-gray-100 text-gray-500 rounded">
                {t('marketplace.providerCard.moreCount', { count: remainingCount })}
              </span>
            )}
          </div>

          {/* Badges */}
          {provider.badges && provider.badges.length > 0 && (
            <div className="mt-2 flex flex-wrap gap-1">
              {provider.badges.map((badge) => (
                <span
                  key={badge}
                  className={`inline-block px-2 py-0.5 text-xs font-medium rounded ${badgeColors[badge]}`}
                >
                  {t(`marketplace.verificationBadge.badge.${badge}`)}
                </span>
              ))}
            </div>
          )}

          {/* Rating and Price */}
          <div className="mt-4 flex items-center justify-between">
            <RatingDisplay rating={provider.averageRating} reviewCount={provider.totalReviews} />
            <div className="text-right">
              <p className="text-sm font-medium text-gray-900">
                {formatPriceRange(
                  t,
                  provider.hourlyRateMin,
                  provider.hourlyRateMax,
                  provider.currency,
                  provider.pricingType
                )}
              </p>
              {provider.totalJobsCompleted !== undefined && provider.totalJobsCompleted > 0 && (
                <p className="text-xs text-gray-500">
                  {t('marketplace.providerCard.jobsCompleted', {
                    count: provider.totalJobsCompleted,
                  })}
                </p>
              )}
            </div>
          </div>

          {/* Response time */}
          {provider.responseTimeHours && (
            <p className="mt-2 text-xs text-gray-500">
              {t('marketplace.providerCard.respondsWithin', { hours: provider.responseTimeHours })}
            </p>
          )}
        </div>
      </div>

      {/* Actions */}
      <div className="mt-4 flex items-center gap-3 pt-4 border-t">
        <button
          type="button"
          onClick={() => onView?.(provider.id)}
          className="text-sm font-medium text-blue-600 hover:text-blue-800"
        >
          {t('marketplace.providerCard.viewProfile')}
        </button>
        <button
          type="button"
          onClick={() => onRequestQuote?.(provider.id)}
          className="text-sm font-medium text-green-600 hover:text-green-800"
        >
          {t('marketplace.common.requestQuote')}
        </button>
        <button
          type="button"
          onClick={() => onContact?.(provider.id)}
          className="text-sm font-medium text-gray-600 hover:text-gray-800"
        >
          {t('marketplace.common.contact')}
        </button>
      </div>
    </div>
  );
}
