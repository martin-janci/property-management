'use client';

import { useTranslations } from 'next-intl';
import type { ListingSectionProps } from './registry';

function formatPrice(price: number, currency: string) {
  const value = Number.isFinite(price) ? price : 0;
  if (typeof currency !== 'string' || currency.length === 0) {
    return new Intl.NumberFormat('en-US', { maximumFractionDigits: 0 }).format(value);
  }
  return new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency: currency,
    maximumFractionDigits: 0,
  }).format(value);
}

export function ListingHeader({ listing }: ListingSectionProps) {
  const t = useTranslations('listing');
  return (
    <div className="listing-header">
      <div className="badges">
        <span className={`badge ${listing.transactionType}`}>
          {listing.transactionType === 'sale' ? t('forSale') : t('forRent')}
        </span>
        <span className="badge type">{listing.propertyType}</span>
      </div>
      <h1 className="title">{listing.title}</h1>
      <p className="address">
        <svg
          width="18"
          height="18"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          aria-hidden="true"
        >
          <path d="M21 10c0 7-9 13-9 13s-9-6-9-13a9 9 0 0 1 18 0z" />
          <circle cx="12" cy="10" r="3" />
        </svg>
        {listing.address?.street && `${listing.address.street}, `}
        {listing.address?.city}
        {listing.address?.district && `, ${listing.address.district}`}
      </p>
      <div className="price-row">
        <span className="price">{formatPrice(listing.price, listing.currency)}</span>
        {listing.transactionType === 'rent' && (
          <span className="price-suffix">{t('perMonth')}</span>
        )}
        {listing.pricePerSqm && (
          <span className="price-per-sqm">
            ({formatPrice(listing.pricePerSqm, listing.currency)}/m²)
          </span>
        )}
      </div>
      <style jsx>{`
        .listing-header {
        }

        .badges {
          display: flex;
          gap: 8px;
          margin-bottom: 12px;
        }

        .badge {
          padding: 4px 12px;
          border-radius: 4px;
          font-size: 12px;
          font-weight: 600;
          text-transform: uppercase;
        }

        .badge.sale {
          background: var(--ppt-color-success);
          color: var(--ppt-fg-on-accent);
        }

        .badge.rent {
          background: var(--ppt-brand-500);
          color: var(--ppt-fg-on-accent);
        }

        .badge.type {
          background: var(--ppt-bg-subtle);
          color: var(--ppt-fg-secondary);
        }

        .title {
          font-size: 1.75rem;
          font-weight: bold;
          color: var(--ppt-fg-primary);
          margin: 0 0 12px;
        }

        .address {
          display: flex;
          align-items: center;
          gap: 8px;
          font-size: 1rem;
          color: var(--ppt-fg-muted);
          margin: 0 0 16px;
        }

        .price-row {
          display: flex;
          align-items: baseline;
          gap: 8px;
        }

        .price {
          font-size: 2rem;
          font-weight: bold;
          color: var(--ppt-fg-primary);
        }

        .price-suffix {
          font-size: 1rem;
          color: var(--ppt-fg-muted);
        }

        .price-per-sqm {
          font-size: 14px;
          color: var(--ppt-fg-subtle);
        }
      `}</style>
    </div>
  );
}
