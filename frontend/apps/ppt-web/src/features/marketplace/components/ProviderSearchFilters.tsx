/**
 * ProviderSearchFilters component - search and filter controls for marketplace.
 * Epic 68: Service Provider Marketplace (Story 68.2)
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { BadgeType, ServiceCategory } from './ProviderCard';

export interface SearchFilters {
  query?: string;
  category?: ServiceCategory;
  location?: string;
  region?: string;
  minRating?: number;
  minHourlyRate?: number;
  maxHourlyRate?: number;
  verifiedOnly?: boolean;
  emergencyOnly?: boolean;
  badges?: BadgeType[];
  sortBy?: 'rating' | 'reviews' | 'jobs_completed' | 'hourly_rate';
  sortOrder?: 'asc' | 'desc';
}

interface ProviderSearchFiltersProps {
  filters: SearchFilters;
  onFiltersChange: (filters: SearchFilters) => void;
  onSearch: () => void;
  onReset: () => void;
  isLoading?: boolean;
}

const categoryValues: ServiceCategory[] = [
  'plumbing',
  'electrical',
  'hvac',
  'cleaning',
  'landscaping',
  'security',
  'painting',
  'roofing',
  'carpentry',
  'locksmith',
  'pest_control',
  'general_maintenance',
  'elevator_maintenance',
  'fire_safety',
  'waste_management',
  'other',
];

const ratingOptions = [4.5, 4, 3.5, 3];

const sortOptions: { value: NonNullable<SearchFilters['sortBy']>; labelKey: string }[] = [
  { value: 'rating', labelKey: 'marketplace.searchFilters.sortRating' },
  { value: 'reviews', labelKey: 'marketplace.searchFilters.sortReviews' },
  { value: 'jobs_completed', labelKey: 'marketplace.searchFilters.sortJobs' },
  { value: 'hourly_rate', labelKey: 'marketplace.searchFilters.sortHourlyRate' },
];

export function ProviderSearchFilters({
  filters,
  onFiltersChange,
  onSearch,
  onReset,
  isLoading,
}: ProviderSearchFiltersProps) {
  const { t } = useTranslation();
  const [showAdvanced, setShowAdvanced] = useState(false);

  const updateFilter = <K extends keyof SearchFilters>(key: K, value: SearchFilters[K]) => {
    onFiltersChange({ ...filters, [key]: value });
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSearch();
  };

  return (
    <form onSubmit={handleSubmit} className="bg-white rounded-lg shadow p-4">
      {/* Primary Search */}
      <div className="flex flex-col md:flex-row gap-4">
        {/* Search Query */}
        <div className="flex-1">
          <label htmlFor="search-query" className="sr-only">
            {t('marketplace.searchFilters.searchProvidersLabel')}
          </label>
          <div className="relative">
            <svg
              className="absolute left-3 top-1/2 -translate-y-1/2 w-5 h-5 text-gray-400"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <title>{t('marketplace.searchFilters.searchIcon')}</title>
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
              />
            </svg>
            <input
              id="search-query"
              type="text"
              placeholder={t('marketplace.searchFilters.searchPlaceholder')}
              value={filters.query || ''}
              onChange={(e) => updateFilter('query', e.target.value || undefined)}
              className="w-full pl-10 pr-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
            />
          </div>
        </div>

        {/* Category */}
        <div className="md:w-48">
          <label htmlFor="category" className="sr-only">
            {t('marketplace.searchFilters.serviceCategoryLabel')}
          </label>
          <select
            id="category"
            value={filters.category || ''}
            onChange={(e) =>
              updateFilter('category', (e.target.value as ServiceCategory) || undefined)
            }
            className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
          >
            <option value="">{t('marketplace.searchFilters.allCategories')}</option>
            {categoryValues.map((cat) => (
              <option key={cat} value={cat}>
                {t(`marketplace.serviceCategory.${cat}`)}
              </option>
            ))}
          </select>
        </div>

        {/* Location */}
        <div className="md:w-48">
          <label htmlFor="location" className="sr-only">
            {t('marketplace.searchFilters.locationLabel')}
          </label>
          <input
            id="location"
            type="text"
            placeholder={t('marketplace.searchFilters.locationPlaceholder')}
            value={filters.location || ''}
            onChange={(e) => updateFilter('location', e.target.value || undefined)}
            className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
          />
        </div>

        {/* Search Button */}
        <button
          type="submit"
          disabled={isLoading}
          className="px-6 py-2 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 disabled:opacity-50"
        >
          {isLoading
            ? t('marketplace.searchFilters.searching')
            : t('marketplace.searchFilters.search')}
        </button>
      </div>

      {/* Toggle Advanced Filters */}
      <button
        type="button"
        onClick={() => setShowAdvanced(!showAdvanced)}
        className="mt-4 text-sm text-blue-600 hover:text-blue-800 flex items-center gap-1"
      >
        <svg
          className={`w-4 h-4 transition-transform ${showAdvanced ? 'rotate-180' : ''}`}
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <title>{t('marketplace.searchFilters.toggleFilters')}</title>
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
        </svg>
        {showAdvanced
          ? t('marketplace.searchFilters.hideAdvanced')
          : t('marketplace.searchFilters.showAdvanced')}
      </button>

      {/* Advanced Filters */}
      {showAdvanced && (
        <div className="mt-4 pt-4 border-t grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          {/* Minimum Rating */}
          <div>
            <label htmlFor="min-rating" className="block text-sm font-medium text-gray-700 mb-1">
              {t('marketplace.searchFilters.minimumRating')}
            </label>
            <select
              id="min-rating"
              value={filters.minRating || ''}
              onChange={(e) =>
                updateFilter('minRating', e.target.value ? Number(e.target.value) : undefined)
              }
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
            >
              <option value="">{t('marketplace.searchFilters.anyRating')}</option>
              {ratingOptions.map((opt) => (
                <option key={opt} value={opt}>
                  {t('marketplace.searchFilters.starsPlus', { rating: opt })}
                </option>
              ))}
            </select>
          </div>

          {/* Price Range */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">
              {t('marketplace.searchFilters.hourlyRateEur')}
            </label>
            <div className="flex items-center gap-2">
              <input
                type="number"
                placeholder={t('marketplace.searchFilters.minPlaceholder')}
                value={filters.minHourlyRate || ''}
                onChange={(e) =>
                  updateFilter('minHourlyRate', e.target.value ? Number(e.target.value) : undefined)
                }
                className="w-20 px-2 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
              />
              <span className="text-gray-500">-</span>
              <input
                type="number"
                placeholder={t('marketplace.searchFilters.maxPlaceholder')}
                value={filters.maxHourlyRate || ''}
                onChange={(e) =>
                  updateFilter('maxHourlyRate', e.target.value ? Number(e.target.value) : undefined)
                }
                className="w-20 px-2 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
              />
            </div>
          </div>

          {/* Sort By */}
          <div>
            <label htmlFor="sort-by" className="block text-sm font-medium text-gray-700 mb-1">
              {t('marketplace.searchFilters.sortByLabel')}
            </label>
            <select
              id="sort-by"
              value={filters.sortBy || ''}
              onChange={(e) =>
                updateFilter('sortBy', (e.target.value as SearchFilters['sortBy']) || undefined)
              }
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
            >
              <option value="">{t('marketplace.searchFilters.relevance')}</option>
              {sortOptions.map((opt) => (
                <option key={opt.value} value={opt.value}>
                  {t(opt.labelKey)}
                </option>
              ))}
            </select>
          </div>

          {/* Checkboxes */}
          <div className="space-y-2">
            <label className="block text-sm font-medium text-gray-700 mb-1">
              {t('marketplace.searchFilters.options')}
            </label>
            <label className="flex items-center gap-2 cursor-pointer">
              <input
                type="checkbox"
                checked={filters.verifiedOnly || false}
                onChange={(e) => updateFilter('verifiedOnly', e.target.checked || undefined)}
                className="w-4 h-4 text-blue-600 border-gray-300 rounded focus:ring-blue-500"
              />
              <span className="text-sm text-gray-700">
                {t('marketplace.searchFilters.verifiedOnly')}
              </span>
            </label>
            <label className="flex items-center gap-2 cursor-pointer">
              <input
                type="checkbox"
                checked={filters.emergencyOnly || false}
                onChange={(e) => updateFilter('emergencyOnly', e.target.checked || undefined)}
                className="w-4 h-4 text-blue-600 border-gray-300 rounded focus:ring-blue-500"
              />
              <span className="text-sm text-gray-700">
                {t('marketplace.searchFilters.emergencyAvailable')}
              </span>
            </label>
          </div>
        </div>
      )}

      {/* Active Filters Summary */}
      {(filters.category ||
        filters.location ||
        filters.minRating ||
        filters.verifiedOnly ||
        filters.emergencyOnly) && (
        <div className="mt-4 pt-4 border-t flex items-center gap-2 flex-wrap">
          <span className="text-sm text-gray-500">
            {t('marketplace.searchFilters.activeFilters')}
          </span>
          {filters.category && (
            <span className="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium bg-blue-100 text-blue-800">
              {t(`marketplace.serviceCategory.${filters.category}`)}
              <button
                type="button"
                onClick={() => updateFilter('category', undefined)}
                className="ml-1 hover:text-blue-600"
                aria-label={t('marketplace.common.remove')}
              >
                x
              </button>
            </span>
          )}
          {filters.location && (
            <span className="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium bg-blue-100 text-blue-800">
              {filters.location}
              <button
                type="button"
                onClick={() => updateFilter('location', undefined)}
                className="ml-1 hover:text-blue-600"
                aria-label={t('marketplace.common.remove')}
              >
                x
              </button>
            </span>
          )}
          {filters.minRating && (
            <span className="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium bg-yellow-100 text-yellow-800">
              {t('marketplace.searchFilters.starsPlus', { rating: filters.minRating })}
              <button
                type="button"
                onClick={() => updateFilter('minRating', undefined)}
                className="ml-1 hover:text-yellow-600"
                aria-label={t('marketplace.common.remove')}
              >
                x
              </button>
            </span>
          )}
          {filters.verifiedOnly && (
            <span className="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium bg-green-100 text-green-800">
              {t('marketplace.common.verified')}
              <button
                type="button"
                onClick={() => updateFilter('verifiedOnly', undefined)}
                className="ml-1 hover:text-green-600"
                aria-label={t('marketplace.common.remove')}
              >
                x
              </button>
            </span>
          )}
          {filters.emergencyOnly && (
            <span className="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium bg-red-100 text-red-800">
              {t('marketplace.searchFilters.emergency')}
              <button
                type="button"
                onClick={() => updateFilter('emergencyOnly', undefined)}
                className="ml-1 hover:text-red-600"
                aria-label={t('marketplace.common.remove')}
              >
                x
              </button>
            </span>
          )}
          <button
            type="button"
            onClick={onReset}
            className="text-sm text-gray-500 hover:text-gray-700 underline"
          >
            {t('marketplace.common.clearAll')}
          </button>
        </div>
      )}
    </form>
  );
}
