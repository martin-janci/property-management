'use client';

/**
 * Cookie policy page — Reality Portal.
 * Screen-map: docs/screens/reality/cookies.md
 */

import { useTranslations } from 'next-intl';
import { useState } from 'react';
import { Footer, Header } from '@/components/ui';
import { COOKIE_SECTIONS, MOCK_COOKIES } from './_mock';

// TODO: replace consent dialog trigger with @ppt/ui-kit/ModalDrawer once available

const TYPE_LABELS: Record<string, string> = {
  essential: 'Nevyhnutné',
  functional: 'Funkčné',
  analytics: 'Analytické',
  marketing: 'Marketingové',
};

const TYPE_COLORS: Record<string, { bg: string; color: string }> = {
  essential: {
    bg: 'var(--ppt-color-info-light, #dbeafe)',
    color: 'var(--ppt-color-info-dark, #1e40af)',
  },
  functional: {
    bg: 'var(--ppt-color-success-light, #d1fae5)',
    color: 'var(--ppt-color-success-dark, #047857)',
  },
  analytics: {
    bg: 'var(--ppt-color-warning-light, #fef3c7)',
    color: 'var(--ppt-color-warning-dark, #b45309)',
  },
  marketing: {
    bg: 'var(--ppt-color-danger-light, #fee2e2)',
    color: 'var(--ppt-color-danger-dark, #b91c1c)',
  },
};

