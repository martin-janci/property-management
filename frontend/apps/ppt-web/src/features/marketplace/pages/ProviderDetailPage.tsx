/**
 * ProviderDetailPage - detailed view of a service provider.
 * Epic 68: Service Provider Marketplace (Story 68.1, 68.4, 68.5)
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { ProviderSummary } from '../components/ProviderCard';
import { RatingBreakdown, type RatingBreakdownData } from '../components/RatingBreakdown';
import { RatingStars } from '../components/RatingStars';
import { type ReviewData, ReviewList } from '../components/ReviewCard';
import type { Badge, Verification } from '../components/VerificationBadge';
import { BadgeList, VerificationList } from '../components/VerificationBadge';

interface ProviderDetailPageProps {
  provider: ProviderSummary & {
    description?: string;
    website?: string;
    contactPhone?: string;
    contactEmail?: string;
    address?: string;
    city?: string;
    coverageRadiusKm?: number;
    coverageRegions?: string[];
    responseTimeHours?: number;
    serviceDescription?: string;
    portfolioImages?: string[];
    portfolioDescription?: string;
  };
  badges: Badge[];
  verifications: Verification[];
  ratingBreakdown: RatingBreakdownData;
  reviews: ReviewData[];
  isLoading?: boolean;
  onRequestQuote: () => void;
  onContact: () => void;
  onBack: () => void;
  onMarkHelpful: (reviewId: string) => void;
  onReportReview: (reviewId: string) => void;
}

type TabType = 'overview' | 'services' | 'portfolio' | 'reviews' | 'verifications';

export function ProviderDetailPage({
  provider,
  badges,
  verifications,
  ratingBreakdown,
  reviews,
  isLoading,
  onRequestQuote,
  onContact,
  onBack,
  onMarkHelpful,
  onReportReview,
}: ProviderDetailPageProps) {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<TabType>('overview');

  const tabs: { id: TabType; label: string }[] = [
    { id: 'overview', label: t('marketplace.providerDetailPage.tabOverview') },
    { id: 'services', label: t('marketplace.providerDetailPage.tabServices') },
    { id: 'portfolio', label: t('marketplace.providerDetailPage.tabPortfolio') },
    {
      id: 'reviews',
      label: t('marketplace.providerDetailPage.tabReviews', {
        count: ratingBreakdown.totalReviews,
      }),
    },
    { id: 'verifications', label: t('marketplace.providerDetailPage.tabVerifications') },
  ];

  if (isLoading) {
    return (
      <div className="max-w-5xl mx-auto px-4 py-8">
        <div className="animate-pulse">
          <div className="h-8 bg-gray-200 rounded w-1/4 mb-4" />
          <div className="h-40 bg-gray-200 rounded mb-6" />
          <div className="grid grid-cols-3 gap-4">
            <div className="h-20 bg-gray-200 rounded" />
            <div className="h-20 bg-gray-200 rounded" />
            <div className="h-20 bg-gray-200 rounded" />
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="max-w-5xl mx-auto px-4 py-8">
      {/* Back Button */}
      <button
        type="button"
        onClick={onBack}
        className="flex items-center gap-2 text-gray-600 hover:text-gray-900 mb-6"
      >
        <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <title>{t('marketplace.common.back')}</title>
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
        </svg>
        {t('marketplace.providerDetailPage.backToMarketplace')}
      </button>

      {/* Header */}
      <div className="bg-white rounded-lg shadow p-6">
        <div className="flex flex-col md:flex-row md:items-start gap-6">
          {/* Logo */}
          <div className="flex-shrink-0">
            {provider.logoUrl ? (
              <img
                src={provider.logoUrl}
                alt={`${provider.companyName} logo`}
                className="w-24 h-24 rounded-lg object-cover"
              />
            ) : (
              <div className="w-24 h-24 rounded-lg bg-gray-200 flex items-center justify-center">
                <svg
                  className="w-12 h-12 text-gray-400"
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

          {/* Info */}
          <div className="flex-1">
            <div className="flex items-start justify-between">
              <div>
                <div className="flex items-center gap-2">
                  <h1 className="text-2xl font-bold text-gray-900">{provider.companyName}</h1>
                  {provider.isVerified && (
                    <svg className="w-6 h-6 text-blue-500" fill="currentColor" viewBox="0 0 20 20">
                      <title>{t('marketplace.common.verified')}</title>
                      <path
                        fillRule="evenodd"
                        d="M6.267 3.455a3.066 3.066 0 001.745-.723 3.066 3.066 0 013.976 0 3.066 3.066 0 001.745.723 3.066 3.066 0 012.812 2.812c.051.643.304 1.254.723 1.745a3.066 3.066 0 010 3.976 3.066 3.066 0 00-.723 1.745 3.066 3.066 0 01-2.812 2.812 3.066 3.066 0 00-1.745.723 3.066 3.066 0 01-3.976 0 3.066 3.066 0 00-1.745-.723 3.066 3.066 0 01-2.812-2.812 3.066 3.066 0 00-.723-1.745 3.066 3.066 0 010-3.976 3.066 3.066 0 00.723-1.745 3.066 3.066 0 012.812-2.812zm7.44 5.252a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
                        clipRule="evenodd"
                      />
                    </svg>
                  )}
                </div>
                {provider.city && <p className="text-gray-500">{provider.city}</p>}
              </div>
              {provider.emergencyAvailable && (
                <span className="inline-flex items-center px-3 py-1 rounded text-sm font-medium bg-red-100 text-red-800">
                  {t('marketplace.providerCard.emergency247')}
                </span>
              )}
            </div>

            {/* Rating */}
            <div className="mt-4 flex items-center gap-4">
              <RatingStars rating={provider.averageRating || 0} size="lg" showValue />
              <span className="text-gray-500">
                {t('marketplace.providerDetailPage.reviewsCount', {
                  count: ratingBreakdown.totalReviews,
                })}
              </span>
              {provider.totalJobsCompleted && (
                <span className="text-gray-500">
                  {t('marketplace.providerDetailPage.jobsCompleted', {
                    count: provider.totalJobsCompleted,
                  })}
                </span>
              )}
            </div>

            {/* Badges */}
            {badges.length > 0 && (
              <div className="mt-4">
                <BadgeList badges={badges} size="md" />
              </div>
            )}

            {/* Description */}
            {provider.description && <p className="mt-4 text-gray-600">{provider.description}</p>}
          </div>

          {/* Actions */}
          <div className="flex flex-col gap-3 md:w-48">
            <button
              type="button"
              onClick={onRequestQuote}
              className="w-full px-4 py-2 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2"
            >
              {t('marketplace.common.requestQuote')}
            </button>
            <button
              type="button"
              onClick={onContact}
              className="w-full px-4 py-2 border border-gray-300 text-gray-700 font-medium rounded-lg hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2"
            >
              {t('marketplace.common.contact')}
            </button>
          </div>
        </div>
      </div>

      {/* Tabs */}
      <div className="mt-6 border-b border-gray-200">
        <nav className="flex gap-8">
          {tabs.map((tab) => (
            <button
              key={tab.id}
              type="button"
              onClick={() => setActiveTab(tab.id)}
              className={`py-4 text-sm font-medium border-b-2 -mb-px ${
                activeTab === tab.id
                  ? 'border-blue-500 text-blue-600'
                  : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
              }`}
            >
              {tab.label}
            </button>
          ))}
        </nav>
      </div>

      {/* Tab Content */}
      <div className="mt-6">
        {activeTab === 'overview' && (
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            {/* Quick Info */}
            <div className="bg-white rounded-lg shadow p-6">
              <h3 className="text-lg font-medium text-gray-900 mb-4">
                {t('marketplace.providerDetailPage.contactInformation')}
              </h3>
              <dl className="space-y-3">
                {provider.contactEmail && (
                  <div>
                    <dt className="text-sm text-gray-500">
                      {t('marketplace.providerDetailPage.email')}
                    </dt>
                    <dd className="text-gray-900">{provider.contactEmail}</dd>
                  </div>
                )}
                {provider.contactPhone && (
                  <div>
                    <dt className="text-sm text-gray-500">
                      {t('marketplace.providerDetailPage.phone')}
                    </dt>
                    <dd className="text-gray-900">{provider.contactPhone}</dd>
                  </div>
                )}
                {provider.website && (
                  <div>
                    <dt className="text-sm text-gray-500">
                      {t('marketplace.providerDetailPage.website')}
                    </dt>
                    <dd>
                      <a
                        href={provider.website}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="text-blue-600 hover:underline"
                      >
                        {provider.website}
                      </a>
                    </dd>
                  </div>
                )}
                {provider.address && (
                  <div>
                    <dt className="text-sm text-gray-500">
                      {t('marketplace.providerDetailPage.address')}
                    </dt>
                    <dd className="text-gray-900">{provider.address}</dd>
                  </div>
                )}
              </dl>
            </div>

            {/* Coverage */}
            <div className="bg-white rounded-lg shadow p-6">
              <h3 className="text-lg font-medium text-gray-900 mb-4">
                {t('marketplace.providerDetailPage.coverageArea')}
              </h3>
              <dl className="space-y-3">
                {provider.coverageRadiusKm && (
                  <div>
                    <dt className="text-sm text-gray-500">
                      {t('marketplace.providerDetailPage.serviceRadius')}
                    </dt>
                    <dd className="text-gray-900">
                      {t('marketplace.providerDetailPage.radiusKm', {
                        count: provider.coverageRadiusKm,
                      })}
                    </dd>
                  </div>
                )}
                {provider.coverageRegions && provider.coverageRegions.length > 0 && (
                  <div>
                    <dt className="text-sm text-gray-500">
                      {t('marketplace.providerDetailPage.regionsServed')}
                    </dt>
                    <dd className="text-gray-900">{provider.coverageRegions.join(', ')}</dd>
                  </div>
                )}
                {provider.responseTimeHours && (
                  <div>
                    <dt className="text-sm text-gray-500">
                      {t('marketplace.providerDetailPage.responseTime')}
                    </dt>
                    <dd className="text-gray-900">
                      {t('marketplace.providerDetailPage.respondsWithinHours', {
                        count: provider.responseTimeHours,
                      })}
                    </dd>
                  </div>
                )}
              </dl>
            </div>

            {/* Rating Summary */}
            <div className="md:col-span-2">
              <RatingBreakdown data={ratingBreakdown} />
            </div>
          </div>
        )}

        {activeTab === 'services' && (
          <div className="bg-white rounded-lg shadow p-6">
            <h3 className="text-lg font-medium text-gray-900 mb-4">
              {t('marketplace.providerDetailPage.servicesOffered')}
            </h3>
            <div className="flex flex-wrap gap-2 mb-6">
              {provider.serviceCategories.map((cat) => (
                <span
                  key={cat}
                  className="px-3 py-1 bg-blue-100 text-blue-800 rounded-full text-sm font-medium"
                >
                  {t(`marketplace.serviceCategory.${cat}`)}
                </span>
              ))}
            </div>
            {provider.serviceDescription && (
              <div>
                <h4 className="font-medium text-gray-900 mb-2">
                  {t('marketplace.providerDetailPage.descriptionHeading')}
                </h4>
                <p className="text-gray-600">{provider.serviceDescription}</p>
              </div>
            )}
          </div>
        )}

        {activeTab === 'portfolio' && (
          <div className="bg-white rounded-lg shadow p-6">
            <h3 className="text-lg font-medium text-gray-900 mb-4">
              {t('marketplace.providerDetailPage.portfolio')}
            </h3>
            {provider.portfolioDescription && (
              <p className="text-gray-600 mb-6">{provider.portfolioDescription}</p>
            )}
            {provider.portfolioImages && provider.portfolioImages.length > 0 ? (
              <div className="grid grid-cols-2 md:grid-cols-3 gap-4">
                {provider.portfolioImages.map((img, idx) => (
                  <img
                    key={`portfolio-${idx}`}
                    src={img}
                    alt={`Portfolio ${idx + 1}`}
                    className="w-full h-48 object-cover rounded-lg"
                  />
                ))}
              </div>
            ) : (
              <p className="text-gray-500 text-center py-8">
                {t('marketplace.providerDetailPage.noPortfolioImages')}
              </p>
            )}
          </div>
        )}

        {activeTab === 'reviews' && (
          <div className="space-y-6">
            <RatingBreakdown data={ratingBreakdown} />
            <ReviewList reviews={reviews} onMarkHelpful={onMarkHelpful} onReport={onReportReview} />
          </div>
        )}

        {activeTab === 'verifications' && (
          <div className="bg-white rounded-lg shadow p-6">
            <h3 className="text-lg font-medium text-gray-900 mb-4">
              {t('marketplace.providerDetailPage.verificationsCredentials')}
            </h3>
            <VerificationList verifications={verifications} />
          </div>
        )}
      </div>
    </div>
  );
}
