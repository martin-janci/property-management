'use client';

import { useTranslations } from 'next-intl';
import type { ListingSectionProps } from './registry';

export function Resources({ listing }: ListingSectionProps) {
  const t = useTranslations('listing');

  if (!listing.virtualTourUrl && !listing.floorPlanUrl) return null;

  return (
    <section className="section">
      <h2 className="section-title">{t('additionalResources')}</h2>
      <div className="resources">
        {listing.virtualTourUrl && (
          <a
            href={listing.virtualTourUrl}
            target="_blank"
            rel="noopener noreferrer"
            className="resource-link"
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
              <circle cx="12" cy="12" r="10" />
              <polygon points="10 8 16 12 10 16 10 8" />
            </svg>
            {t('virtualTour')}
          </a>
        )}
        {listing.floorPlanUrl && (
          <a
            href={listing.floorPlanUrl}
            target="_blank"
            rel="noopener noreferrer"
            className="resource-link"
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
              <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
              <line x1="3" y1="9" x2="21" y2="9" />
              <line x1="9" y1="21" x2="9" y2="9" />
            </svg>
            {t('floorPlan')}
          </a>
        )}
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

        .resources {
          display: flex;
          gap: 16px;
          flex-wrap: wrap;
        }

        .resource-link {
          display: flex;
          align-items: center;
          gap: 8px;
          padding: 12px 20px;
          background: var(--ppt-bg-subtle);
          border-radius: 8px;
          color: var(--ppt-fg-secondary);
          text-decoration: none;
          font-weight: 500;
          transition: background 0.2s;
        }

        .resource-link:hover {
          background: var(--ppt-border-default);
        }
      `}</style>
    </section>
  );
}
