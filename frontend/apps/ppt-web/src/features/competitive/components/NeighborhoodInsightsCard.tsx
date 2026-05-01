/**
 * Neighborhood Insights Card Component
 * Story 70.3: Neighborhood Insights
 *
 * Displays walk/transit/bike scores, demographics, and safety ratings.
 */

export interface NeighborhoodInsights {
  id: string;
  listingId?: string;
  latitude: number;
  longitude: number;
  walkScore?: number;
  transitScore?: number;
  bikeScore?: number;
  population?: number;
  medianAge?: number;
  medianIncome?: number;
  crimeIndex?: number;
  safetyRating?: string;
  dataSources: Record<string, string>;
  fetchedAt: string;
  validUntil: string;
}

export interface NeighborhoodInsightsCardProps {
  insights: NeighborhoodInsights;
  onRefresh?: () => void;
  onViewAmenities?: () => void;
  className?: string;
}

/**
 * Displays neighborhood scores and statistics.
 */
export function NeighborhoodInsightsCard({
  insights,
  onRefresh,
  onViewAmenities,
  className = '',
}: NeighborhoodInsightsCardProps) {
  const getScoreColor = (score: number) => {
    if (score >= 70) return 'text-green-600';
    if (score >= 50) return 'text-yellow-600';
    return 'text-red-600';
  };

  const getScoreLabel = (score: number) => {
    if (score >= 90) return "Walker's Paradise";
    if (score >= 70) return 'Very Walkable';
    if (score >= 50) return 'Somewhat Walkable';
    if (score >= 25) return 'Car-Dependent';
    return 'Almost All Errands Require a Car';
  };

  const formatNumber = (num: number) => {
    return new Intl.NumberFormat('sk-SK').format(num);
  };

  const formatCurrency = (num: number) => {
    return new Intl.NumberFormat('sk-SK', {
      style: 'currency',
      currency: 'EUR',
      maximumFractionDigits: 0,
    }).format(num);
  };

  const getSafetyColor = (rating?: string) => {
    switch (rating?.toLowerCase()) {
      case 'excellent':
        return 'text-green-600 bg-green-100';
      case 'good':
        return 'text-blue-600 bg-blue-100';
      case 'moderate':
        return 'text-yellow-600 bg-yellow-100';
      default:
        return 'text-gray-600 bg-gray-100';
    }
  };

  const ScoreCircle = ({
    score,
    label,
    icon,
  }: {
    score?: number;
    label: string;
    icon: React.ReactNode;
  }) => (
    <div className="flex flex-col items-center">
      <div className="relative w-20 h-20">
        <svg className="w-20 h-20 transform -rotate-90" viewBox="0 0 36 36">
          <circle cx="18" cy="18" r="16" fill="none" stroke="var(--ppt-border-default)" strokeWidth="2" />
          {score !== undefined && (
            <circle
              cx="18"
              cy="18"
              r="16"
              fill="none"
              stroke={score >= 70 ? '#16a34a' : score >= 50 ? '#ca8a04' : 'var(--ppt-color-danger-hover)'}
              strokeWidth="2"
              strokeDasharray={`${(score / 100) * 100.53} 100.53`}
              strokeLinecap="round"
            />
          )}
        </svg>
        <div className="absolute inset-0 flex flex-col items-center justify-center">
          {icon}
          <span
            className={`text-lg font-bold ${score !== undefined ? getScoreColor(score) : 'text-gray-400'}`}
          >
            {score ?? '-'}
          </span>
        </div>
      </div>
      <span className="text-xs text-gray-600 mt-1">{label}</span>
    </div>
  );

  return (
    <div className={`bg-white rounded-lg shadow-sm border ${className}`}>
      {/* Header */}
      <div className="flex items-center justify-between p-4 border-b">
        <h3 className="text-lg font-semibold text-gray-900">Neighborhood Insights</h3>
        {onRefresh && (
          <button
            type="button"
            onClick={onRefresh}
            className="p-2 text-gray-400 hover:text-gray-600 hover:bg-gray-100 rounded-full transition-colors"
            title="Refresh insights"
          >
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
              />
            </svg>
          </button>
        )}
      </div>

      {/* Scores */}
      <div className="p-4 border-b">
        <div className="flex justify-around">
          <ScoreCircle
            score={insights.walkScore}
            label="Walk Score"
            icon={
              <svg
                className="w-4 h-4 text-gray-400 mb-0.5"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M13 7h8m0 0v8m0-8l-8 8-4-4-6 6"
                />
              </svg>
            }
          />
          <ScoreCircle
            score={insights.transitScore}
            label="Transit Score"
            icon={
              <svg
                className="w-4 h-4 text-gray-400 mb-0.5"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M8 7h.01M12 7h.01M16 7h.01M8 11h.01M12 11h.01M16 11h.01M8 15h.01M12 15h.01M16 15h.01"
                />
              </svg>
            }
          />
          <ScoreCircle
            score={insights.bikeScore}
            label="Bike Score"
            icon={
              <svg
                className="w-4 h-4 text-gray-400 mb-0.5"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M13 10V3L4 14h7v7l9-11h-7z"
                />
              </svg>
            }
          />
        </div>
        {insights.walkScore !== undefined && (
          <p className="text-center text-sm text-gray-500 mt-3">
            {getScoreLabel(insights.walkScore)}
          </p>
        )}
      </div>

      {/* Demographics */}
      <div className="p-4 border-b">
        <h4 className="text-sm font-medium text-gray-700 mb-3">Demographics</h4>
        <div className="grid grid-cols-3 gap-4 text-center">
          <div className="p-2 bg-gray-50 rounded-lg">
            <div className="text-lg font-semibold text-gray-900">
              {insights.population ? formatNumber(insights.population) : '-'}
            </div>
            <div className="text-xs text-gray-500">Population</div>
          </div>
          <div className="p-2 bg-gray-50 rounded-lg">
            <div className="text-lg font-semibold text-gray-900">{insights.medianAge ?? '-'}</div>
            <div className="text-xs text-gray-500">Median Age</div>
          </div>
          <div className="p-2 bg-gray-50 rounded-lg">
            <div className="text-lg font-semibold text-gray-900">
              {insights.medianIncome ? formatCurrency(insights.medianIncome) : '-'}
            </div>
            <div className="text-xs text-gray-500">Median Income</div>
          </div>
        </div>
      </div>

      {/* Safety */}
      {(insights.crimeIndex !== undefined || insights.safetyRating) && (
        <div className="p-4 border-b">
          <h4 className="text-sm font-medium text-gray-700 mb-3">Safety</h4>
          <div className="flex items-center gap-4">
            {insights.safetyRating && (
              <span
                className={`px-3 py-1 rounded-full text-sm font-medium capitalize ${getSafetyColor(insights.safetyRating)}`}
              >
                {insights.safetyRating}
              </span>
            )}
            {insights.crimeIndex !== undefined && (
              <span className="text-sm text-gray-600">Crime Index: {insights.crimeIndex}/100</span>
            )}
          </div>
        </div>
      )}

      {/* View Amenities */}
      {onViewAmenities && (
        <div className="p-4">
          <button
            type="button"
            onClick={onViewAmenities}
            className="w-full flex items-center justify-center gap-2 px-4 py-2 text-blue-600 bg-blue-50 hover:bg-blue-100 rounded-lg transition-colors text-sm font-medium"
          >
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
            View Nearby Amenities
          </button>
        </div>
      )}

      {/* Data sources */}
      <div className="px-4 py-3 bg-gray-50 rounded-b-lg">
        <p className="text-xs text-gray-400">
          Data sources: {Object.values(insights.dataSources).join(', ') || 'Various'}
        </p>
      </div>
    </div>
  );
}
