/**
 * ListingGrid Component
 *
 * Grid/List view for listing search results (Epic 44, Story 44.2).
 */

'use client';

import type { ListingSummary } from '@ppt/reality-api-client';
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
    return (
      <div className="empty-state">
        <svg
          width="64"
          height="64"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="1"
          aria-hidden="true"
        >
          <circle cx="11" cy="11" r="8" />
          <path d="m21 21-4.35-4.35" />
        </svg>
        <h3 className="empty-title">No listings found</h3>
        <p className="empty-text">
          Try adjusting your filters or search criteria to find more properties.
        </p>
        <style jsx>{`
          .empty-state {
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            padding: 64px 24px;
            text-align: center;
            color: var(--ppt-fg-muted);
          }
          .empty-title {
            font-size: 1.25rem;
            font-weight: 600;
            color: var(--ppt-fg-secondary);
            margin: 16px 0 8px;
          }
          .empty-text {
            margin: 0;
            max-width: 300px;
          }
        `}</style>
      </div>
    );
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
