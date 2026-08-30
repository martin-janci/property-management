/**
 * ListingGrid Component
 *
 * Grid/List view for listing search results (Epic 44, Story 44.2).
 */

'use client';

import type { ListingSummary } from '@ppt/reality-api-client';
import { useTranslations } from 'next-intl';
import { EmptyState } from '@/components/states/EmptyState';
import { ListingCard } from './ListingCard';

interface ListingGridProps {
  listings: ListingSummary[];
  viewMode: 'grid' | 'list';
  onToggleFavorite?: (listingId: string, isFavorite: boolean) => void;
  isLoading?: boolean;
}

export function ListingGrid({
  listings,
  viewMode,
  onToggleFavorite,
  isLoading = false,
}: ListingGridProps) {
  const t = useTranslations('listings');

  if (isLoading) {
    return (
      <div className={`listing-grid ${viewMode}`}>
        {[1, 2, 3, 4, 5, 6].map((i) => (
          <div key={`grid-skeleton-${i}`} className="skeleton-card" />
        ))}
        <style jsx>{`
          .listing-grid {
            display: grid;
            gap: 24px;
          }
          .listing-grid.grid {
            grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
          }
          .listing-grid.list {
            grid-template-columns: 1fr;
          }
          .skeleton-card {
            height: 320px;
            background: var(--ppt-border-default);
            border-radius: 12px;
            animation: pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
          }
          @keyframes pulse {
            0%, 100% {
              opacity: 1;
            }
            50% {
              opacity: 0.5;
            }
          }
        `}</style>
      </div>
    );
  }

  if (listings.length === 0) {
    return <EmptyState icon="🔍" title={t('emptyTitle')} description={t('emptyDescription')} />;
  }

  return (
    <div className={`listing-grid ${viewMode}`}>
      {listings.map((listing) => (
        <ListingCard key={listing.id} listing={listing} onToggleFavorite={onToggleFavorite} />
      ))}
      <style jsx>{`
        .listing-grid {
          display: grid;
          gap: 24px;
        }
        .listing-grid.grid {
          grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
        }
        .listing-grid.list {
          grid-template-columns: 1fr;
        }
      `}</style>
    </div>
  );
}
