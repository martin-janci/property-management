'use client';

/**
 * Price map page — choropleth of city/district prices.
 *
 * Every figure on this page is live: it comes from reality-server
 * `GET /api/v1/price-map` via `usePriceMap`, which aggregates avg €/m² from
 * the listings table. The only static config is `_geometry.ts` (SVG outlines
 * + colour scale) — there is no hardcoded price dataset.
 *
 * Screen-map: docs/screens/reality/price-map.md
 */

import { type DistrictPriceData, usePriceMap } from '@ppt/reality-api-client';
import { useTranslations } from 'next-intl';
import { useMemo, useState } from 'react';
import { Footer, Header } from '@/components/ui';
import {
  colorForPrice,
  geometryFor,
  LEGEND_ITEMS,
  type PriceMapFilter,
} from './_geometry';

// TODO: replace filter strip with @ppt/ui-kit/ChipGroup once available
// TODO: replace with @ppt/ui-kit/SegmentedControl for transactionType toggle

type TransactionType = 'sale' | 'rent';
type PropertyType = 'all' | 'apartment' | 'house' | 'land';

const PROPERTY_OPTIONS: { value: PropertyType; label: string }[] = [
  { value: 'all', label: 'Všetky' },
  { value: 'apartment', label: 'Byty' },
  { value: 'house', label: 'Domy' },
  { value: 'land', label: 'Pozemky' },
];

const formatPrice = (v: number) => `${Math.round(v).toLocaleString('sk-SK')} €`;

/** Short display label — drop the "Bratislava I – " prefix when present. */
const shortName = (name: string) => name.split(' – ')[1] ?? name;

interface Insight {
  label: string;
  value: string;
  trend: string;
}

/** Derive the insights band entirely from the live district rows. */
function computeInsights(districts: DistrictPriceData[]): Insight[] {
  const priced = districts.filter(
    (d) => d.avg_price_per_m2 != null && d.listing_count > 0,
  );

  // Listing-count-weighted average €/m² across all priced districts.
  const totalListings = districts.reduce((s, d) => s + d.listing_count, 0);
  const weightSum = priced.reduce((s, d) => s + d.listing_count, 0);
  const weightedAvg =
    weightSum > 0
      ? priced.reduce((s, d) => s + (d.avg_price_per_m2 ?? 0) * d.listing_count, 0) /
        weightSum
      : null;

  const mostExpensive = priced.reduce<DistrictPriceData | null>(
    (best, d) =>
      best == null || (d.avg_price_per_m2 ?? 0) > (best.avg_price_per_m2 ?? 0)
        ? d
        : best,
    null,
  );

  const grew = districts.filter((d) => d.trend_pct_qoq != null);
  const fastestGrowth = grew.reduce<DistrictPriceData | null>(
    (best, d) =>
      best == null || (d.trend_pct_qoq ?? 0) > (best.trend_pct_qoq ?? 0) ? d : best,
    null,
  );

  return [
    {
      label: 'Priemerná cena / m²',
      value: weightedAvg != null ? formatPrice(weightedAvg) : '—',
      trend: `${districts.length} ${districts.length === 1 ? 'lokalita' : 'lokalít'}`,
    },
    {
      label: 'Najdrahšia štvrť',
      value: mostExpensive ? shortName(mostExpensive.district_name) : '—',
      trend:
        mostExpensive?.avg_price_per_m2 != null
          ? `${formatPrice(mostExpensive.avg_price_per_m2)} / m²`
          : '—',
    },
    {
      label: 'Najväčší rast',
      value: fastestGrowth ? shortName(fastestGrowth.district_name) : '—',
      trend:
        fastestGrowth?.trend_pct_qoq != null
          ? `${fastestGrowth.trend_pct_qoq > 0 ? '+' : ''}${fastestGrowth.trend_pct_qoq} % (12m)`
          : '—',
    },
    {
      label: 'Aktívne inzeráty',
      value: totalListings.toLocaleString('sk-SK'),
      trend: 'spolu vo výbere',
    },
  ];
}

