/**
 * CategoryCards Component
 *
 * Property type category cards for homepage (Epic 44, Story 44.1).
 */

'use client';

import { useCategoryCounts } from '@ppt/reality-api-client';
import { useTranslations } from 'next-intl';
import Link from 'next/link';

const categoryIcons: Record<string, JSX.Element> = {
  apartment: (
    <svg
      width="32"
      height="32"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      aria-hidden="true"
    >
      <rect x="4" y="2" width="16" height="20" rx="2" ry="2" />
      <path d="M9 22v-4h6v4" />
      <path d="M8 6h.01M16 6h.01M8 10h.01M16 10h.01M8 14h.01M16 14h.01" />
    </svg>
  ),
  house: (
    <svg
      width="32"
      height="32"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      aria-hidden="true"
    >
      <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
      <polyline points="9 22 9 12 15 12 15 22" />
    </svg>
  ),
  land: (
    <svg
      width="32"
      height="32"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      aria-hidden="true"
    >
      <path d="M2 20h20" />
      <path d="M5 20v-4l4-4 4 4 4-8 4 8v4" />
    </svg>
  ),
  commercial: (
    <svg
      width="32"
      height="32"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      aria-hidden="true"
    >
      <path d="M2 22h20" />
      <rect x="6" y="8" width="12" height="14" rx="1" />
      <rect x="2" y="12" width="4" height="10" rx="1" />
      <rect x="18" y="12" width="4" height="10" rx="1" />
      <path d="M10 8V4a2 2 0 0 1 2-2h0a2 2 0 0 1 2 2v4" />
    </svg>
  ),
  office: (
    <svg
      width="32"
      height="32"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      aria-hidden="true"
    >
      <rect x="2" y="7" width="20" height="14" rx="2" ry="2" />
      <path d="M16 21V5a2 2 0 0 0-2-2h-4a2 2 0 0 0-2 2v16" />
    </svg>
  ),
  garage: (
    <svg
      width="32"
      height="32"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      aria-hidden="true"
    >
      <path d="M5 22V9l7-7 7 7v13H5z" />
      <rect x="8" y="13" width="8" height="9" />
    </svg>
  ),
};

const defaultIcon = (
  <svg
    width="32"
    height="32"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.5"
    aria-hidden="true"
  >
    <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
  </svg>
);

export function CategoryCards() {
  const t = useTranslations('home');
  const { data: categories, isLoading, error } = useCategoryCounts();

  if (isLoading) {
    return (
      <section className="categories-section">
        <div className="container">
          <div className="skeleton-header" />
          <div className="grid">
            {[1, 2, 3, 4].map((i) => (
              <div key={`cat-skeleton-${i}`} className="skeleton-card" />
            ))}
          </div>
        </div>
        <style jsx>{`
          .categories-section {
            padding: 64px 16px;
            background: var(--ppt-bg-surface);
          }
          .container {
            max-width: 1280px;
            margin: 0 auto;
          }
          .skeleton-header {
            height: 40px;
            width: 250px;
            background: var(--ppt-border-default);
            border-radius: 8px;
            margin: 0 auto 32px;
          }
          .grid {
            display: grid;
            grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
            gap: 16px;
          }
          .skeleton-card {
            height: 120px;
            background: var(--ppt-border-default);
            border-radius: 12px;
          }
        `}</style>
      </section>
    );
  }

  if (error || !categories?.length) {
    return null;
  }

  return (
    <section className="categories-section">
      <div className="container">
        <div className="section-header">
          <div>
            <h2 className="section-title">{t('browseByType')}</h2>
            <p className="section-subtitle">{t('browseByTypeSubtitle', { defaultValue: 'Pick a category to narrow the search.' })}</p>
          </div>
        </div>
        <div className="grid">
          {categories.map((category) => (
            <Link
              key={category.type}
              href={`/listings?propertyType=${category.type}`}
              className="category-card"
            >
              <div className="icon">{categoryIcons[category.type] || defaultIcon}</div>
              <h3 className="label">{category.label}</h3>
              <p className="count">
                {t('listingsCount', { count: category.count.toLocaleString() })}
              </p>
            </Link>
          ))}
        </div>
      </div>

      <style jsx>{`
        .categories-section {
          padding: 48px 32px 24px;
          background: var(--ppt-bg-app);
        }

        .container {
          max-width: var(--ppt-content-max, 1280px);
          margin: 0 auto;
        }

        .section-header {
          display: flex;
          justify-content: space-between;
          align-items: flex-end;
          margin-bottom: 24px;
        }

        .section-title {
          font-size: 22px;
          font-weight: var(--ppt-font-weight-bold, 700);
          color: var(--ppt-fg-primary);
          margin: 0;
          letter-spacing: -0.01em;
        }

        .section-subtitle {
          font-size: var(--ppt-font-size-sm, 14px);
          color: var(--ppt-fg-muted);
          margin: 4px 0 0;
        }

        .grid {
          display: grid;
          grid-template-columns: repeat(6, 1fr);
          gap: 10px;
        }

        @media (max-width: 1024px) {
          .grid { grid-template-columns: repeat(3, 1fr); }
        }

        @media (max-width: 640px) {
          .grid { grid-template-columns: repeat(2, 1fr); }
          .categories-section { padding: 32px 16px 16px; }
        }

        .category-card {
          display: flex;
          flex-direction: column;
          gap: 8px;
          padding: 14px 14px 12px;
          background: var(--ppt-bg-surface);
          border: 1px solid var(--ppt-border-default);
          border-radius: 10px;
          text-decoration: none;
          color: inherit;
          cursor: pointer;
          transition: border-color var(--ppt-transition-fast),
                      box-shadow var(--ppt-transition-fast);
        }

        .category-card:hover {
          border-color: var(--ppt-color-primary);
          box-shadow: var(--ppt-shadow-lg, 0 4px 12px rgba(0,0,0,0.15));
        }

        .category-card:focus-visible {
          outline: none;
          box-shadow: var(--ppt-focus-ring-shadow);
        }

        .icon {
          width: 32px;
          height: 32px;
          border-radius: var(--ppt-radius-md, 8px);
          background: var(--ppt-color-primary-soft-bg);
          color: var(--ppt-color-primary);
          display: flex;
          align-items: center;
          justify-content: center;
        }

        .label {
          font-size: 13.5px;
          font-weight: var(--ppt-font-weight-semibold, 600);
          color: var(--ppt-fg-primary);
          margin: 0;
        }

        .count {
          font-size: var(--ppt-font-size-xs, 12px);
          color: var(--ppt-fg-muted);
          margin: 0;
        }
      `}</style>
    </section>
  );
}
