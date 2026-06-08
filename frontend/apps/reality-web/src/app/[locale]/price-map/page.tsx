'use client';

/**
 * Price map page — choropleth of district/city prices.
 * Screen-map: docs/screens/reality/price-map.md
 *
 * Data comes from reality-server's `/api/v1/price-map` aggregation via
 * `usePriceMap` (@ppt/reality-api-client). The server groups by city and
 * carries no map geometry, so SVG outlines live in `_geometry.ts` as
 * presentational scaffolding; every district the API returns still appears
 * in the list and insights even without a matching outline.
 */

import {
  type DistrictPriceData,
  type PriceMapPropertyType,
  usePriceMap,
} from '@ppt/reality-api-client';
import { useTranslations } from 'next-intl';
import { useMemo, useState } from 'react';
import { Footer, Header } from '@/components/ui';
import { geometryFor, priceToColor } from './_geometry';

// TODO: replace filter strip with @ppt/ui-kit/ChipGroup once available
// TODO: replace with @ppt/ui-kit/SegmentedControl for transactionType toggle

type TransactionType = 'sale' | 'rent';

const PROPERTY_OPTIONS: { value: PriceMapPropertyType; label: string }[] = [
  { value: 'all', label: 'Všetky' },
  { value: 'apartment', label: 'Byty' },
  { value: 'house', label: 'Domy' },
  { value: 'land', label: 'Pozemky' },
];

const formatPrice = (v: number | null | undefined) =>
  v == null ? '—' : `${Math.round(v).toLocaleString('sk-SK')} €`;

