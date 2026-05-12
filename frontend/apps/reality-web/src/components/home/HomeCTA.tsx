'use client';

/**
 * Selling-CTA — homepage section #6 in the design system.
 *
 * Soft-blue panel that closes the homepage above the footer. Two actions
 * (secondary "Learn more" → /about, primary "Start listing" → /sell).
 *
 * Design reference:
 *   project/Reality Portal Home - Standalone.html `section.cta`
 *   project/colors_and_type.css `.cta / .btn-pri / .btn-sec`
 */

import { useTranslations } from 'next-intl';
import { Link } from '../../i18n/routing';

export function HomeCTA() {
  const t = useTranslations('home.cta');
  return (
    <section className="cta-wrap">
      <div className="cta">
        <div className="copy">
          <h3 className="title">{t('title')}</h3>
          <p className="body">{t('body')}</p>
        </div>
        <div className="actions">
          <Link href="/about" className="btn-sec">
            {t('learnMore')}
          </Link>
          <Link href="/sell" className="btn-pri">
            {t('startListing')}
          </Link>
        </div>
      </div>

      <style jsx>{`
        .cta-wrap {
          padding: 16px 32px 48px;
        }
        .cta {
          max-width: var(--ppt-content-max, 1280px);
          margin: 40px auto 0;
          padding: 36px 40px;
          background: var(--ppt-color-primary-soft-bg, #eff6ff);
          border: 1px solid var(--ppt-color-primary-soft-border, #dbeafe);
          border-radius: 14px;
          display: grid;
          grid-template-columns: 1fr auto;
          gap: 24px;
          align-items: center;
        }
        .copy {
          min-width: 0;
        }
        .title {
          font-size: 22px;
          font-weight: var(--ppt-font-weight-bold, 700);
          color: var(--ppt-fg-primary);
          margin: 0 0 6px;
          letter-spacing: -0.01em;
        }
        .body {
          font-size: var(--ppt-font-size-sm, 14px);
          color: var(--ppt-fg-secondary);
          margin: 0;
          max-width: 560px;
          line-height: 1.5;
        }
        .actions {
          display: flex;
          gap: 8px;
          flex-shrink: 0;
        }
        @media (max-width: 768px) {
          .cta {
            grid-template-columns: 1fr;
            padding: 28px 24px;
          }
          .actions {
            flex-wrap: wrap;
          }
          .cta-wrap {
            padding: 8px 16px 32px;
          }
        }
        /*
         * :global() — Link from next-intl/navigation renders its own <a>,
         * which styled-jsx can't tag with the component's hashed className.
         * Without :global(), .btn-pri / .btn-sec selectors won't match the
         * Link-rendered <a> and the buttons render as plain text. Scoping
         * stays tight because the wrapping .actions block still has the
         * hashed class — these rules only target classNames we set ourselves.
         */
        .actions :global(.btn-pri),
        .actions :global(.btn-sec) {
          padding: 10px 16px;
          font-size: 13.5px;
          font-weight: var(--ppt-font-weight-semibold, 600);
          border-radius: 8px;
          text-decoration: none;
          display: inline-flex;
          align-items: center;
          justify-content: center;
          transition: background var(--ppt-transition-fast),
            border-color var(--ppt-transition-fast);
        }
        .actions :global(.btn-pri) {
          background: var(--ppt-color-primary);
          color: var(--ppt-fg-on-accent, #fff);
          border: 1px solid var(--ppt-color-primary);
        }
        .actions :global(.btn-pri):hover {
          background: var(--ppt-color-primary-hover);
          border-color: var(--ppt-color-primary-hover);
        }
        .actions :global(.btn-sec) {
          background: var(--ppt-bg-surface);
          color: var(--ppt-fg-secondary);
          border: 1px solid var(--ppt-border-default);
        }
        .actions :global(.btn-sec):hover {
          background: var(--ppt-bg-subtle);
          border-color: var(--ppt-border-strong);
        }
        .actions :global(.btn-pri):focus-visible,
        .actions :global(.btn-sec):focus-visible {
          outline: none;
          box-shadow: var(--ppt-focus-ring-shadow);
        }
      `}</style>
    </section>
  );
}
