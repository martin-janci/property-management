/**
 * ReviewCard component - displays a provider review.
 * Epic 68: Service Provider Marketplace (Story 68.5)
 */

import { useTranslation } from 'react-i18next';
import { RatingStars } from './RatingStars';

export interface ReviewData {
  id: string;
  providerId: string;
  reviewerName: string;
  reviewerOrganization?: string;
  qualityRating: number;
  timelinessRating: number;
  communicationRating: number;
  valueRating: number;
  overallRating: number;
  reviewTitle?: string;
  reviewText?: string;
  providerResponse?: string;
  providerRespondedAt?: string;
  helpfulCount?: number;
  isHelpfulByCurrentUser?: boolean;
  createdAt: string;
}

interface ReviewCardProps {
  review: ReviewData;
  isProviderView?: boolean;
  onRespond?: (reviewId: string) => void;
  onMarkHelpful?: (reviewId: string) => void;
  onReport?: (reviewId: string) => void;
}

function formatDate(dateString: string): string {
  return new Date(dateString).toLocaleDateString('en-EU', {
    month: 'long',
    day: 'numeric',
    year: 'numeric',
  });
}

function getInitials(name: string): string {
  return name
    .split(' ')
    .map((n) => n[0])
    .join('')
    .toUpperCase()
    .slice(0, 2);
}

export function ReviewCard({
  review,
  isProviderView = false,
  onRespond,
  onMarkHelpful,
  onReport,
}: ReviewCardProps) {
  const { t } = useTranslation();
  const canRespond = isProviderView && !review.providerResponse;

  return (
    <div className="bg-white rounded-lg shadow p-6">
      {/* Header */}
      <div className="flex items-start justify-between">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-full bg-gray-200 flex items-center justify-center text-gray-600 font-medium">
            {getInitials(review.reviewerName)}
          </div>
          <div>
            <p className="font-medium text-gray-900">{review.reviewerName}</p>
            {review.reviewerOrganization && (
              <p className="text-sm text-gray-500">{review.reviewerOrganization}</p>
            )}
          </div>
        </div>
        <div className="text-right">
          <RatingStars rating={review.overallRating} size="sm" />
          <p className="text-xs text-gray-400 mt-1">{formatDate(review.createdAt)}</p>
        </div>
      </div>

      {/* Title and Review Text */}
      {review.reviewTitle && (
        <h3 className="mt-4 text-lg font-medium text-gray-900">{review.reviewTitle}</h3>
      )}
      {review.reviewText && <p className="mt-2 text-gray-600">{review.reviewText}</p>}

      {/* Rating Breakdown */}
      <div className="mt-4 grid grid-cols-2 md:grid-cols-4 gap-4">
        <div className="text-center p-2 bg-gray-50 rounded">
          <p className="text-xs text-gray-500">{t('marketplace.reviewCard.quality')}</p>
          <p className="font-medium text-gray-900">
            {t('marketplace.reviewCard.ratingOutOf', { rating: review.qualityRating })}
          </p>
        </div>
        <div className="text-center p-2 bg-gray-50 rounded">
          <p className="text-xs text-gray-500">{t('marketplace.reviewCard.timeliness')}</p>
          <p className="font-medium text-gray-900">
            {t('marketplace.reviewCard.ratingOutOf', { rating: review.timelinessRating })}
          </p>
        </div>
        <div className="text-center p-2 bg-gray-50 rounded">
          <p className="text-xs text-gray-500">{t('marketplace.reviewCard.communication')}</p>
          <p className="font-medium text-gray-900">
            {t('marketplace.reviewCard.ratingOutOf', { rating: review.communicationRating })}
          </p>
        </div>
        <div className="text-center p-2 bg-gray-50 rounded">
          <p className="text-xs text-gray-500">{t('marketplace.reviewCard.value')}</p>
          <p className="font-medium text-gray-900">
            {t('marketplace.reviewCard.ratingOutOf', { rating: review.valueRating })}
          </p>
        </div>
      </div>

      {/* Provider Response */}
      {review.providerResponse && (
        <div className="mt-4 p-4 bg-blue-50 rounded-lg border-l-4 border-blue-500">
          <p className="text-sm font-medium text-blue-800">
            {t('marketplace.reviewCard.providerResponse')}
          </p>
          <p className="mt-1 text-sm text-blue-700">{review.providerResponse}</p>
          {review.providerRespondedAt && (
            <p className="mt-2 text-xs text-blue-500">
              {t('marketplace.reviewCard.respondedOn', {
                date: formatDate(review.providerRespondedAt),
              })}
            </p>
          )}
        </div>
      )}

      {/* Actions */}
      <div className="mt-4 flex items-center justify-between pt-4 border-t">
        <div className="flex items-center gap-4">
          {onMarkHelpful && (
            <button
              type="button"
              onClick={() => onMarkHelpful(review.id)}
              className={`flex items-center gap-1 text-sm ${
                review.isHelpfulByCurrentUser
                  ? 'text-blue-600'
                  : 'text-gray-500 hover:text-gray-700'
              }`}
            >
              <svg
                className="w-4 h-4"
                fill={review.isHelpfulByCurrentUser ? 'currentColor' : 'none'}
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <title>{t('marketplace.reviewCard.helpful')}</title>
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M14 10h4.764a2 2 0 011.789 2.894l-3.5 7A2 2 0 0115.263 21h-4.017c-.163 0-.326-.02-.485-.06L7 20m7-10V5a2 2 0 00-2-2h-.095c-.5 0-.905.405-.905.905 0 .714-.211 1.412-.608 2.006L7 11v9m7-10h-2M7 20H5a2 2 0 01-2-2v-6a2 2 0 012-2h2.5"
                />
              </svg>
              {t('marketplace.reviewCard.helpfulCount', { count: review.helpfulCount || 0 })}
            </button>
          )}
          {onReport && (
            <button
              type="button"
              onClick={() => onReport(review.id)}
              className="text-sm text-gray-400 hover:text-gray-600"
            >
              {t('marketplace.reviewCard.report')}
            </button>
          )}
        </div>

        {canRespond && (
          <button
            type="button"
            onClick={() => onRespond?.(review.id)}
            className="text-sm font-medium text-blue-600 hover:text-blue-800"
          >
            {t('marketplace.reviewCard.respondToReview')}
          </button>
        )}
      </div>
    </div>
  );
}