export default function PriceMapPage() {
  const t = useTranslations('pages.priceMap');
  const [transactionType, setTransactionType] = useState<TransactionType>('sale');
  const [propertyType, setPropertyType] = useState<PriceMapPropertyType>('all');
  const [selectedId, setSelectedId] = useState<string | null>(null);

  const { data, isLoading, isError } = usePriceMap({
    mode: transactionType,
    propertyType,
  });

  const districts: DistrictPriceData[] = data?.districts ?? [];
  const selected = districts.find((d) => d.district_id === selectedId) ?? null;

  // Insights derived from live data — no mock constants.
  const insights = useMemo(() => {
    const priced = districts.filter((d) => d.avg_price_per_m2 != null);
    const avgPpm2 = priced.length
      ? priced.reduce((s, d) => s + (d.avg_price_per_m2 as number), 0) / priced.length
      : null;
    const dearest = priced.reduce<DistrictPriceData | null>(
      (best, d) =>
        best == null || (d.avg_price_per_m2 as number) > (best.avg_price_per_m2 as number)
          ? d
          : best,
      null
    );
    const trending = districts
      .filter((d) => d.trend_pct_qoq != null)
      .reduce<DistrictPriceData | null>(
        (best, d) =>
          best == null || (d.trend_pct_qoq as number) > (best.trend_pct_qoq as number) ? d : best,
        null
      );
    const totalListings = districts.reduce((s, d) => s + d.listing_count, 0);

    return [
      {
        label: 'Priemerná cena / m²',
        value: formatPrice(avgPpm2),
        trend: `${priced.length} lokalít`,
      },
      {
        label: 'Najdrahšia lokalita',
        value: dearest?.district_name ?? '—',
        trend: dearest ? `${formatPrice(dearest.avg_price_per_m2)} / m²` : '',
      },
      {
        label: 'Najväčší rast',
        value: trending?.district_name ?? '—',
        trend:
          trending?.trend_pct_qoq != null
            ? `${trending.trend_pct_qoq > 0 ? '+' : ''}${trending.trend_pct_qoq} % QoQ`
            : '',
      },
      {
        label: 'Aktívne inzeráty',
        value: totalListings.toLocaleString('sk-SK'),
        trend: `${districts.length} lokalít`,
      },
    ];
  }, [districts]);

  const mappable = districts.filter((d) => geometryFor(d.district_id, d.district_name));

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
                onClick={() => setTransactionType(tx)}
                style={{
                  padding: '7px 16px',
                  borderRadius: 6,
                  border: 'none',
                  background:
                    transactionType === tx
                      ? 'var(--ppt-color-primary, #2563eb)'
                      : 'var(--ppt-bg-app, #f1f5f9)',
                  color: transactionType === tx ? '#fff' : 'var(--ppt-fg-secondary)',
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
                onClick={() => setPropertyType(opt.value)}
                style={{
                  padding: '6px 14px',
                  borderRadius: 99,
                  border: `1px solid ${propertyType === opt.value ? 'var(--ppt-color-primary, #2563eb)' : 'var(--ppt-border-default, #e5e7eb)'}`,
                  background:
                    propertyType === opt.value
                      ? 'var(--ppt-color-primary-light, #dbeafe)'
                      : 'transparent',
                  color:
                    propertyType === opt.value
                      ? 'var(--ppt-color-primary, #2563eb)'
                      : 'var(--ppt-fg-secondary)',
                  fontWeight: propertyType === opt.value ? 600 : 400,
                  cursor: 'pointer',
                  fontSize: '0.875rem',
                }}
              >
                {opt.label}
              </button>
            ))}
          </div>
        </div>

        <div className="price-map-grid">
          {/* SVG Map */}
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
            {isLoading ? (
              <div style={{ color: 'var(--ppt-fg-secondary)', fontSize: '0.9375rem' }}>
                Načítavam dáta…
              </div>
            ) : isError ? (
              <div style={{ color: 'var(--ppt-color-danger, #ef4444)', fontSize: '0.9375rem' }}>
                Dáta o cenách sa nepodarilo načítať.
              </div>
            ) : mappable.length === 0 ? (
              <div
                style={{
                  color: 'var(--ppt-fg-secondary)',
                  fontSize: '0.9375rem',
                  textAlign: 'center',
                  maxWidth: 360,
                }}
              >
                Pre vybrané lokality zatiaľ nie je dostupný mapový podklad. Štatistiky nájdete v
                zozname vpravo.
              </div>
            ) : (
              <svg
                viewBox="0 80 400 200"
                style={{ width: '100%', maxWidth: 600, height: 'auto' }}
                aria-label="Mapa cien podľa lokality"
              >
                {mappable.map((district) => (
                  <path
                    key={district.district_id}
                    d={geometryFor(district.district_id, district.district_name)}
                    fill={priceToColor(district.avg_price_per_m2)}
                    opacity={selectedId === district.district_id ? 1 : 0.7}
                    stroke="#fff"
                    strokeWidth={2}
                    style={{ cursor: 'pointer', transition: 'opacity .15s' }}
                    onClick={() =>
                      setSelectedId(
                        selectedId === district.district_id ? null : district.district_id
                      )
                    }
                    aria-label={district.district_name}
                  >
                    {/* Single template-string child avoids the SSR/CSR
                        whitespace-handling mismatch that triggered Next.js's
                        "hydration error" overlay on first paint. */}
                    <title>{`${district.district_name}: ${formatPrice(district.avg_price_per_m2)} / m²`}</title>
                  </path>
                ))}
              </svg>
            )}

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
              <div style={{ fontWeight: 600, marginBottom: 6, color: 'var(--ppt-fg-primary)' }}>
                Cena / m²
              </div>
              {[
                { color: '#65a30d', label: 'do 3 500 €' },
                { color: '#d97706', label: '3 500 – 5 000 €' },
                { color: '#ea580c', label: '5 000 – 5 500 €' },
                { color: '#dc2626', label: 'nad 5 500 €' },
              ].map((item) => (
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

          {/* Right panel */}
          <div
            style={{
              background: 'var(--ppt-bg-surface)',
              borderLeft: '1px solid var(--ppt-border-default, #e5e7eb)',
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
                      value: formatPrice(selected.avg_price_per_m2),
                    },
                    {
                      label: 'Trend (medzikvartálne)',
                      value:
                        selected.trend_pct_qoq == null
                          ? '—'
                          : `${selected.trend_pct_qoq > 0 ? '+' : ''}${selected.trend_pct_qoq} %`,
                      positive: (selected.trend_pct_qoq ?? 0) > 0,
                    },
                    { label: 'Aktívne inzeráty', value: String(selected.listing_count) },
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
                            'positive' in row
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
                  Prehľad lokalít
                </h2>
                <p
                  style={{
                    fontSize: '0.875rem',
                    color: 'var(--ppt-fg-secondary)',
                    marginBottom: 20,
                  }}
                >
                  Kliknite na lokalitu pre detailné štatistiky.
                </p>
                {isLoading ? (
                  <p style={{ fontSize: '0.875rem', color: 'var(--ppt-fg-secondary)' }}>
                    Načítavam dáta…
                  </p>
                ) : isError ? (
                  <p style={{ fontSize: '0.875rem', color: 'var(--ppt-color-danger, #ef4444)' }}>
                    Dáta o cenách sa nepodarilo načítať.
                  </p>
                ) : districts.length === 0 ? (
                  <p style={{ fontSize: '0.875rem', color: 'var(--ppt-fg-secondary)' }}>
                    Pre zvolené filtre nie sú dostupné žiadne dáta.
                  </p>
                ) : (
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
                              background: priceToColor(d.avg_price_per_m2),
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
                            {d.district_name}
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
                          {formatPrice(d.avg_price_per_m2)}/m²
                        </span>
                      </button>
                    ))}
                  </div>
                )}
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
        }
      `}</style>
    </div>
  );
}
