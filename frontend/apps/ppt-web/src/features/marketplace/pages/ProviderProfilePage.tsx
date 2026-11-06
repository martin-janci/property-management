/**
 * ProviderProfilePage - provider's own profile management page.
 * Epic 68: Service Provider Marketplace (Story 68.1, 68.4, 68.5)
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { ProviderSummary } from '../components/ProviderCard';
import {
  ProviderProfileForm,
  type ProviderProfileFormData,
} from '../components/ProviderProfileForm';
import { RatingBreakdown, type RatingBreakdownData } from '../components/RatingBreakdown';
import { type ReviewData, ReviewList } from '../components/ReviewCard';
import { ResponseForm } from '../components/ReviewForm';
import type { Badge, Verification } from '../components/VerificationBadge';
import { BadgeList, VerificationList } from '../components/VerificationBadge';
import { VerificationForm, type VerificationFormData } from '../components/VerificationForm';

interface ProviderProfilePageProps {
  profile: ProviderSummary & {
    description?: string;
    website?: string;
    contactPhone?: string;
    contactEmail?: string;
    contactName?: string;
    address?: string;
    city?: string;
    postalCode?: string;
    country?: string;
    businessRegistrationNumber?: string;
    taxId?: string;
    coverageRadiusKm?: number;
    coverageRegions?: string[];
    coveragePostalCodes?: string[];
    responseTimeHours?: number;
    serviceDescription?: string;
    specializations?: string[];
    portfolioImages?: string[];
    portfolioDescription?: string;
  };
  badges: Badge[];
  verifications: Verification[];
  ratingBreakdown: RatingBreakdownData;
  reviews: ReviewData[];
  pendingReviews: number;
  isLoading?: boolean;
  onUpdateProfile: (data: ProviderProfileFormData) => void;
  onSubmitVerification: (data: VerificationFormData) => void;
  onRespondToReview: (reviewId: string, response: string) => void;
}

type ViewType = 'dashboard' | 'edit-profile' | 'add-verification' | 'respond-review';

export function ProviderProfilePage({
  profile,
  badges,
  verifications,
  ratingBreakdown,
  reviews,
  pendingReviews,
  isLoading,
  onUpdateProfile,
  onSubmitVerification,
  onRespondToReview,
}: ProviderProfilePageProps) {
  const { t } = useTranslation();
  const [currentView, setCurrentView] = useState<ViewType>('dashboard');
  const [selectedReviewId, setSelectedReviewId] = useState<string | null>(null);

  const handleRespondToReview = (reviewId: string) => {
    setSelectedReviewId(reviewId);
    setCurrentView('respond-review');
  };

  const handleSubmitResponse = (reviewId: string, response: string) => {
    onRespondToReview(reviewId, response);
    setCurrentView('dashboard');
    setSelectedReviewId(null);
  };

  // Edit Profile View
  if (currentView === 'edit-profile') {
    const initialData: Partial<ProviderProfileFormData> = {
      companyName: profile.companyName,
      description: profile.description,
      logoUrl: profile.logoUrl,
      website: profile.website,
      contactName: profile.contactName || '',
      contactEmail: profile.contactEmail || '',
      contactPhone: profile.contactPhone,
      address: profile.address,
      city: profile.city,
      postalCode: profile.postalCode,
      country: profile.country,
      businessRegistrationNumber: profile.businessRegistrationNumber,
      taxId: profile.taxId,
      serviceCategories: profile.serviceCategories,
      serviceDescription: profile.serviceDescription,
      specializations: profile.specializations,
      coverageRadiusKm: profile.coverageRadiusKm,
      coverageRegions: profile.coverageRegions,
      coveragePostalCodes: profile.coveragePostalCodes,
      pricingType: profile.pricingType,
      hourlyRateMin: profile.hourlyRateMin,
      hourlyRateMax: profile.hourlyRateMax,
      currency: profile.currency,
      responseTimeHours: profile.responseTimeHours,
      emergencyAvailable: profile.emergencyAvailable,
      portfolioImages: profile.portfolioImages,
      portfolioDescription: profile.portfolioDescription,
    };

    return (
      <div className="max-w-4xl mx-auto px-4 py-8">
        <ProviderProfileForm
          initialData={initialData}
          onSubmit={(data) => {
            onUpdateProfile(data);
            setCurrentView('dashboard');
          }}
          onCancel={() => setCurrentView('dashboard')}
          isLoading={isLoading}
          isEdit
        />
      </div>
    );
  }

  // Add Verification View
  if (currentView === 'add-verification') {
    return (
      <div className="max-w-2xl mx-auto px-4 py-8">
        <VerificationForm
          onSubmit={(data) => {
            onSubmitVerification(data);
            setCurrentView('dashboard');
          }}
          onCancel={() => setCurrentView('dashboard')}
          isLoading={isLoading}
        />
      </div>
    );
  }

  // Respond to Review View
  if (currentView === 'respond-review' && selectedReviewId) {
    return (
      <div className="max-w-2xl mx-auto px-4 py-8">
        <button
          type="button"
          onClick={() => {
            setCurrentView('dashboard');
            setSelectedReviewId(null);
          }}
          className="flex items-center gap-2 text-gray-600 hover:text-gray-900 mb-6"
        >
          <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <title>{t('marketplace.common.back')}</title>
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M15 19l-7-7 7-7"
            />
          </svg>
          {t('marketplace.providerProfilePage.backToDashboard')}
        </button>
        <ResponseForm
          reviewId={selectedReviewId}
          onSubmit={handleSubmitResponse}
          onCancel={() => {
            setCurrentView('dashboard');
            setSelectedReviewId(null);
          }}
          isLoading={isLoading}
        />
      </div>
    );
  }

  // Dashboard View
  return (
    <div className="max-w-5xl mx-auto px-4 py-8">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold text-gray-900">
          {t('marketplace.providerProfilePage.dashboard')}
        </h1>
        <button
          type="button"
          onClick={() => setCurrentView('edit-profile')}
          className="px-4 py-2 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2"
        >
          {t('marketplace.providerProfilePage.editProfile')}
        </button>
      </div>

      {/* Profile Summary */}
      <div className="bg-white rounded-lg shadow p-6 mb-6">
        <div className="flex items-start gap-6">
          {profile.logoUrl ? (
            <img
              src={profile.logoUrl}
              alt={`${profile.companyName} logo`}
              className="w-20 h-20 rounded-lg object-cover"
            />
          ) : (
            <div className="w-20 h-20 rounded-lg bg-gray-200 flex items-center justify-center">
              <svg
                className="w-10 h-10 text-gray-400"
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
          <div className="flex-1">
            <div className="flex items-center gap-2">
              <h2 className="text-xl font-semibold text-gray-900">{profile.companyName}</h2>
              {profile.isVerified && (
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
            {profile.city && <p className="text-gray-500">{profile.city}</p>}
            {badges.length > 0 && (
              <div className="mt-3">
                <BadgeList badges={badges} size="sm" />
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Stats Cards */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4 mb-6">
        <div className="bg-white rounded-lg shadow p-4 text-center">
          <p className="text-3xl font-bold text-gray-900">
            {ratingBreakdown.averageOverall.toFixed(1)}
          </p>
          <p className="text-sm text-gray-500">
            {t('marketplace.providerProfilePage.averageRating')}
          </p>
        </div>
        <div className="bg-white rounded-lg shadow p-4 text-center">
          <p className="text-3xl font-bold text-gray-900">{ratingBreakdown.totalReviews}</p>
          <p className="text-sm text-gray-500">
            {t('marketplace.providerProfilePage.totalReviews')}
          </p>
        </div>
        <div className="bg-white rounded-lg shadow p-4 text-center">
          <p className="text-3xl font-bold text-gray-900">{profile.totalJobsCompleted || 0}</p>
          <p className="text-sm text-gray-500">
            {t('marketplace.providerProfilePage.jobsCompleted')}
          </p>
        </div>
        <div className="bg-white rounded-lg shadow p-4 text-center">
          <p className="text-3xl font-bold text-blue-600">{pendingReviews}</p>
          <p className="text-sm text-gray-500">
            {t('marketplace.providerProfilePage.pendingResponses')}
          </p>
        </div>
      </div>

      {/* Verifications Section */}
      <div className="bg-white rounded-lg shadow p-6 mb-6">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-lg font-medium text-gray-900">
            {t('marketplace.providerProfilePage.verifications')}
          </h3>
          <button
            type="button"
            onClick={() => setCurrentView('add-verification')}
            className="text-sm font-medium text-blue-600 hover:text-blue-800"
          >
            {t('marketplace.providerProfilePage.addVerification')}
          </button>
        </div>
        <VerificationList verifications={verifications} />
      </div>

      {/* Rating Breakdown */}
      <div className="mb-6">
        <RatingBreakdown data={ratingBreakdown} />
      </div>

      {/* Recent Reviews */}
      <div className="bg-white rounded-lg shadow p-6">
        <h3 className="text-lg font-medium text-gray-900 mb-4">
          {t('marketplace.providerProfilePage.recentReviews')}
        </h3>
        <ReviewList reviews={reviews} isProviderView onRespond={handleRespondToReview} />
      </div>
    </div>
  );
}
