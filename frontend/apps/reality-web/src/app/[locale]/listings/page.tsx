/**
 * Listings Page
 *
 * Property search results with filters (Epic 44, Story 44.2).
 */

'use client';

import type { ListingFilters as FilterType, ListingSortField } from '@ppt/reality-api-client';
import { useListings, useToggleFavorite } from '@ppt/reality-api-client';
import { useSearchParams } from 'next/navigation';
import { useTranslations } from 'next-intl';
import { Suspense, useCallback, useState } from 'react';
import { ListingFilters } from '@/components/listings/ListingFilters';
import { ListingGrid } from '@/components/listings/ListingGrid';
import { SearchBar } from '@/components/listings/SearchBar';
import { LoadingSkeleton } from '@/components/states';
import { Footer, Header } from '@/components/ui';
import { useRouter } from '@/i18n/routing';

function ListingsFallback() {
  return (
    <div className="page-container">
      <Header />
      <main className="main">
        <div className="tabs-container">
          <div className="tabs" />
        </div>
        <div className="content-container">
          <div className="search-section">
            <LoadingSkeleton height="48px" />
          </div>
          <div className="content-grid">
            <div className="filters-desktop">
              <LoadingSkeleton lines={8} />
            </div>
            <div className="results-section">
              <LoadingSkeleton cards={6} />
            </div>
          </div>
        </div>
      </main>
      <Footer />

      <style jsx>{`
        .page-container {
          min-height: 100vh;
          display: flex;
          flex-direction: column;
          background: var(--ppt-bg-app);
        }
        .main { flex: 1; }
        .tabs-container {
          background: var(--ppt-bg-surface);
          border-bottom: 1px solid var(--ppt-border-default);
          height: 53px;
        }
        .tabs {
          max-width: 1280px;
          margin: 0 auto;
          padding: 0 32px;
        }
        .content-container {
          max-width: 1280px;
          margin: 0 auto;
          padding: 24px 32px;
        }
        @media (max-width: 640px) {
          .tabs { padding: 0 16px; }
          .content-container { padding: 24px 16px; }
        }
        .search-section { margin-bottom: 24px; }
        .content-grid { display: grid; gap: 24px; }
        @media (min-width: 1024px) {
          .content-grid { grid-template-columns: 280px 1fr; }
        }
        .filters-desktop { display: none; }
        @media (min-width: 1024px) {
          .filters-desktop { display: block; }
        }
        .results-section { min-width: 0; }
      `}</style>
    </div>
  );
}

