'use client';

import { useTranslations } from 'next-intl';
import { Footer, Header } from '@/components/ui';
import { ABOUT_HERO, ABOUT_SECTIONS, ABOUT_STATS } from './_content';

export default function AboutPage() {
  const t = useTranslations('pages.about');

  return (
    <div
      style={{
        minHeight: '100vh',
        display: 'flex',
        flexDirection: 'column',
        background: 'var(--ppt-bg-app)',
      }}
    >
      <Header />
      <main style={{ flex: 1 }}>
        {/* Hero */}
        <section
          style={{
            background:
              'linear-gradient(135deg, var(--ppt-color-primary, #2563eb), var(--ppt-color-primary-hover, #1e3a5f))',
            color: 'var(--ppt-fg-on-accent, #fff)',
            padding: '64px 24px',
          }}
        >
          <div style={{ maxWidth: 900, margin: '0 auto' }}>
            <h1 style={{ fontSize: '2.25rem', fontWeight: 800, margin: '0 0 16px' }}>
              {t('title')}
            </h1>
            <p style={{ fontSize: '1.0625rem', lineHeight: 1.6, opacity: 0.92, margin: 0 }}>
              {ABOUT_HERO.subtitle}
            </p>
          </div>
        </section>

        {/* Stats strip */}
        <section
          style={{
            background: 'var(--ppt-bg-surface)',
            borderBottom: '1px solid var(--ppt-border-default)',
          }}
        >
          <div
            style={{
              maxWidth: 900,
              margin: '0 auto',
              padding: '32px 24px',
              display: 'grid',
              gridTemplateColumns: 'repeat(auto-fit, minmax(160px, 1fr))',
              gap: 24,
            }}
          >
            {ABOUT_STATS.map((s) => (
              <div key={s.label}>
                <div
                  style={{
                    fontSize: '1.75rem',
                    fontWeight: 700,
                    color: 'var(--ppt-color-primary, #2563eb)',
                  }}
                >
                  {s.value}
                </div>
                <div
                  style={{ fontSize: '0.875rem', color: 'var(--ppt-fg-secondary)', marginTop: 4 }}
                >
                  {s.label}
                </div>
              </div>
            ))}
          </div>
        </section>

        {/* Body sections */}
        <div style={{ maxWidth: 800, margin: '0 auto', padding: '48px 24px' }}>
          {ABOUT_SECTIONS.map((section) => (
            <section key={section.id} style={{ marginBottom: 40 }}>
              <h2
                style={{
                  fontSize: '1.5rem',
                  fontWeight: 700,
                  color: 'var(--ppt-fg-primary)',
                  margin: '0 0 16px',
                }}
              >
                {section.heading}
              </h2>
              {section.paragraphs?.map((para, idx) => (
                <p
                  key={`${section.id}-p-${idx}`}
                  style={{ color: 'var(--ppt-fg-secondary)', lineHeight: 1.7, margin: '0 0 12px' }}
                >
                  {para}
                </p>
              ))}
              {section.items && (
                <div style={{ display: 'grid', gap: 16, marginTop: 8 }}>
                  {section.items.map((it) => (
                    <div
                      key={it.label}
                      style={{
                        padding: 16,
                        background: 'var(--ppt-bg-surface)',
                        border: '1px solid var(--ppt-border-default)',
                        borderRadius: 8,
                      }}
                    >
                      <div
                        style={{
                          fontWeight: 600,
                          color: 'var(--ppt-fg-primary)',
                          marginBottom: 4,
                        }}
                      >
                        {it.label}
                      </div>
                      <div style={{ fontSize: '0.9375rem', color: 'var(--ppt-fg-secondary)' }}>
                        {it.description}
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </section>
          ))}
        </div>
      </main>
      <Footer />
    </div>
  );
}
