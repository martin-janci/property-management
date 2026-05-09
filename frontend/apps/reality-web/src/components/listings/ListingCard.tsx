/**
 * ListingCard Component
 *
 * Property listing card for search results and featured sections (Epic 44).
 */

'use client';

import type { ListingSummary } from '@ppt/reality-api-client';
import Link from 'next/link';
import { useTranslations } from 'next-intl';

interface ListingCardProps {
  listing: ListingSummary;
  onToggleFavorite?: (listingId: string, isFavorite: boolean) => void;
  showFavoriteButton?: boolean;
}

export function ListingCard({
  listing,
  onToggleFavorite,
  showFavoriteButton = true,
}: ListingCardProps) {
  const t = useTranslations('listing');

  const formatPrice = (price: number, currency: string) => {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: currency,
      maximumFractionDigits: 0,
    }).format(price);
  };

  const handleFavoriteClick = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    onToggleFavorite?.(listing.id, listing.isFavorite ?? false);
  };

  return (
    <Link href={`/listings/${listing.slug}`} className="card">
      {/* Image */}
      <div className="image-container">
        {listing.primaryPhoto ? (
          <img src={listing.primaryPhoto.thumbnailUrl} alt={listing.title} className="image" />
        ) : (
          <div className="image-placeholder">
            <svg
              width="48"
              height="48"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="1"
              aria-hidden="true"
            >
              <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
              <circle cx="8.5" cy="8.5" r="1.5" />
              <polyline points="21 15 16 10 5 21" />
            </svg>
          </div>
        )}

        {/* Badges */}
        <div className="badges">
          {listing.isFeatured && <span className="badge featured">{t('featured')}</span>}
          <span className={`badge ${listing.transactionType}`}>
            {listing.transactionType === 'sale' ? t('forSale') : t('forRent')}
          </span>
        </div>

        {/* Favorite Button */}
        {showFavoriteButton && (
          <button
            type="button"
            className={`favorite-button ${listing.isFavorite ? 'active' : ''}`}
            onClick={handleFavoriteClick}
            aria-label={listing.isFavorite ? 'Remove from favorites' : 'Add to favorites'}
          >
            <svg
              width="20"
              height="20"
              viewBox="0 0 24 24"
              fill={listing.isFavorite ? 'currentColor' : 'none'}
              stroke="currentColor"
              strokeWidth="2"
              aria-hidden="true"
            >
              <path d="M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z" />
            </svg>
          </button>
        )}
      </div>

      {/* Content */}
      <div className="content">
        <div className="price-row">
          <span className="price">{formatPrice(listing.price, listing.currency)}</span>
          {listing.transactionType === 'rent' && (
            <span className="price-suffix">{t('perMonth')}</span>
          )}
        </div>

        <h3 className="title">{listing.title}</h3>

        <p className="location">
          <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            aria-hidden="true"
          >
            <path d="M21 10c0 7-9 13-9 13s-9-6-9-13a9 9 0 0 1 18 0z" />
            <circle cx="12" cy="10" r="3" />
          </svg>
          {listing.address.city}
          {listing.address.district && `, ${listing.address.district}`}
        </p>

        {/* Features */}
        <div className="features">
          {listing.rooms !== undefined && (
            <div className="feature">
              <svg
                width="16"
                height="16"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                aria-hidden="true"
              >
                <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
              </svg>
              <span>{t('roomsCount', { count: listing.rooms })}</span>
            </div>
          )}
          <div className="feature">
            <svg
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              aria-hidden="true"
            >
              <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
            </svg>
            <span>
              {listing.area} {t('sqm')}
            </span>
          </div>
          {listing.floor !== undefined && (
            <div className="feature">
              <svg
                width="16"
                height="16"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                aria-hidden="true"
              >
                <path d="M18 20V4H6v16" />
                <path d="M2 20h20" />
              </svg>
              <span>{t('floorNumber', { number: listing.floor })}</span>
            </div>
          )}
        </div>
      </div>

      <style jsx>{`
        .card {
          display: block;
          background: var(--ppt-bg-surface);
          border-radius: var(--ppt-radius-lg);
          border: 1px solid var(--ppt-border-default);
          overflow: hidden;
          box-shadow: var(--ppt-shadow-md);
          text-decoration: none;
          color: inherit;
          transition: box-shadow var(--ppt-transition-normal),
                      border-color var(--ppt-transition-normal),
                      transform var(--ppt-transition-normal);
        }

        .card:hover {
          box-shadow: var(--ppt-shadow-lg);
          border-color: var(--ppt-border-strong);
          transform: translateY(-2px);
        }

        .card:focus-visible {
          outline: none;
          box-shadow: var(--ppt-focus-ring-shadow);
        }

        .image-container {
          position: relative;
          aspect-ratio: 4 / 3;
          background: var(--ppt-neutral-100);
        }

        .image {
          width: 100%;
          height: 100%;
          object-fit: cover;
        }

        .image-placeholder {
          width: 100%;
          height: 100%;
          display: flex;
          align-items: center;
          justify-content: center;
          color: var(--ppt-fg-subtle);
        }

        .badges {
          position: absolute;
          top: 10px;
          left: 10px;
          display: flex;
          gap: 4px;
          z-index: 2;
        }

        .badge {
          padding: 3px 8px;
          border-radius: var(--ppt-radius-sm);
          font-size: 10px;
          font-weight: 700;
          text-transform: uppercase;
          letter-spacing: 0.04em;
          color: #fff;
        }

        .badge.featured {
          background: var(--ppt-badge-featured-bg);
          color: var(--ppt-badge-featured-ink);
        }

        .badge.sale {
          background: var(--ppt-badge-sale-bg);
          color: var(--ppt-badge-sale-ink);
        }

        .badge.rent {
          background: var(--ppt-badge-rent-bg);
          color: var(--ppt-badge-rent-ink);
        }

        .favorite-button {
          position: absolute;
          top: 10px;
          right: 10px;
          width: 30px;
          height: 30px;
          border-radius: var(--ppt-radius-full);
          background: rgba(255, 255, 255, 0.92);
          border: none;
          cursor: pointer;
          display: flex;
          align-items: center;
          justify-content: center;
          color: var(--ppt-fg-secondary);
          transition: color var(--ppt-transition-fast),
                      background var(--ppt-transition-fast);
          z-index: 2;
        }

        .favorite-button:hover {
          background: #fff;
          color: var(--ppt-color-danger);
        }

        .favorite-button.active {
          color: var(--ppt-color-danger);
        }

        .favorite-button:focus-visible {
          outline: none;
          box-shadow: var(--ppt-focus-ring-shadow);
        }

        .content {
          padding: 12px 14px 14px;
        }

        .price-row {
          display: flex;
          align-items: baseline;
          gap: 4px;
          margin-bottom: 2px;
        }

        .price {
          font-size: var(--ppt-font-size-xl);
          font-weight: var(--ppt-font-weight-bold);
          color: var(--ppt-fg-primary);
          letter-spacing: var(--ppt-tracking-tight);
        }

        .price-suffix {
          font-size: var(--ppt-font-size-xs);
          color: var(--ppt-fg-muted);
          font-weight: var(--ppt-font-weight-medium);
        }

        .title {
          font-size: var(--ppt-font-size-sm);
          font-weight: var(--ppt-font-weight-semibold);
          color: var(--ppt-fg-primary);
          margin: 6px 0 0;
          line-height: var(--ppt-line-height-snug);
          display: -webkit-box;
          -webkit-line-clamp: 2;
          -webkit-box-orient: vertical;
          overflow: hidden;
        }

        .location {
          display: flex;
          align-items: center;
          gap: 4px;
          font-size: var(--ppt-font-size-xs);
          color: var(--ppt-fg-muted);
          margin: 4px 0 0;
        }

        .features {
          display: flex;
          gap: 10px;
          flex-wrap: wrap;
          margin-top: 8px;
          padding-top: 8px;
          border-top: 1px solid var(--ppt-border-subtle);
        }

        .feature {
          display: flex;
          align-items: center;
          gap: 3px;
          font-size: 11px;
          color: var(--ppt-fg-muted);
        }
      `}</style>
    </Link>
  );
}