function ListingsContent() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const [showMobileFilters, setShowMobileFilters] = useState(false);
  const [viewMode, setViewMode] = useState<'grid' | 'list'>('grid');
  const t = useTranslations('listings');

  // Parse filters from URL
  const getFiltersFromUrl = useCallback((): FilterType => {
    const filters: FilterType = {};

    const query = searchParams.get('q');
    if (query) filters.query = query;

    const transactionType = searchParams.get('transactionType');
    if (transactionType === 'sale' || transactionType === 'rent') {
      filters.transactionType = transactionType;
    }

    const propertyType = searchParams.get('propertyType');
    if (propertyType) {
      filters.propertyType = propertyType.split(',') as FilterType['propertyType'];
    }

    const priceMin = searchParams.get('priceMin');
    if (priceMin) filters.priceMin = Number(priceMin);

    const priceMax = searchParams.get('priceMax');
    if (priceMax) filters.priceMax = Number(priceMax);

    const areaMin = searchParams.get('areaMin');
    if (areaMin) filters.areaMin = Number(areaMin);

    const areaMax = searchParams.get('areaMax');
    if (areaMax) filters.areaMax = Number(areaMax);

    const roomsMin = searchParams.get('roomsMin');
    if (roomsMin) filters.roomsMin = Number(roomsMin);

    const city = searchParams.get('city');
    if (city) filters.city = city;

    const sortBy = searchParams.get('sortBy') as ListingSortField | undefined;
    if (sortBy) filters.sortBy = sortBy;

    const sortOrder = searchParams.get('sortOrder');
    if (sortOrder === 'asc' || sortOrder === 'desc') {
      filters.sortOrder = sortOrder;
    }

    return filters;
  }, [searchParams]);

  const filters = getFiltersFromUrl();
  const currentPage = Number(searchParams.get('page') || 1);
  const { data, isLoading } = useListings(filters, currentPage);
  const toggleFavorite = useToggleFavorite();

  const updateUrl = useCallback(
    (newFilters: FilterType, page = 1) => {
      const params = new URLSearchParams();

      if (newFilters.query) params.set('q', newFilters.query);
      if (newFilters.transactionType) params.set('transactionType', newFilters.transactionType);
      if (newFilters.propertyType?.length)
        params.set('propertyType', newFilters.propertyType.join(','));
      if (newFilters.priceMin !== undefined) params.set('priceMin', String(newFilters.priceMin));
      if (newFilters.priceMax !== undefined) params.set('priceMax', String(newFilters.priceMax));
      if (newFilters.areaMin !== undefined) params.set('areaMin', String(newFilters.areaMin));
      if (newFilters.areaMax !== undefined) params.set('areaMax', String(newFilters.areaMax));
      if (newFilters.roomsMin !== undefined) params.set('roomsMin', String(newFilters.roomsMin));
      if (newFilters.city) params.set('city', newFilters.city);
      if (newFilters.sortBy) params.set('sortBy', newFilters.sortBy);
      if (newFilters.sortOrder) params.set('sortOrder', newFilters.sortOrder);
      if (page > 1) params.set('page', String(page));

      router.push(`/listings?${params.toString()}`);
    },
    [router]
  );

  const handleFiltersChange = (newFilters: FilterType) => {
    updateUrl(newFilters);
  };

  const handleSearch = (query: string) => {
    updateUrl({ ...filters, query: query || undefined });
  };

  const handleSortChange = (sortBy: ListingSortField, sortOrder: 'asc' | 'desc') => {
    updateUrl({ ...filters, sortBy, sortOrder });
  };

  const handleToggleFavorite = (listingId: string, isFavorite: boolean) => {
    toggleFavorite.mutate({ listingId, isFavorite });
  };

  const getTransactionTypeLabel = () => {
    if (filters.transactionType === 'sale') return t('propertiesForSale');
    if (filters.transactionType === 'rent') return t('propertiesForRent');
    return t('allProperties');
  };

  return (
    <div className="page-container">
      <Header />
      <main className="main">
        {/* Transaction Type Tabs */}
        <div className="tabs-container">
          <div className="tabs">
            <button
              type="button"
              className={`tab ${!filters.transactionType ? 'active' : ''}`}
              onClick={() => updateUrl({ ...filters, transactionType: undefined })}
            >
              {t('tabAll')}
            </button>
            <button
              type="button"
              className={`tab ${filters.transactionType === 'sale' ? 'active' : ''}`}
              onClick={() => updateUrl({ ...filters, transactionType: 'sale' })}
            >
              {t('tabForSale')}
            </button>
            <button
              type="button"
              className={`tab ${filters.transactionType === 'rent' ? 'active' : ''}`}
              onClick={() => updateUrl({ ...filters, transactionType: 'rent' })}
            >
              {t('tabForRent')}
            </button>
          </div>
        </div>

        <div className="content-container">
          {/* Search Bar — initialQuery shows city if no text query set */}
          <div className="search-section">
            <SearchBar initialQuery={filters.query ?? filters.city ?? ''} onSearch={handleSearch} />
          </div>

          <div className="content-grid">
            {/* Desktop Filters */}
            <div className="filters-desktop">
              <ListingFilters filters={filters} onFiltersChange={handleFiltersChange} />
            </div>

            {/* Results Section */}
            <div className="results-section">
              {/* Results Header */}
              <div className="results-header">
                <div className="results-info">
                  <h1 className="results-title">{getTransactionTypeLabel()}</h1>
                  {data && (
                    <p className="results-count">{t('resultsCount', { count: data.total })}</p>
                  )}
                </div>

                <div className="results-actions">
                  {/* Mobile Filter Button */}
                  <button
                    type="button"
                    className="filter-button-mobile"
                    onClick={() => setShowMobileFilters(true)}
                  >
                    <svg
                      width="20"
                      height="20"
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      strokeWidth="2"
                      aria-hidden="true"
                    >
                      <polygon points="22 3 2 3 10 12.46 10 19 14 21 14 12.46 22 3" />
                    </svg>
                    {t('filters')}
                  </button>

                  {/* Sort Dropdown */}
                  <select
                    className="sort-select"
                    value={`${filters.sortBy || 'createdAt'}-${filters.sortOrder || 'desc'}`}
                    onChange={(e) => {
                      const [sortBy, sortOrder] = e.target.value.split('-');
                      handleSortChange(sortBy as ListingSortField, sortOrder as 'asc' | 'desc');
                    }}
                  >
                    <option value="createdAt-desc">{t('sortNewest')}</option>
                    <option value="createdAt-asc">{t('sortOldest')}</option>
                    <option value="price-asc">{t('sortPriceLow')}</option>
                    <option value="price-desc">{t('sortPriceHigh')}</option>
                    <option value="area-desc">{t('sortSizeLarge')}</option>
                    <option value="area-asc">{t('sortSizeSmall')}</option>
                  </select>

                  {/* View Mode Toggle */}
                  <div className="view-toggle">
                    <button
                      type="button"
                      className={`view-button ${viewMode === 'grid' ? 'active' : ''}`}
                      onClick={() => setViewMode('grid')}
                      aria-label={t('gridView')}
                    >
                      <svg
                        width="18"
                        height="18"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        strokeWidth="2"
                        aria-hidden="true"
                      >
                        <rect x="3" y="3" width="7" height="7" />
                        <rect x="14" y="3" width="7" height="7" />
                        <rect x="14" y="14" width="7" height="7" />
                        <rect x="3" y="14" width="7" height="7" />
                      </svg>
                    </button>
                    <button
                      type="button"
                      className={`view-button ${viewMode === 'list' ? 'active' : ''}`}
                      onClick={() => setViewMode('list')}
                      aria-label={t('listView')}
                    >
                      <svg
                        width="18"
                        height="18"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        strokeWidth="2"
                        aria-hidden="true"
                      >
                        <line x1="8" y1="6" x2="21" y2="6" />
                        <line x1="8" y1="12" x2="21" y2="12" />
                        <line x1="8" y1="18" x2="21" y2="18" />
                        <line x1="3" y1="6" x2="3.01" y2="6" />
                        <line x1="3" y1="12" x2="3.01" y2="12" />
                        <line x1="3" y1="18" x2="3.01" y2="18" />
                      </svg>
                    </button>
                  </div>
                </div>
              </div>

              {/* Listings Grid */}
              <ListingGrid
                listings={data?.data ?? []}
                viewMode={viewMode}
                onToggleFavorite={handleToggleFavorite}
                isLoading={isLoading}
              />

              {/* Pagination */}
              {data && data.totalPages > 1 && (
                <div className="pagination">
                  <button
                    type="button"
                    className="pagination-btn"
                    disabled={data.page <= 1}
                    onClick={() => updateUrl(filters, data.page - 1)}
                    aria-label={t('prevPage')}
                  >
                    ‹
                  </button>
                  <p className="pagination-info">
                    {t('pageOf', { page: data.page, total: data.totalPages })}
                  </p>
                  <button
                    type="button"
                    className="pagination-btn"
                    disabled={data.page >= data.totalPages}
                    onClick={() => updateUrl(filters, data.page + 1)}
                    aria-label={t('nextPage')}
                  >
                    ›
                  </button>
                </div>
              )}
            </div>
          </div>
        </div>
      </main>

      {/* Mobile Filters Modal */}
      {showMobileFilters && (
        <ListingFilters
          filters={filters}
          onFiltersChange={handleFiltersChange}
          onClose={() => setShowMobileFilters(false)}
          isMobile
        />
      )}

      <Footer />

      <style jsx>{`
        .page-container {
          min-height: 100vh;
          display: flex;
          flex-direction: column;
          background: var(--ppt-bg-app);
        }

        .main {
          flex: 1;
        }

        .tabs-container {
          background: var(--ppt-bg-surface);
          border-bottom: 1px solid var(--ppt-border-default);
        }

        .tabs {
          max-width: 1280px;
          margin: 0 auto;
          padding: 0 32px;
          display: flex;
          gap: 8px;
        }

        .tab {
          padding: 16px 24px;
          border: none;
          background: transparent;
          font-size: 14px;
          font-weight: 500;
          color: var(--ppt-fg-muted);
          cursor: pointer;
          border-bottom: 2px solid transparent;
          transition: all 0.2s;
        }

        .tab:hover {
          color: var(--ppt-fg-secondary);
        }

        .tab.active {
          color: var(--ppt-color-primary);
          border-bottom-color: var(--ppt-color-primary);
        }

        .content-container {
          max-width: 1280px;
          margin: 0 auto;
          padding: 24px 32px;
        }

        @media (max-width: 640px) {
          .tabs { padding: 0 16px; }
          .content-container { padding: 24px 16px; }
        }

        .search-section {
          margin-bottom: 24px;
        }

        .content-grid {
          display: grid;
          gap: 24px;
        }

        @media (min-width: 1024px) {
          .content-grid {
            grid-template-columns: 280px 1fr;
          }
        }

        .filters-desktop {
          display: none;
        }

        @media (min-width: 1024px) {
          .filters-desktop {
            display: block;
          }
        }

        .results-section {
          min-width: 0;
        }

        .results-header {
          display: flex;
          flex-wrap: wrap;
          justify-content: space-between;
          align-items: center;
          gap: 16px;
          margin-bottom: 24px;
        }

        .results-title {
          font-size: 1.5rem;
          font-weight: bold;
          color: var(--ppt-fg-primary);
          margin: 0;
        }

        .results-count {
          font-size: 14px;
          color: var(--ppt-fg-muted);
          margin: 4px 0 0;
        }

        .results-actions {
          display: flex;
          align-items: center;
          gap: 12px;
        }

        .filter-button-mobile {
          display: flex;
          align-items: center;
          gap: 8px;
          padding: 8px 16px;
          background: var(--ppt-bg-surface);
          border: 1px solid var(--ppt-border-default);
          border-radius: 8px;
          font-size: 14px;
          color: var(--ppt-fg-secondary);
          cursor: pointer;
        }

        @media (min-width: 1024px) {
          .filter-button-mobile {
            display: none;
          }
        }

        .sort-select {
          padding: 8px 32px 8px 12px;
          background: var(--ppt-bg-surface);
          border: 1px solid var(--ppt-border-default);
          border-radius: 8px;
          font-size: 14px;
          color: var(--ppt-fg-secondary);
          cursor: pointer;
          appearance: none;
          background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' fill='none' viewBox='0 0 24 24' stroke='%236b7280'%3E%3Cpath stroke-linecap='round' stroke-linejoin='round' stroke-width='2' d='M19 9l-7 7-7-7'%3E%3C/path%3E%3C/svg%3E");
          background-repeat: no-repeat;
          background-position: right 8px center;
          background-size: 16px;
        }

        .view-toggle {
          display: none;
          background: var(--ppt-bg-surface);
          border: 1px solid var(--ppt-border-default);
          border-radius: 8px;
          overflow: hidden;
        }

        @media (min-width: 640px) {
          .view-toggle {
            display: flex;
          }
        }

        .view-button {
          padding: 8px;
          border: none;
          background: transparent;
          color: var(--ppt-fg-muted);
          cursor: pointer;
        }

        .view-button:first-child {
          border-right: 1px solid var(--ppt-border-default);
        }

        .view-button.active {
          background: var(--ppt-bg-subtle);
          color: var(--ppt-color-primary);
        }

        .pagination {
          margin-top: 32px;
          display: flex;
          align-items: center;
          justify-content: center;
          gap: 16px;
        }

        .pagination-info {
          font-size: 14px;
          color: var(--ppt-fg-muted);
          margin: 0;
        }

        .pagination-btn {
          padding: 8px 14px;
          font-size: 18px;
          line-height: 1;
          background: var(--ppt-bg-surface);
          border: 1px solid var(--ppt-border-default);
          border-radius: 8px;
          color: var(--ppt-fg-secondary);
          cursor: pointer;
          transition: background var(--ppt-transition-fast);
        }

        .pagination-btn:hover:not(:disabled) {
          background: var(--ppt-bg-subtle);
        }

        .pagination-btn:disabled {
          opacity: 0.4;
          cursor: not-allowed;
        }
      `}</style>
    </div>
  );
}

export default function ListingsPage() {
  return (
    <Suspense fallback={<ListingsFallback />}>
      <ListingsContent />
    </Suspense>
  );
}
