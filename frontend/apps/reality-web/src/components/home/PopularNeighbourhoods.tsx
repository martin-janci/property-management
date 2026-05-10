'use client';

/**
 * Popular neighbourhoods — homepage section #4 in the design system.
 *
 * The reality-server doesn't aggregate listings by district yet, so this is
 * a curated marketing surface (matches the design which also hardcoded six
 * neighbourhoods). Each card links to `/listings?city=…&district=…` so it
 * doubles as discovery — when the API ships an aggregation endpoint we can
 * swap the static list for a hook without touching the markup.
 *
 * Design reference:
 *   project/Reality Portal Home - Standalone.html `section.wrap` #4
 *   project/colors_and_type.css `.nbh / .nbhcard / .nimg / .nbody`
 */

import { useTranslations } from 'next-intl';
import { Link } from '../../i18n/routing';

type Neighbourhood = {
  /** URL-safe id used as the React key and the gradient `data-tone` selector. */
  id: string;
  /** District label (translated client-side). */
  nameKey: string;
  /** Parent city label (already localised in messages). */
  cityKey: string;
  /** Query for /listings — kept here so the component owns its routing. */
  city: string;
  district: string;
  /** Two-stop gradient for the photo placeholder (no real images yet). */
  tone: { from: string; to: string };
};

const NEIGHBOURHOODS: Neighbourhood[] = [
  {
    id: 'ruzinov',
    nameKey: 'ruzinov',
    cityKey: 'bratislava',
    city: 'Bratislava',
    district: 'Ružinov',
    tone: { from: '#60a5fa', to: '#1e40af' },
  },
  {
    id: 'stare-mesto',
    nameKey: 'staremesto',
    cityKey: 'bratislava',
    city: 'Bratislava',
    district: 'Staré Mesto',
    tone: { from: '#fbbf24', to: '#d97706' },
  },
  {
    id: 'petrzalka',
    nameKey: 'petrzalka',
    cityKey: 'bratislava',
    city: 'Bratislava',
    district: 'Petržalka',
    tone: { from: '#34d399', to: '#047857' },
  },
  {
    id: 'nove-mesto',
    nameKey: 'novemesto',
    cityKey: 'bratislava',
    city: 'Bratislava',
    district: 'Nové Mesto',
    tone: { from: '#a78bfa', to: '#5b21b6' },
  },
  {
    id: 'karlova-ves',
    nameKey: 'karlovaves',
    cityKey: 'bratislava',
    city: 'Bratislava',
    district: 'Karlova Ves',
    tone: { from: '#f472b6', to: '#9d174d' },
  },
  {
    id: 'dubravka',
    nameKey: 'dubravka',
    cityKey: 'bratislava',
    city: 'Bratislava',
    district: 'Dúbravka',
    tone: { from: '#22d3ee', to: '#155e75' },
  },
];

export function PopularNeighbourhoods() {
  const t = useTranslations('home.neighbourhoods');
  const tCity = useTranslations('home.city');

  return (
    <section className="section">
      <div className="container">
        <div className="sec-hd">
          <div>
            <h2 className="title">{t('title')}</h2>
            <p className="subtitle">{t('subtitle')}</p>
          </div>
          <Link href="/price-map" className="see-all">
            {t('exploreMap')}
          </Link>
        </div>
        <div className="grid">
          {NEIGHBOURHOODS.map((n) => (
            <Link
              key={n.id}
              href={{
                pathname: '/listings',
                query: { city: n.city, district: n.district },
              }}
              className="card"
            >
              <div
                className="img"
                style={{
                  // CSS-vars feed `linear-gradient(135deg, var(--a), var(--b))`
                  // — picked up by the ::after overlay below.
                  ['--a' as string]: n.tone.from,
                  ['--b' as string]: n.tone.to,
                }}
              >
                <div className="lbl">
                  <strong>{t(n.nameKey)}</strong>
                  <small>{tCity(n.cityKey)}</small>
                </div>
              </div>
              <div className="body">
                <span className="cta">{t('browse')}</span>
                <span className="arrow" aria-hidden="true">
                  →
                </span>
              </div>
            </Link>
          ))}
        </div>
      </div>

      <style jsx>{`
        .section {
          padding: 40px 32px 24px;
          background: var(--ppt-bg-app);
        }
        .container {
          max-width: var(--ppt-content-max, 1280px);
          margin: 0 auto;
        }
        .sec-hd {
          display: flex;
          justify-content: space-between;
          align-items: flex-end;
          margin-bottom: 24px;
          gap: 16px;
        }
        .title {
          font-size: 22px;
          font-weight: var(--ppt-font-weight-bold, 700);
          color: var(--ppt-fg-primary);
          margin: 0;
          letter-spacing: -0.01em;
        }
        .subtitle {
          font-size: var(--ppt-font-size-sm, 14px);
          color: var(--ppt-fg-muted);
          margin: 4px 0 0;
        }
        .see-all {
          color: var(--ppt-color-primary);
          font-size: 13.5px;
          font-weight: var(--ppt-font-weight-semibold, 600);
          text-decoration: none;
          flex-shrink: 0;
        }
        .see-all::after {
          content: ' →';
        }
        .see-all:hover {
          text-decoration: underline;
        }
        .grid {
          display: grid;
          grid-template-columns: repeat(3, 1fr);
          gap: 16px;
        }
        @media (max-width: 1024px) {
          .grid {
            grid-template-columns: repeat(2, 1fr);
          }
        }
        @media (max-width: 640px) {
          .grid {
            grid-template-columns: 1fr;
          }
          .section {
            padding: 32px 16px 16px;
          }
        }
        .card {
          display: block;
          background: var(--ppt-bg-surface);
          border: 1px solid var(--ppt-border-default);
          border-radius: 12px;
          overflow: hidden;
          text-decoration: none;
          color: inherit;
          transition: border-color var(--ppt-transition-fast),
            box-shadow var(--ppt-transition-fast),
            transform var(--ppt-transition-fast);
        }
        .card:hover {
          border-color: var(--ppt-color-primary);
          box-shadow: var(--ppt-shadow-lg, 0 4px 12px rgba(0, 0, 0, 0.15));
          transform: translateY(-2px);
        }
        .card:focus-visible {
          outline: none;
          box-shadow: var(--ppt-focus-ring-shadow);
        }
        .img {
          aspect-ratio: 2 / 1;
          background: linear-gradient(135deg, var(--a), var(--b));
          position: relative;
        }
        .img::after {
          content: '';
          position: absolute;
          inset: 0;
          background: linear-gradient(
            180deg,
            transparent 40%,
            rgba(0, 0, 0, 0.35) 100%
          );
        }
        .lbl {
          position: absolute;
          left: 14px;
          bottom: 12px;
          z-index: 2;
          color: #fff;
          line-height: 1.3;
        }
        .lbl strong {
          display: block;
          font-size: 17px;
          font-weight: var(--ppt-font-weight-bold, 700);
        }
        .lbl small {
          font-size: 12px;
          opacity: 0.9;
        }
        .body {
          padding: 14px 16px;
          display: flex;
          justify-content: space-between;
          align-items: center;
        }
        .cta {
          font-size: 13px;
          color: var(--ppt-fg-secondary);
          font-weight: var(--ppt-font-weight-medium, 500);
        }
        .arrow {
          color: var(--ppt-color-primary);
          font-weight: var(--ppt-font-weight-bold, 700);
        }
      `}</style>
    </section>
  );
}
