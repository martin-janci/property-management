/**
 * Competitive Status Badge Component
 * Epic 70: Competitive Feature Enhancements
 *
 * Shows the status of competitive features for a listing.
 */

import { useTranslation } from 'react-i18next';

export interface CompetitiveFeaturesStatus {
  listingId: string;
  hasVirtualTours: boolean;
  virtualTourCount: number;
  hasPricingAnalysis: boolean;
  pricingAnalysisValid: boolean;
  hasNeighborhoodInsights: boolean;
  neighborhoodInsightsValid: boolean;
  hasComparables: boolean;
  comparablesCount: number;
}

export interface CompetitiveStatusBadgeProps {
  status: CompetitiveFeaturesStatus;
  onViewDetails?: () => void;
  compact?: boolean;
  className?: string;
}

/**
 * Badge showing competitive features availability.
 */
export function CompetitiveStatusBadge({
  status,
  onViewDetails,
  compact = false,
  className = '',
}: CompetitiveStatusBadgeProps) {
  const { t } = useTranslation();
  const features = [
    {
      key: 'tours',
      label: t('competitiveStatus.features.tours'),
      active: status.hasVirtualTours,
      count: status.virtualTourCount,
      icon: (
        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z"
          />
        </svg>
      ),
    },
    {
      key: 'pricing',
      label: t('competitiveStatus.features.pricing'),
      active: status.hasPricingAnalysis && status.pricingAnalysisValid,
      icon: (
        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
          />
        </svg>
      ),
    },
    {
      key: 'neighborhood',
      label: t('competitiveStatus.features.neighborhood'),
      active: status.hasNeighborhoodInsights && status.neighborhoodInsightsValid,
      icon: (
        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M17.657 16.657L13.414 20.9a1.998 1.998 0 01-2.827 0l-4.244-4.243a8 8 0 1111.314 0z"
          />
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M15 11a3 3 0 11-6 0 3 3 0 016 0z"
          />
        </svg>
      ),
    },
    {
      key: 'comparables',
      label: t('competitiveStatus.features.comparables'),
      active: status.hasComparables,
      count: status.comparablesCount,
      icon: (
        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"
          />
        </svg>
      ),
    },
  ];

  const activeCount = features.filter((f) => f.active).length;
  const completionPercent = Math.round((activeCount / features.length) * 100);

  if (compact) {
    return (
      <button
        type="button"
        onClick={onViewDetails}
        className={`inline-flex items-center gap-2 px-3 py-1.5 rounded-full text-sm ${
          activeCount === features.length
            ? 'bg-green-100 text-green-700'
            : 'bg-gray-100 text-gray-700'
        } ${className}`}
      >
        <span className="font-medium">{completionPercent}%</span>
        <span className="text-xs">{t('competitiveStatus.competitive')}</span>
      </button>
    );
  }

  return (
    <div className={`bg-white rounded-lg shadow-sm border p-4 ${className}`}>
      {/* Header */}
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-sm font-semibold text-gray-900">{t('competitiveStatus.title')}</h3>
        <span
          className={`px-2 py-1 text-xs font-medium rounded-full ${
            completionPercent === 100
              ? 'bg-green-100 text-green-700'
              : completionPercent >= 50
                ? 'bg-yellow-100 text-yellow-700'
                : 'bg-gray-100 text-gray-600'
          }`}
        >
          {t('competitiveStatus.percentComplete', { percent: completionPercent })}
        </span>
      </div>

      {/* Progress bar */}
      <div className="h-2 bg-gray-200 rounded-full mb-4 overflow-hidden">
        <div
          className={`h-full rounded-full transition-all ${
            completionPercent === 100
              ? 'bg-green-500'
              : completionPercent >= 50
                ? 'bg-yellow-500'
                : 'bg-gray-400'
          }`}
          style={{ width: `${completionPercent}%` }}
        />
      </div>

      {/* Feature list */}
      <div className="space-y-2">
        {features.map((feature) => (
          <div
            key={feature.key}
            className="flex items-center justify-between p-2 rounded-lg hover:bg-gray-50"
          >
            <div className="flex items-center gap-2">
              <span className={`${feature.active ? 'text-green-600' : 'text-gray-400'}`}>
                {feature.icon}
              </span>
              <span className={`text-sm ${feature.active ? 'text-gray-900' : 'text-gray-500'}`}>
                {feature.label}
              </span>
            </div>
            <div className="flex items-center gap-2">
              {feature.count !== undefined && feature.count > 0 && (
                <span className="text-xs text-gray-500">{feature.count}</span>
              )}
              {feature.active ? (
                <svg className="w-5 h-5 text-green-500" fill="currentColor" viewBox="0 0 20 20">
                  <path
                    fillRule="evenodd"
                    d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
                    clipRule="evenodd"
                  />
                </svg>
              ) : (
                <svg className="w-5 h-5 text-gray-300" fill="currentColor" viewBox="0 0 20 20">
                  <path
                    fillRule="evenodd"
                    d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z"
                    clipRule="evenodd"
                  />
                </svg>
              )}
            </div>
          </div>
        ))}
      </div>

      {/* Action button */}
      {onViewDetails && (
        <button
          type="button"
          onClick={onViewDetails}
          className="w-full mt-4 px-4 py-2 text-sm font-medium text-blue-600 bg-blue-50 hover:bg-blue-100 rounded-lg transition-colors"
        >
          {t('competitiveStatus.viewAnalysis')}
        </button>
      )}
    </div>
  );
}
