/**
 * MarketplacePage — `/community/marketplace`.
 *
 * Active listings of the selected building
 * (`GET /community/buildings/{id}/marketplace?category=&limit=&offset=`).
 * Category filtering and paging are server-side; the title search runs over
 * the loaded page. Each card opens the listing detail.
 */

import {
  type MarketplaceCategory,
  type MarketplaceItemCondition,
  useMarketplaceItems,
} from '@ppt/api-client';
import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { CommunityLayout } from '../components/CommunityLayout';
import { ItemCard } from '../components/ItemCard';
import { EmptyState, ErrorState, LoadingState } from '../components/StateViews';
import { useCommunityBuilding, withBuilding } from '../hooks/useCommunityBuilding';

export const MARKETPLACE_CATEGORIES: MarketplaceCategory[] = [
  'furniture',
  'electronics',
  'clothing',
  'sports',
  'books',
  'toys',
  'home_garden',
  'services',
  'free',
  'wanted',
  'other',
];

export const ITEM_CONDITIONS: MarketplaceItemCondition[] = [
  'new',
  'like_new',
  'good',
  'fair',
  'for_parts',
];

const PAGE_SIZE = 24;

export function MarketplacePage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const building = useCommunityBuilding();

  return (
    <CommunityLayout
      active="marketplace"
      title={t('community.marketplace.title')}
      subtitle={t('community.marketplace.subtitle')}
      building={building}
      actions={
        <button
          type="button"
          onClick={() => navigate(withBuilding('/community/marketplace/new', building.buildingId))}
          className="px-4 py-2 text-sm font-medium bg-blue-600 text-white rounded-md hover:bg-blue-700"
        >
          {t('community.marketplace.create')}
        </button>
      }
    >
      {(buildingId) => <MarketplaceContent key={buildingId} buildingId={buildingId} />}
    </CommunityLayout>
  );
}

function MarketplaceContent({ buildingId }: { buildingId: string }) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [category, setCategory] = useState<MarketplaceCategory | ''>('');
  const [search, setSearch] = useState('');
  const [page, setPage] = useState(0);

  const params = useMemo(
    () => ({ category: category || undefined, limit: PAGE_SIZE, offset: page * PAGE_SIZE }),
    [category, page]
  );
  const itemsQuery = useMarketplaceItems(buildingId, params);
  const loaded = itemsQuery.data ?? [];
  const needle = search.trim().toLowerCase();
  const items = needle
    ? loaded.filter(
        (i) =>
          i.title.toLowerCase().includes(needle) ||
          (i.description ?? '').toLowerCase().includes(needle)
      )
    : loaded;
  const hasNext = loaded.length === PAGE_SIZE;

  return (
    <div className="space-y-6">
      <div className="bg-white rounded-lg shadow p-4 flex flex-wrap items-center gap-3">
        <input
          type="search"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder={t('community.marketplace.searchPlaceholder')}
          aria-label={t('community.common.search')}
          className="flex-1 min-w-[12rem] px-3 py-2 text-sm border border-gray-300 rounded-md"
        />
        <select
          value={category}
          onChange={(e) => {
            setCategory(e.target.value as MarketplaceCategory | '');
            setPage(0);
          }}
          aria-label={t('community.hub.category')}
          className="px-3 py-2 text-sm border border-gray-300 rounded-md"
        >
          <option value="">{t('community.marketplace.categories.allCategories')}</option>
          {MARKETPLACE_CATEGORIES.map((c) => (
            <option key={c} value={c}>
              {t(`community.marketplaceCategories.${c}`)}
            </option>
          ))}
        </select>
      </div>

      {itemsQuery.isLoading ? (
        <LoadingState />
      ) : itemsQuery.error ? (
        <ErrorState message={t('community.hub.loadError')} onRetry={() => itemsQuery.refetch()} />
      ) : items.length === 0 ? (
        <EmptyState
          title={t('community.marketplace.emptyTitle')}
          body={t('community.marketplace.emptyBody')}
        />
      ) : (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
          {items.map((item) => (
            <ItemCard
              key={item.id}
              item={item}
              onOpen={(id) => navigate(withBuilding(`/community/marketplace/${id}`, buildingId))}
            />
          ))}
        </div>
      )}

      {(page > 0 || hasNext) && !itemsQuery.isLoading && (
        <div className="flex items-center justify-center gap-3">
          <button
            type="button"
            onClick={() => setPage((p) => Math.max(0, p - 1))}
            disabled={page === 0}
            className="px-3 py-1 text-sm border border-gray-300 rounded-md disabled:opacity-50"
          >
            {t('community.common.previous')}
          </button>
          <span className="text-sm text-gray-500">{page + 1}</span>
          <button
            type="button"
            onClick={() => setPage((p) => p + 1)}
            disabled={!hasNext}
            className="px-3 py-1 text-sm border border-gray-300 rounded-md disabled:opacity-50"
          >
            {t('community.common.next')}
          </button>
        </div>
      )}
    </div>
  );
}
