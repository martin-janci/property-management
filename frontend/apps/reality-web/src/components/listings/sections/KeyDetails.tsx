'use client';

import { useTranslations } from 'next-intl';
import type { ListingSectionProps } from './registry';

export function KeyDetails({ listing }: ListingSectionProps) {
  const t = useTranslations('listing');
  return (
    <div className="key-details">
      {listing.rooms !== undefined && (
        <div className="detail-item">
          <span className="detail-value">{listing.rooms}</span>
          <span className="detail-label">{t('rooms')}</span>
        </div>
      )}
      {listing.bedrooms !== undefined && (
        <div className="detail-item">
          <span className="detail-value">{listing.bedrooms}</span>
          <span className="detail-label">{t('bedrooms')}</span>
        </div>
      )}
      {listing.bathrooms !== undefined && (
        <div className="detail-item">
          <span className="detail-value">{listing.bathrooms}</span>
          <span className="detail-label">{t('bathrooms')}</span>
        </div>
      )}
      <div className="detail-item">
        <span className="detail-value">{listing.area}</span>
        <span className="detail-label">{t('sqm')}</span>
      </div>
      {listing.floor !== undefined && (
        <div className="detail-item">
          <span className="detail-value">
            {listing.floor}
            {listing.totalFloors && `/${listing.totalFloors}`}
          </span>
          <span className="detail-label">{t('floor')}</span>
        </div>
      )}
      {listing.yearBuilt !== undefined && (
        <div className="detail-item">
          <span className="detail-value">{listing.yearBuilt}</span>
          <span className="detail-label">{t('built')}</span>
        </div>
      )}
      <style jsx>{`
        .key-details {
          display: grid;
          grid-template-columns: repeat(auto-fit, minmax(100px, 1fr));
          gap: 16px;
          padding: 24px;
          background: var(--ppt-bg-surface);
          border-radius: 12px;
        }

        .detail-item {
          text-align: center;
        }

        .detail-value {
          display: block;
          font-size: 1.5rem;
          font-weight: 600;
          color: var(--ppt-fg-primary);
        }

        .detail-label {
          font-size: 14px;
          color: var(--ppt-fg-muted);
        }
      `}</style>
    </div>
  );
}
