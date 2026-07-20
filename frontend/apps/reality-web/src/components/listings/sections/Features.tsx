'use client';

import type { ListingFeatures } from '@ppt/reality-api-client';
import { useTranslations } from 'next-intl';
import type { ListingSectionProps } from './registry';

export function Features({ listing }: ListingSectionProps) {
  const t = useTranslations('listing');
  const tFeatures = useTranslations('features');

  const getFeatureLabel = (key: keyof ListingFeatures): string => {
    return tFeatures(key);
  };

  const activeFeatures = Object.entries(listing.features)
    .filter(([, value]) => value === true)
    .map(([key]) => key as keyof ListingFeatures);

  if (activeFeatures.length === 0) return null;

  return (
    <section className="section">
      <h2 className="section-title">{t('features')}</h2>
      <div className="features-grid">
        {activeFeatures.map((feature) => (
          <div key={feature} className="feature-item">
            <svg
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              aria-hidden="true"
            >
              <polyline points="20 6 9 17 4 12" />
            </svg>
            <span>{getFeatureLabel(feature)}</span>
          </div>
        ))}
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

        .features-grid {
          display: grid;
          grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
          gap: 12px;
        }

        .feature-item {
          display: flex;
          align-items: center;
          gap: 8px;
          color: var(--ppt-fg-secondary);
        }

        .feature-item svg {
          color: var(--ppt-color-success);
        }
      `}</style>
    </section>
  );
}
