'use client';

/**
 * Value strip — homepage section #5 in the design system.
 *
 * Four-cell stat bar that sits between Featured listings and the closing
 * CTA. Marketing copy lives in i18n; the numeric values (verified-agents
 * count, building-passport coverage %, price-history horizon, mortgage
 * pre-approval SLA) are hardcoded as marketing claims because nothing in
 * the API exposes them yet.
 *
 * Design reference:
 *   project/Reality Portal Home - Standalone.html `section.value`
 *   project/colors_and_type.css `.value / .value-inner / .vcell`
 */

import { useTranslations } from 'next-intl';

const CELLS = [
  { key: 'agents', value: '1,120' },
  { key: 'passport', value: '98%' },
  { key: 'history', value: '5y' },
  { key: 'mortgage', value: '48h' },
] as const;

export function ValueStrip() {
  const t = useTranslations('home.value');
  return (
    <section className="value">
      <div className="inner">
        {CELLS.map((c) => (
          <div key={c.key} className="cell">
            <div className="label">{t(`${c.key}.label`)}</div>
            <div className="num">{c.value}</div>
            <div className="desc">{t(`${c.key}.desc`)}</div>
          </div>
        ))}
      </div>

      <style jsx>{`
        .value {
          background: var(--ppt-bg-surface);
          border-top: 1px solid var(--ppt-border-default);
          border-bottom: 1px solid var(--ppt-border-default);
          padding: 32px 0;
          margin-top: 40px;
        }
        .inner {
          max-width: var(--ppt-content-max, 1280px);
          margin: 0 auto;
          padding: 0 32px;
          display: grid;
          grid-template-columns: repeat(4, 1fr);
          gap: 32px;
        }
        @media (max-width: 1024px) {
          .inner {
            grid-template-columns: repeat(2, 1fr);
            row-gap: 24px;
          }
        }
        @media (max-width: 640px) {
          .inner {
            grid-template-columns: 1fr;
            padding: 0 16px;
          }
        }
        .cell {
          display: flex;
          flex-direction: column;
          gap: 4px;
        }
        .label {
          font-size: 12px;
          color: var(--ppt-fg-muted);
          text-transform: uppercase;
          letter-spacing: 0.06em;
          font-weight: var(--ppt-font-weight-semibold, 600);
        }
        .num {
          font-size: 26px;
          font-weight: 800;
          color: var(--ppt-fg-primary);
          letter-spacing: -0.01em;
        }
        .desc {
          font-size: 13px;
          color: var(--ppt-fg-secondary);
          margin-top: 2px;
          /* German runs ~35% longer than English — design rule says wrap, never truncate. */
          overflow-wrap: break-word;
          hyphens: auto;
        }
      `}</style>
    </section>
  );
}
