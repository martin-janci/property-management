/**
 * Integration Marketplace Page (Epic 150, Story 150.1)
 *
 * Browse and install integrations from the marketplace.
 */

import { type MarketplaceIntegrationSummary, useMarketplaceIntegrations } from '@ppt/api-client';
import { useMemo, useState } from 'react';
import { IntegrationCard, type IntegrationCardProps } from '../components/IntegrationCard';

/**
 * Map a backend `MarketplaceIntegrationSummary` (snake_case wire format) onto
 * the `IntegrationCardProps` shape the card component expects.
 */
function toCardProps(i: MarketplaceIntegrationSummary): IntegrationCardProps {
  return {
    id: i.id,
    slug: i.slug,
    name: i.name,
    description: i.description,
    category: i.category,
    iconUrl: i.icon_url ?? undefined,
    vendorName: i.vendor_name,
    status: i.status,
    ratingAverage: i.rating_average ?? undefined,
    ratingCount: i.rating_count,
    installCount: i.install_count,
    isFeatured: i.is_featured,
    isPremium: i.is_premium,
  };
}

const categories = [
  { value: '', label: 'All Categories' },
  { value: 'accounting', label: 'Accounting' },
  { value: 'crm', label: 'CRM' },
  { value: 'calendar', label: 'Calendar' },
  { value: 'communication', label: 'Communication' },
  { value: 'payment', label: 'Payment' },
  { value: 'property_portal', label: 'Property Portal' },
  { value: 'iot', label: 'IoT' },
  { value: 'analytics', label: 'Analytics' },
  { value: 'document_management', label: 'Document Management' },
  { value: 'other', label: 'Other' },
];

