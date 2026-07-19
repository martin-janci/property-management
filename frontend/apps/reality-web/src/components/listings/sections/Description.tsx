'use client';

import { useTranslations } from 'next-intl';
import type { ListingSectionProps } from './registry';

export function Description({ listing }: ListingSectionProps) {
  const t = useTranslations('listing');
  return (
    <section className="section">
      <h2 className="section-title">{t('description')}</h2>
      <p className="description">{listing.description}</p>
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

        .description {
          color: var(--ppt-fg-secondary);
          line-height: 1.7;
          white-space: pre-line;
          margin: 0;
        }
      `}</style>
    </section>
  );
}
