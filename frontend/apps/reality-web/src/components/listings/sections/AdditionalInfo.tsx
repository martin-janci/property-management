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

export function AdditionalInfo({ listing }: ListingSectionProps) {
  const t = useTranslations('listing');
  return (
    <section className="section">
      <h2 className="section-title">{t('additionalInfo')}</h2>
      <div className="info-grid">
        {listing.energyRating && (
          <div className="info-item">
            <span className="info-label">{t('energyRating')}</span>
            <span className="info-value">{listing.energyRating}</span>
          </div>
        )}
        {listing.monthlyCharges !== undefined && (
          <div className="info-item">
            <span className="info-label">{t('monthlyCharges')}</span>
            <span className="info-value">
              {formatPrice(listing.monthlyCharges, listing.currency)}
            </span>
          </div>
        )}
        {listing.availableFrom && (
          <div className="info-item">
            <span className="info-label">{t('availableFrom')}</span>
            <span className="info-value">
              {new Date(listing.availableFrom).toLocaleDateString()}
            </span>
          </div>
        )}
        <div className="info-item">
          <span className="info-label">{t('listed')}</span>
          <span className="info-value">{new Date(listing.createdAt).toLocaleDateString()}</span>
        </div>
      </div>
      <style jsx>{`
        .section {
          padding: 24px;
          background: var(--ppt-bg-surface);
          border-radius: 12px;
        }

        .section-title {
          font-size: 1.125rem;
          font-weight: 600;
          color: var(--ppt-fg-primary);
          margin: 0 0 16px;
        }

        .info-grid {
          display: grid;
          grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
          gap: 16px;
        }

        .info-item {
          display: flex;
          justify-content: space-between;
          padding: 12px 0;
          border-bottom: 1px solid var(--ppt-bg-subtle);
        }

        .info-label {
          color: var(--ppt-fg-muted);
        }

        .info-value {
          font-weight: 500;
          color: var(--ppt-fg-primary);
        }
      `}</style>
    </section>
  );
}