export default function CookiesPage() {
  const t = useTranslations('pages.cookies');
  const [consentBannerOpen, setConsentBannerOpen] = useState(false);

  return (
    <div
      data-i18n="pages.cookies.root"
      style={{
        minHeight: '100vh',
        display: 'flex',
        flexDirection: 'column',
        background: 'var(--ppt-bg-app)',
      }}
    >
      <Header />

      <main
        style={{ flex: 1, maxWidth: 900, margin: '0 auto', padding: '48px 24px', width: '100%' }}
      >
        <h1
          style={{
            fontSize: '2rem',
            fontWeight: 800,
            color: 'var(--ppt-fg-primary)',
            marginBottom: 8,
          }}
        >
          {t('title')}
        </h1>
        <p style={{ color: 'var(--ppt-fg-secondary)', marginBottom: 40 }}>{t('lastUpdated')}</p>

        {/* §§ text sections */}
        {COOKIE_SECTIONS.map((section) => (
          <section key={section.id} style={{ marginBottom: 32 }}>
            <h2
              style={{
                fontSize: '1.25rem',
                fontWeight: 700,
                color: 'var(--ppt-fg-primary)',
                marginBottom: 10,
              }}
            >
              {section.heading}
            </h2>
            <p style={{ color: 'var(--ppt-fg-secondary)', lineHeight: 1.7, margin: 0 }}>
              {section.body}
            </p>
          </section>
        ))}

        {/* Cookie table */}
        <section style={{ marginBottom: 40 }}>
          <h2
            style={{
              fontSize: '1.25rem',
              fontWeight: 700,
              color: 'var(--ppt-fg-primary)',
              marginBottom: 16,
            }}
          >
            Zoznam cookies
          </h2>
          <div style={{ overflowX: 'auto' }}>
            <table
              style={{
                width: '100%',
                borderCollapse: 'collapse',
                fontSize: '0.875rem',
                background: 'var(--ppt-bg-surface)',
                borderRadius: 10,
                overflow: 'hidden',
                boxShadow: '0 1px 4px rgba(0,0,0,.06)',
              }}
            >
              <thead>
                <tr style={{ background: 'var(--ppt-bg-subtle, #f1f5f9)', textAlign: 'left' }}>
                  {['Názov', 'Poskytovateľ', 'Účel', 'Platnosť', 'Typ'].map((h) => (
                    <th
                      key={h}
                      style={{
                        padding: '12px 16px',
                        fontWeight: 600,
                        color: 'var(--ppt-fg-primary)',
                        whiteSpace: 'nowrap',
                      }}
                    >
                      {h}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {MOCK_COOKIES.map((row, idx) => (
                  <tr
                    key={row.name}
                    style={{
                      borderTop:
                        idx > 0 ? '1px solid var(--ppt-border-default, #e5e7eb)' : undefined,
                    }}
                  >
                    <td
                      style={{
                        padding: '12px 16px',
                        fontFamily: 'monospace',
                        color: 'var(--ppt-fg-primary)',
                      }}
                    >
                      {row.name}
                    </td>
                    <td style={{ padding: '12px 16px', color: 'var(--ppt-fg-secondary)' }}>
                      {row.provider}
                    </td>
                    <td style={{ padding: '12px 16px', color: 'var(--ppt-fg-secondary)' }}>
                      {row.purpose}
                    </td>
                    <td
                      style={{
                        padding: '12px 16px',
                        color: 'var(--ppt-fg-secondary)',
                        whiteSpace: 'nowrap',
                      }}
                    >
                      {row.duration}
                    </td>
                    <td style={{ padding: '12px 16px' }}>
                      {/* TODO: replace with @ppt/ui-kit/StatusPill once available */}
                      <span
                        style={{
                          display: 'inline-block',
                          padding: '3px 8px',
                          borderRadius: 99,
                          fontSize: '0.75rem',
                          fontWeight: 500,
                          background: TYPE_COLORS[row.type]?.bg ?? '#f3f4f6',
                          color: TYPE_COLORS[row.type]?.color ?? '#374151',
                        }}
                      >
                        {TYPE_LABELS[row.type] ?? row.type}
                      </span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </section>

        {/* Re-open consent button */}
        <section
          style={{
            padding: '28px 24px',
            background: 'var(--ppt-bg-surface)',
            borderRadius: 10,
            boxShadow: '0 1px 4px rgba(0,0,0,.06)',
          }}
        >
          <h2
            style={{
              fontSize: '1.125rem',
              fontWeight: 700,
              color: 'var(--ppt-fg-primary)',
              margin: '0 0 8px',
            }}
          >
            Spravovať súhlas s cookies
          </h2>
          <p
            style={{ color: 'var(--ppt-fg-secondary)', margin: '0 0 16px', fontSize: '0.9375rem' }}
          >
            Kedykoľvek môžete zmeniť svoje nastavenia súhlasu.
          </p>
          <button
            type="button"
            onClick={() => setConsentBannerOpen(true)}
            style={{
              padding: '10px 20px',
              background: 'var(--ppt-color-primary, #2563eb)',
              color: '#fff',
              border: 'none',
              borderRadius: 8,
              fontWeight: 600,
              cursor: 'pointer',
              fontSize: '0.9375rem',
            }}
          >
            Znova otvoriť nastavenia cookies
          </button>

          {/* TODO: replace with @ppt/ui-kit/ModalDrawer once available */}
          {consentBannerOpen && (
            <div
              role="dialog"
              aria-modal="true"
              aria-label="Nastavenia cookies"
              style={{
                position: 'fixed',
                inset: 0,
                background: 'rgba(0,0,0,.5)',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                zIndex: 1000,
              }}
            >
              <div
                style={{
                  background: 'var(--ppt-bg-surface)',
                  borderRadius: 12,
                  padding: '32px',
                  maxWidth: 480,
                  width: '90%',
                }}
              >
                <h3 style={{ margin: '0 0 16px', color: 'var(--ppt-fg-primary)' }}>
                  Nastavenia cookies
                </h3>
                <p style={{ color: 'var(--ppt-fg-secondary)', marginBottom: 24 }}>
                  Funkcia úpravy súhlasu bude plne implementovaná po integrácii CMP knižnice.
                </p>
                <button
                  type="button"
                  onClick={() => setConsentBannerOpen(false)}
                  style={{
                    padding: '10px 20px',
                    background: 'var(--ppt-color-primary, #2563eb)',
                    color: '#fff',
                    border: 'none',
                    borderRadius: 8,
                    fontWeight: 600,
                    cursor: 'pointer',
                  }}
                >
                  Zavrieť
                </button>
              </div>
            </div>
          )}
        </section>
      </main>

      <Footer />
    </div>
  );
}