export default function PriceMapPage() {
  const t = useTranslations('pages.priceMap');
  const [filter, setFilter] = useState<PriceMapFilter>({
    propertyType: 'all',
    transactionType: 'sale',
  });
  const [selectedId, setSelectedId] = useState<string | null>(null);

  const {
    data,
    isLoading,
    isError,
    refetch,
    isFetching,
  } = usePriceMap({
    mode: filter.transactionType,
    propertyType: filter.propertyType === 'all' ? undefined : filter.propertyType,
    timeWindow: '12m',
  });

  const districts = data?.districts ?? [];
  const selected = districts.find((d) => d.district_id === selectedId) ?? null;
  const insights = useMemo(() => computeInsights(districts), [districts]);

  // Districts we can actually draw on the SVG. When none match geometry we
  // drop the map column and let the list span the full width — no empty map.
  const mapDistricts = districts.filter((d) => geometryFor(d.district_name));
  const showMap = mapDistricts.length > 0;

  return (
    <div
      data-i18n="pages.price-map.root"
      style={{
        minHeight: '100vh',
        display: 'flex',
        flexDirection: 'column',
        background: 'var(--ppt-bg-app)',
      }}
    >
      <Header />

      <main style={{ flex: 1 }}>
        {/* Filter strip */}
        <div
          style={{
            background: 'var(--ppt-bg-surface)',
            borderBottom: '1px solid var(--ppt-border-default, #e5e7eb)',
            padding: '14px 24px',
            display: 'flex',
            gap: 16,
            alignItems: 'center',
            flexWrap: 'wrap',
          }}
        >
          <h1
            style={{
              fontSize: '1.25rem',
              fontWeight: 800,
              color: 'var(--ppt-fg-primary)',
              margin: 0,
              flexShrink: 0,
            }}
          >
            {t('h1')}
          </h1>
          <div style={{ display: 'flex', gap: 4 }}>
            {/* TODO: replace with @ppt/ui-kit/SegmentedControl once available */}
            {(['sale', 'rent'] as TransactionType[]).map((tx) => (
              <button
                key={tx}
                type="button"
                onClick={() => {
                  setSelectedId(null);
                  setFilter((f) => ({ ...f, transactionType: tx }));
                }}
                style={{
                  padding: '7px 16px',
                  borderRadius: 6,
                  border: 'none',
                  background:
                    filter.transactionType === tx
                      ? 'var(--ppt-color-primary, #2563eb)'
                      : 'var(--ppt-bg-app, #f1f5f9)',
                  color: filter.transactionType === tx ? '#fff' : 'var(--ppt-fg-secondary)',
                  fontWeight: 600,
                  cursor: 'pointer',
                  fontSize: '0.875rem',
                }}
              >
                {tx === 'sale' ? 'Predaj' : 'Prenájom'}
              </button>
            ))}
          </div>
          {/* TODO: replace with @ppt/ui-kit/ChipGroup once available */}
          <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
            {PROPERTY_OPTIONS.map((opt) => (
              <button
                key={opt.value}
                type="button"
                onClick={() => {
                  setSelectedId(null);
                  setFilter((f) => ({ ...f, propertyType: opt.value }));
                }}
                style={{
                  padding: '6px 14px',
                  borderRadius: 99,
                  border: `1px solid ${filter.propertyType === opt.value ? 'var(--ppt-color-primary, #2563eb)' : 'var(--ppt-border-default, #e5e7eb)'}`,
                  background:
                    filter.propertyType === opt.value
                      ? 'var(--ppt-color-primary-light, #dbeafe)'
                      : 'transparent',
                  color:
                    filter.propertyType === opt.value
                      ? 'var(--ppt-color-primary, #2563eb)'
                      : 'var(--ppt-fg-secondary)',
                  fontWeight: filter.propertyType === opt.value ? 600 : 400,
                  cursor: 'pointer',
                  fontSize: '0.875rem',
                }}
              >
                {opt.label}
              </button>
            ))}
          </div>
          {isFetching && !isLoading && (
            <span style={{ fontSize: '0.8125rem', color: 'var(--ppt-fg-muted, #9ca3af)' }}>
              Aktualizujem…
            </span>
          )}
        </div>

        {/* Loading state */}
        {isLoading && (
          <div
            style={{
              minHeight: 400,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              color: 'var(--ppt-fg-secondary)',
              fontSize: '0.9375rem',
            }}
          >
            Načítavam cenové dáta…
          </div>
        )}

        {/* Error state */}
        {!isLoading && isError && (
          <div
            style={{
              minHeight: 400,
              display: 'flex',
              flexDirection: 'column',
              alignItems: 'center',
              justifyContent: 'center',
              gap: 14,
              padding: 24,
              textAlign: 'center',
            }}
          >
            <p style={{ color: 'var(--ppt-fg-primary)', fontWeight: 600, margin: 0 }}>
              Cenové dáta sa nepodarilo načítať.
            </p>
            <button
              type="button"
              onClick={() => refetch()}
              style={{
                padding: '9px 18px',
                background: 'var(--ppt-color-primary, #2563eb)',
                color: '#fff',
                border: 'none',
                borderRadius: 8,
                fontWeight: 700,
                cursor: 'pointer',
                fontSize: '0.875rem',
              }}
            >
              Skúsiť znova
            </button>
          </div>
        )}

        {/* Empty state */}
        {!isLoading && !isError && districts.length === 0 && (
          <div
            style={{
              minHeight: 400,
              display: 'flex',
              flexDirection: 'column',
              alignItems: 'center',
              justifyContent: 'center',
              gap: 8,
              padding: 24,
              textAlign: 'center',
            }}
          >
            <p style={{ color: 'var(--ppt-fg-primary)', fontWeight: 600, margin: 0 }}>
              Pre zvolený filter zatiaľ nie sú dostupné žiadne cenové dáta.
            </p>
            <p style={{ color: 'var(--ppt-fg-secondary)', margin: 0, fontSize: '0.875rem' }}>
              Skúste zmeniť typ transakcie alebo typ nehnuteľnosti.
            </p>
          </div>
        )}

        {/* Data */}
        {!isLoading && !isError && districts.length > 0 && (
          <>
            <div className={`price-map-grid${showMap ? '' : ' price-map-grid--single'}`}>
              {/* SVG Map — only when at least one district has geometry */}
              {showMap && (
                <div
                  style={{
                    background: 'var(--ppt-bg-subtle, #f1f5f9)',
                    position: 'relative',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    padding: 24,
                  }}
                >
                  <svg
                    viewBox="0 80 400 200"
                    style={{ width: '100%', maxWidth: 600, height: 'auto' }}
                    aria-label="Mapa mestských štvrtí"
                  >
                    {mapDistricts.map((district) => {
                      const path = geometryFor(district.district_name);
                      if (!path) return null;
                      const fill = colorForPrice(district.avg_price_per_m2);
                      const priceLabel =
                        district.avg_price_per_m2 != null
                          ? `${formatPrice(district.avg_price_per_m2)} / m²`
                          : 'bez dát';
                      return (
                        <path
                          key={district.district_id}
                          d={path}
                          fill={fill}
                          opacity={selectedId === district.district_id ? 1 : 0.7}
                          stroke="#fff"
                          strokeWidth={2}
                          style={{ cursor: 'pointer', transition: 'opacity .15s' }}
                          onClick={() =>
                            setSelectedId(
                              selectedId === district.district_id
                                ? null
                                : district.district_id,
                            )
                          }
                          aria-label={district.district_name}
                        >
                          {/* Single template-string child avoids the SSR/CSR
                              whitespace-handling mismatch that triggered Next.js's
                              "hydration error" overlay on first paint. */}
                          <title>{`${district.district_name}: ${priceLabel}`}</title>
                        </path>
                      );
                    })}
                  </svg>

                  {/* Legend */}
                  <div
                    style={{
                      position: 'absolute',
                      bottom: 24,
                      left: 24,
                      background: 'rgba(255,255,255,.9)',
                      borderRadius: 8,
                      padding: '10px 14px',
                      fontSize: '0.8125rem',
                      color: 'var(--ppt-fg-secondary)',
                    }}
                  >
                    <div
                      style={{ fontWeight: 600, marginBottom: 6, color: 'var(--ppt-fg-primary)' }}
                    >
                      Cena / m²
                    </div>
                    {LEGEND_ITEMS.map((item) => (
                      <div
                        key={item.label}
                        style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 3 }}
                      >
                        <span
                          style={{
                            width: 14,
                            height: 14,
                            borderRadius: 3,
                            background: item.color,
                            flexShrink: 0,
                          }}
                        />
                        {item.label}
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {/* Right panel */}
              <div
                style={{
                  background: 'var(--ppt-bg-surface)',
                  borderLeft: showMap ? '1px solid var(--ppt-border-default, #e5e7eb)' : 'none',
                  overflow: 'auto',
                }}
              >
                {selected ? (
                  <div style={{ padding: '28px 24px' }}>
                    <button
                      type="button"
                      onClick={() => setSelectedId(null)}
                      style={{
                        background: 'none',
                        border: 'none',
                        color: 'var(--ppt-color-primary, #2563eb)',
                        cursor: 'pointer',
                        padding: 0,
                        marginBottom: 16,
                        fontSize: '0.875rem',
                      }}
                    >
                      ← Späť na prehľad
                    </button>
                    <h2
                      style={{
                        fontSize: '1.25rem',
                        fontWeight: 800,
                        color: 'var(--ppt-fg-primary)',
                        margin: '0 0 20px',
                      }}
                    >
                      {selected.district_name}
                    </h2>
                    <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
                      {[
                        {
                          label: 'Priem. cena / m²',
                          value:
                            selected.avg_price_per_m2 != null
                              ? formatPrice(selected.avg_price_per_m2)
                              : '—',
                        },
                        {
                          label: 'Zmena (12 mesiacov)',
                          value:
                            selected.trend_pct_qoq != null
                              ? `${selected.trend_pct_qoq > 0 ? '+' : ''}${selected.trend_pct_qoq} %`
                              : '—',
                          positive:
                            selected.trend_pct_qoq != null ? selected.trend_pct_qoq > 0 : undefined,
                        },
                        {
                          label: 'Aktívne inzeráty',
                          value: selected.listing_count.toLocaleString('sk-SK'),
                        },
                      ].map((row) => (
                        <div
                          key={row.label}
                          style={{
                            display: 'flex',
                            justifyContent: 'space-between',
                            alignItems: 'center',
                          }}
                        >
                          <span style={{ fontSize: '0.875rem', color: 'var(--ppt-fg-secondary)' }}>
                            {row.label}
                          </span>
                          <span
                            style={{
                              fontWeight: 700,
                              color:
                                'positive' in row && row.positive !== undefined
                                  ? row.positive
                                    ? 'var(--ppt-color-success-dark, #047857)'
                                    : 'var(--ppt-color-danger, #ef4444)'
                                  : 'var(--ppt-fg-primary)',
                            }}
                          >
                            {row.value}
                          </span>
                        </div>
                      ))}
                    </div>
                    <a
                      href={`/listings?city=${encodeURIComponent(selected.district_name)}`}
                      style={{
                        display: 'block',
                        marginTop: 24,
                        padding: '11px',
                        background: 'var(--ppt-color-primary, #2563eb)',
                        color: '#fff',
                        borderRadius: 8,
                        textAlign: 'center',
                        fontWeight: 700,
                        textDecoration: 'none',
                        fontSize: '0.9375rem',
                      }}
                    >
                      Zobraziť inzeráty v tejto lokalite
                    </a>
                  </div>
                ) : (
                  <div style={{ padding: '28px 24px' }}>
                    <h2
                      style={{
                        fontSize: '1.0625rem',
                        fontWeight: 700,
                        color: 'var(--ppt-fg-primary)',
                        margin: '0 0 6px',
                      }}
                    >
                      {t('subtitle')}
                    </h2>
                    <p
                      style={{
                        fontSize: '0.875rem',
                        color: 'var(--ppt-fg-secondary)',
                        marginBottom: 20,
                      }}
                    >
                      {t('clickHint')}
                    </p>
                    <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
                      {districts.map((d) => (
                        <button
                          key={d.district_id}
                          type="button"
                          onClick={() => setSelectedId(d.district_id)}
                          style={{
                            display: 'flex',
                            justifyContent: 'space-between',
                            alignItems: 'center',
                            padding: '12px 14px',
                            background: 'var(--ppt-bg-subtle, #f8fafc)',
                            borderRadius: 8,
                            border: '1px solid var(--ppt-border-default, #e5e7eb)',
                            cursor: 'pointer',
                            textAlign: 'left',
                          }}
                        >
                          <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
                            <span
                              style={{
                                width: 12,
                                height: 12,
                                borderRadius: 3,
                                background: colorForPrice(d.avg_price_per_m2),
                                flexShrink: 0,
                              }}
                            />
                            <span
                              style={{
                                fontSize: '0.875rem',
                                color: 'var(--ppt-fg-primary)',
                                fontWeight: 500,
                              }}
                            >
                              {shortName(d.district_name)}
                            </span>
                          </div>
                          <span
                            style={{
                              fontSize: '0.875rem',
                              fontWeight: 700,
                              color: 'var(--ppt-fg-primary)',
                              flexShrink: 0,
                            }}
                          >
                            {d.avg_price_per_m2 != null
                              ? `${formatPrice(d.avg_price_per_m2)}/m²`
                              : '—'}
                          </span>
                        </button>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            </div>

            {/* Insights band */}
            <div
              style={{
                background: 'var(--ppt-bg-surface)',
                borderTop: '1px solid var(--ppt-border-default, #e5e7eb)',
                padding: '24px',
                display: 'grid',
                gridTemplateColumns: 'repeat(auto-fill, minmax(180px, 1fr))',
                gap: 16,
              }}
            >
              {insights.map((insight) => (
                <div key={insight.label}>
                  <div
                    style={{
                      fontSize: '0.75rem',
                      color: 'var(--ppt-fg-muted, #9ca3af)',
                      textTransform: 'uppercase',
                      letterSpacing: '.5px',
                      marginBottom: 4,
                    }}
                  >
                    {insight.label}
                  </div>
                  <div
                    style={{
                      fontSize: '1.25rem',
                      fontWeight: 800,
                      color: 'var(--ppt-fg-primary)',
                      marginBottom: 2,
                    }}
                  >
                    {insight.value}
                  </div>
                  <div style={{ fontSize: '0.8125rem', color: 'var(--ppt-fg-secondary)' }}>
                    {insight.trend}
                  </div>
                </div>
              ))}
            </div>
          </>
        )}
      </main>

      <Footer />

      <style jsx>{`
        /* Map + detail-panel grid. Stacks on mobile so the fixed 340 px
           panel doesn't overflow the viewport (+13 px observed at 375 px
           before this rule). Height clamp stays in both layouts. */
        .price-map-grid {
          display: grid;
          grid-template-columns: 1fr;
          min-height: 500px;
        }
        @media (min-width: 768px) {
          .price-map-grid {
            grid-template-columns: 1fr 340px;
            height: calc(100vh - 200px);
          }
          /* No geometry to draw → list spans full width, no empty map column. */
          .price-map-grid--single {
            grid-template-columns: 1fr;
          }
        }
      `}</style>
    </div>
  );
}
