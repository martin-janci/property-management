'use client';

/**
 * Price map page — choropleth of Bratislava district prices.
 * Screen-map: docs/screens/reality/price-map.md
 */

import { useTranslations } from 'next-intl';
import { useState } from 'react';
import { Footer, Header } from '@/components/ui';
import { MOCK_DISTRICTS, MOCK_INSIGHTS, type PriceMapFilter } from './_mock';

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

export default function PriceMapPage() {
  const t = useTranslations('pages.priceMap');
  const [filter, setFilter] = useState<PriceMapFilter>({
    propertyType: 'all',
    transactionType: 'sale',
  });
  const [selectedId, setSelectedId] = useState<string | null>(null);

  const selected = MOCK_DISTRICTS.find((d) => d.id === selectedId) ?? null;

  const formatPrice = (v: number) => `${v.toLocaleString('sk-SK')} €`;

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
            {(['sale', 'rent'] as TransactionType[]).map((t) => (
              <button
                key={t}
                type="button"
                onClick={() => setFilter((f) => ({ ...f, transactionType: t }))}
                style={{
                  padding: '7px 16px',
                  borderRadius: 6,
                  border: 'none',
                  background:
                    filter.transactionType === t
                      ? 'var(--ppt-color-primary, #2563eb)'
                      : 'var(--ppt-bg-app, #f1f5f9)',
                  color: filter.transactionType === t ? '#fff' : 'var(--ppt-fg-secondary)',
                  fontWeight: 600,
                  cursor: 'pointer',
                  fontSize: '0.875rem',
                }}
              >
                {t === 'sale' ? 'Predaj' : 'Prenájom'}
              </button>
            ))}
          </div>
          {/* TODO: replace with @ppt/ui-kit/ChipGroup once available */}
          <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
            {PROPERTY_OPTIONS.map((opt) => (
              <button
                key={opt.value}
                type="button"
                onClick={() => setFilter((f) => ({ ...f, propertyType: opt.value }))}
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
            <svg
              viewBox="0 80 400 200"
              style={{ width: '100%', maxWidth: 600, height: 'auto' }}
              aria-label="Mapa bratislavských mestských štvrtí"
            >
              {MOCK_DISTRICTS.map((district) => (
                <path
                  key={district.id}
                  d={district.svgPath}
                  fill={district.color}
                  opacity={selectedId === district.id ? 1 : 0.7}
                  stroke="#fff"
                  strokeWidth={2}
                  style={{ cursor: 'pointer', transition: 'opacity .15s' }}
                  onClick={() => setSelectedId(selectedId === district.id ? null : district.id)}
                  aria-label={district.name}
                >
                  {/* Single template-string child avoids the SSR/CSR
                      whitespace-handling mismatch that triggered Next.js's
                      "hydration error" overlay on first paint. */}
                  <title>{`${district.name}: ${formatPrice(district.avgPricePerSqm)} / m²`}</title>
                </path>
              ))}
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
                  {selected.name}
                </h2>
                <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
                  {[
                    { label: 'Priem. cena / m²', value: `${formatPrice(selected.avgPricePerSqm)}` },
                    {
                      label: 'Zmena (12 mesiacov)',
                      value: `${selected.change12m > 0 ? '+' : ''}${selected.change12m} %`,
                      positive: selected.change12m > 0,
                    },
                    { label: 'Medián celkovej ceny', value: formatPrice(selected.medianTotal) },
                    { label: 'Aktívne inzeráty', value: String(selected.listings) },
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
                  href={`/listings?city=${encodeURIComponent(selected.name)}`}
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
                  Zobraziť inzeráty v tejto štvrti
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
                  Bratislava – Prehľad
                </h2>
                <p
                  style={{
                    fontSize: '0.875rem',
                    color: 'var(--ppt-fg-secondary)',
                    marginBottom: 20,
                  }}
                >
                  Kliknite na mestskú štvrť pre detailné štatistiky.
                </p>
                <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
                  {MOCK_DISTRICTS.map((d) => (
                    <button
                      key={d.id}
                      type="button"
                      onClick={() => setSelectedId(d.id)}
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
                            background: d.color,
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
                          {d.name.split(' – ')[1] ?? d.name}
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
                        {formatPrice(d.avgPricePerSqm)}/m²
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
          {MOCK_INSIGHTS.map((insight) => (
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