interface ReviewListProps {
  reviews: ReviewData[];
  isProviderView?: boolean;
  onRespond?: (reviewId: string) => void;
  onMarkHelpful?: (reviewId: string) => void;
  onReport?: (reviewId: string) => void;
}

export function ReviewList({
  reviews,
  isProviderView,
  onRespond,
  onMarkHelpful,
  onReport,
}: ReviewListProps) {
  const { t } = useTranslation();

  if (reviews.length === 0) {
    return (
      <div className="bg-white rounded-lg shadow p-8 text-center">
        <svg
          className="mx-auto w-12 h-12 text-gray-400"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <title>{t('marketplace.reviewCard.noReviews')}</title>
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M11.049 2.927c.3-.921 1.603-.921 1.902 0l1.519 4.674a1 1 0 00.95.69h4.915c.969 0 1.371 1.24.588 1.81l-3.976 2.888a1 1 0 00-.363 1.118l1.518 4.674c.3.922-.755 1.688-1.538 1.118l-3.976-2.888a1 1 0 00-1.176 0l-3.976 2.888c-.783.57-1.838-.197-1.538-1.118l1.518-4.674a1 1 0 00-.363-1.118l-3.976-2.888c-.784-.57-.38-1.81.588-1.81h4.914a1 1 0 00.951-.69l1.519-4.674z"
          />
        </svg>
        <p className="mt-2 text-gray-500">{t('marketplace.reviewCard.noReviews')}</p>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {reviews.map((review) => (
        <ReviewCard
          key={review.id}
          review={review}
          isProviderView={isProviderView}
          onRespond={onRespond}
          onMarkHelpful={onMarkHelpful}
          onReport={onReport}
        />
      ))}
    </div>
  );
}
