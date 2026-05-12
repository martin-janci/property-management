'use client';

/**
 * Help centre page — Reality Portal.
 * Screen-map: docs/screens/reality/help.md
 */

import { useTranslations } from 'next-intl';
import { useState } from 'react';
import { Footer, Header } from '@/components/ui';
import { MOCK_CATEGORIES, MOCK_FAQ } from './_mock';

// TODO: replace FAQ accordion with @ppt/ui-kit/Accordion once available
// TODO: replace category tiles with @ppt/ui-kit/Card once available

export default function HelpPage() {
  const t = useTranslations('pages.help');
  const [query, setQuery] = useState('');
  const [openFaq, setOpenFaq] = useState<string | null>(null);

  const filteredFaq = MOCK_FAQ.filter(
    (item) =>
      query.trim() === '' ||
      item.question.toLowerCase().includes(query.toLowerCase()) ||
      item.answer.toLowerCase().includes(query.toLowerCase())
  );

  return (
    <div
      data-i18n="pages.help.root"
      style={{
        minHeight: '100vh',
        display: 'flex',
        flexDirection: 'column',
        background: 'var(--ppt-bg-app)',
      }}
    >
      <Header />

      <main style={{ flex: 1 }}>
        {/* Hero / search */}
        <section
          style={{
            background:
              'linear-gradient(135deg, var(--ppt-brand-700, #1d4ed8) 0%, var(--ppt-brand-500, #3b82f6) 100%)',
            padding: '56px 24px',
            textAlign: 'center',
          }}
        >
          <h1
            style={{
              fontSize: 'clamp(1.5rem, 3vw, 2.25rem)',
              fontWeight: 800,
              color: '#fff',
              marginBottom: 8,
            }}
          >
            {t('subtitle')}
          </h1>
          <p style={{ color: 'rgba(255,255,255,.8)', marginBottom: 28 }}>{t('intro')}</p>
          <div style={{ maxWidth: 560, margin: '0 auto', position: 'relative' }}>
            <input
              type="search"
              placeholder={t('searchPlaceholder')}
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              data-i18n="pages.help.searchPlaceholder"
              style={{
                width: '100%',
                padding: '14px 48px 14px 20px',
                borderRadius: 10,
                border: 'none',
                fontSize: '1rem',
                boxSizing: 'border-box',
                boxShadow: '0 4px 16px rgba(0,0,0,.15)',
                outline: 'none',
              }}
            />
            <span
              style={{
                position: 'absolute',
                right: 16,
                top: '50%',
                transform: 'translateY(-50%)',
                fontSize: '1.25rem',
              }}
            >
              🔍
            </span>
          </div>
        </section>

        {/* Category tiles */}
        <section style={{ maxWidth: 1100, margin: '0 auto', padding: '56px 24px' }}>
          <h2
            style={{
              fontSize: '1.375rem',
              fontWeight: 700,
              color: 'var(--ppt-fg-primary)',
              marginBottom: 24,
            }}
          >
            Kategórie
          </h2>
          {/* TODO: replace with @ppt/ui-kit/Card once available */}
          <div
            style={{
              display: 'grid',
              gridTemplateColumns: 'repeat(auto-fill, minmax(220px, 1fr))',
              gap: 20,
            }}
          >
            {MOCK_CATEGORIES.map((cat) => (
              <button
                key={cat.id}
                type="button"
                onClick={() => setQuery(cat.title.toLowerCase())}
                style={{
                  background: 'var(--ppt-bg-surface)',
                  borderRadius: 12,
                  padding: '24px 20px',
                  boxShadow: '0 1px 4px rgba(0,0,0,.07)',
                  border: '1px solid var(--ppt-border-default, #e5e7eb)',
                  textAlign: 'left',
                  cursor: 'pointer',
                  transition: 'box-shadow .15s',
                }}
              >
                <div style={{ fontSize: '2rem', marginBottom: 10 }}>{cat.icon}</div>
                <div style={{ fontWeight: 700, color: 'var(--ppt-fg-primary)', marginBottom: 6 }}>
                  {cat.title}
                </div>
                <div
                  style={{
                    fontSize: '0.875rem',
                    color: 'var(--ppt-fg-secondary)',
                    marginBottom: 8,
                  }}
                >
                  {cat.description}
                </div>
                <div style={{ fontSize: '0.8125rem', color: 'var(--ppt-fg-muted, #9ca3af)' }}>
                  {cat.articleCount} článkov
                </div>
              </button>
            ))}
          </div>
        </section>

        {/* FAQ accordion */}
        <section style={{ maxWidth: 800, margin: '0 auto', padding: '0 24px 64px' }}>
          <h2
            style={{
              fontSize: '1.375rem',
              fontWeight: 700,
              color: 'var(--ppt-fg-primary)',
              marginBottom: 24,
            }}
          >
            Často kladené otázky
          </h2>
          {/* TODO: replace with @ppt/ui-kit/Accordion once available */}
          {filteredFaq.length === 0 && (
            <p style={{ color: 'var(--ppt-fg-secondary)' }}>
              Žiadne výsledky pre &quot;{query}&quot;.
            </p>
          )}
          <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
            {filteredFaq.map((item) => (
              <div
                key={item.id}
                style={{
                  background: 'var(--ppt-bg-surface)',
                  borderRadius: 10,
                  border: '1px solid var(--ppt-border-default, #e5e7eb)',
                  overflow: 'hidden',
                }}
              >
                <button
                  type="button"
                  onClick={() => setOpenFaq(openFaq === item.id ? null : item.id)}
                  aria-expanded={openFaq === item.id}
                  style={{
                    width: '100%',
                    padding: '16px 20px',
                    textAlign: 'left',
                    background: 'none',
                    border: 'none',
                    cursor: 'pointer',
                    display: 'flex',
                    justifyContent: 'space-between',
                    alignItems: 'center',
                    gap: 12,
                    fontWeight: 600,
                    color: 'var(--ppt-fg-primary)',
                    fontSize: '0.9375rem',
                  }}
                >
                  {item.question}
                  <span
                    style={{
                      flexShrink: 0,
                      transition: 'transform .2s',
                      transform: openFaq === item.id ? 'rotate(180deg)' : 'none',
                    }}
                  >
                    ▾
                  </span>
                </button>
                {openFaq === item.id && (
                  <div
                    style={{
                      padding: '0 20px 16px',
                      color: 'var(--ppt-fg-secondary)',
                      lineHeight: 1.7,
                      fontSize: '0.9375rem',
                    }}
                  >
                    {item.answer}
                  </div>
                )}
              </div>
            ))}
          </div>
        </section>
      </main>

      <Footer />
    </div>
  );
}