export function IntegrationMarketplacePage() {
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedCategory, setSelectedCategory] = useState('');
  const [showFeaturedOnly, setShowFeaturedOnly] = useState(false);
  const [showPremiumOnly, setShowPremiumOnly] = useState(false);
  const [sortBy, setSortBy] = useState<'popular' | 'rating' | 'name'>('popular');

  const { data, isLoading, isError, error, refetch } = useMarketplaceIntegrations();

  const integrations = useMemo<IntegrationCardProps[]>(() => (data ?? []).map(toCardProps), [data]);

  const filteredIntegrations = useMemo(() => {
    let result = [...integrations];

    // Search filter
    if (searchQuery) {
      const query = searchQuery.toLowerCase();
      result = result.filter(
        (i) =>
          i.name.toLowerCase().includes(query) ||
          i.description.toLowerCase().includes(query) ||
          i.vendorName.toLowerCase().includes(query)
      );
    }

    // Category filter
    if (selectedCategory) {
      result = result.filter((i) => i.category === selectedCategory);
    }

    // Featured filter
    if (showFeaturedOnly) {
      result = result.filter((i) => i.isFeatured);
    }

    // Premium filter
    if (showPremiumOnly) {
      result = result.filter((i) => i.isPremium);
    }

    // Sort
    result.sort((a, b) => {
      switch (sortBy) {
        case 'popular':
          return b.installCount - a.installCount;
        case 'rating':
          return (b.ratingAverage || 0) - (a.ratingAverage || 0);
        case 'name':
          return a.name.localeCompare(b.name);
        default:
          return 0;
      }
    });

    return result;
  }, [integrations, searchQuery, selectedCategory, showFeaturedOnly, showPremiumOnly, sortBy]);

  const handleInstall = (_id: string) => {
    // TODO: Implement installation flow (Epic API-marketplace install)
  };

  const handleViewDetails = (_id: string) => {
    // TODO: Navigate to integration details route
  };

  return (
    <div className="min-h-screen bg-gray-50">
      {/* Header */}
      <div className="bg-white shadow">
        <div className="mx-auto max-w-7xl px-4 py-6 sm:px-6 lg:px-8">
          <h1 className="text-3xl font-bold tracking-tight text-gray-900">
            Integration Marketplace
          </h1>
          <p className="mt-2 text-sm text-gray-600">
            Browse and install integrations to connect PPT with your favorite tools and services.
          </p>
        </div>
      </div>

      {/* Filters */}
      <div className="mx-auto max-w-7xl px-4 py-6 sm:px-6 lg:px-8">
        <div className="flex flex-col space-y-4 lg:flex-row lg:items-center lg:justify-between lg:space-y-0">
          {/* Search */}
          <div className="relative flex-1 lg:max-w-md">
            <input
              type="text"
              placeholder="Search integrations..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="block w-full rounded-lg border-gray-300 pl-10 shadow-sm focus:border-primary-500 focus:ring-primary-500 sm:text-sm"
            />
            <div className="pointer-events-none absolute inset-y-0 left-0 flex items-center pl-3">
              <svg
                className="h-5 w-5 text-gray-400"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
                />
              </svg>
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-4">
            {/* Category filter */}
            <select
              value={selectedCategory}
              onChange={(e) => setSelectedCategory(e.target.value)}
              className="rounded-lg border-gray-300 shadow-sm focus:border-primary-500 focus:ring-primary-500 sm:text-sm"
            >
              {categories.map((cat) => (
                <option key={cat.value} value={cat.value}>
                  {cat.label}
                </option>
              ))}
            </select>

            {/* Sort */}
            <select
              value={sortBy}
              onChange={(e) => setSortBy(e.target.value as 'popular' | 'rating' | 'name')}
              className="rounded-lg border-gray-300 shadow-sm focus:border-primary-500 focus:ring-primary-500 sm:text-sm"
            >
              <option value="popular">Most Popular</option>
              <option value="rating">Highest Rated</option>
              <option value="name">Name A-Z</option>
            </select>

            {/* Toggle filters */}
            <label className="flex items-center space-x-2">
              <input
                type="checkbox"
                checked={showFeaturedOnly}
                onChange={(e) => setShowFeaturedOnly(e.target.checked)}
                className="h-4 w-4 rounded border-gray-300 text-primary-600 focus:ring-primary-500"
              />
              <span className="text-sm text-gray-700">Featured</span>
            </label>

            <label className="flex items-center space-x-2">
              <input
                type="checkbox"
                checked={showPremiumOnly}
                onChange={(e) => setShowPremiumOnly(e.target.checked)}
                className="h-4 w-4 rounded border-gray-300 text-primary-600 focus:ring-primary-500"
              />
              <span className="text-sm text-gray-700">Premium</span>
            </label>
          </div>
        </div>

        {/* Results count */}
        {!isLoading && !isError && (
          <p className="mt-4 text-sm text-gray-500">
            Showing {filteredIntegrations.length} integration
            {filteredIntegrations.length !== 1 ? 's' : ''}
          </p>
        )}
      </div>

      {/* Integration grid */}
      <div className="mx-auto max-w-7xl px-4 pb-12 sm:px-6 lg:px-8">
        {isLoading ? (
          <div className="rounded-lg border-2 border-dashed border-gray-300 p-12 text-center text-sm text-gray-500">
            Loading integrations…
          </div>
        ) : isError ? (
          <div className="rounded-lg border-2 border-dashed border-red-300 bg-red-50 p-12 text-center">
            <h3 className="text-sm font-medium text-red-800">Failed to load integrations</h3>
            <p className="mt-1 text-sm text-red-600">
              {error instanceof Error ? error.message : 'An unexpected error occurred.'}
            </p>
            <button
              type="button"
              onClick={() => refetch()}
              className="mt-4 rounded-lg border border-red-300 bg-white px-4 py-2 text-sm font-medium text-red-700 hover:bg-red-50"
            >
              Retry
            </button>
          </div>
        ) : filteredIntegrations.length > 0 ? (
          <div className="grid grid-cols-1 gap-6 md:grid-cols-2 lg:grid-cols-3">
            {filteredIntegrations.map((integration) => (
              <IntegrationCard
                key={integration.id}
                {...integration}
                onInstall={() => handleInstall(integration.id)}
                onViewDetails={() => handleViewDetails(integration.id)}
              />
            ))}
          </div>
        ) : (
          <div className="rounded-lg border-2 border-dashed border-gray-300 p-12 text-center">
            <svg
              className="mx-auto h-12 w-12 text-gray-400"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M9.172 16.172a4 4 0 015.656 0M9 10h.01M15 10h.01M12 2a10 10 0 110 20 10 10 0 010-20z"
              />
            </svg>
            <h3 className="mt-2 text-sm font-medium text-gray-900">No integrations found</h3>
            <p className="mt-1 text-sm text-gray-500">
              Try adjusting your search or filter criteria.
            </p>
          </div>
        )}
      </div>
    </div>
  );
}

export default IntegrationMarketplacePage;
