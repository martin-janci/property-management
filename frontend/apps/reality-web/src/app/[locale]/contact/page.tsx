'use client';

import { useTranslations } from 'next-intl';
import { Footer, Header } from '@/components/ui';
import { CONTACT_CHANNELS, CONTACT_HERO, CONTACT_HOURS, CONTACT_OFFICE } from './_content';

export default function ContactPage() {
  const t = useTranslations('pages.contact');

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
        <section
          style={{
            background:
              'linear-gradient(135deg, var(--ppt-color-primary, #2563eb), var(--ppt-color-primary-hover, #1e3a5f))',
            color: 'var(--ppt-fg-on-accent, #fff)',
            padding: '56px 24px',
          }}
        >
          <div style={{ maxWidth: 900, margin: '0 auto' }}>
            <h1 style={{ fontSize: '2.25rem', fontWeight: 800, margin: '0 0 12px' }}>
              {t('title')}
            </h1>
            <p style={{ fontSize: '1.0625rem', lineHeight: 1.6, opacity: 0.92, margin: 0 }}>
              {CONTACT_HERO.subtitle}
            </p>
          </div>
        </section>

        <section style={{ maxWidth: 900, margin: '0 auto', padding: '48px 24px' }}>
          <div
            style={{
              display: 'grid',
              gridTemplateColumns: 'repeat(auto-fit, minmax(260px, 1fr))',
              gap: 20,
            }}
          >
            {CONTACT_CHANNELS.map((c) => (
              <a
                key={c.id}
                href={c.href}
                style={{
                  display: 'block',
                  padding: 20,
                  background: 'var(--ppt-bg-surface)',
                  border: '1px solid var(--ppt-border-default)',
                  borderRadius: 10,
                  textDecoration: 'none',
                  color: 'inherit',
                  transition: 'border-color .15s, transform .15s',
                }}
              >
                <div style={{ fontSize: 24, marginBottom: 8 }}>{c.icon}</div>
                <div
                  style={{
                    fontSize: '1.0625rem',
                    fontWeight: 700,
                    color: 'var(--ppt-fg-primary)',
                    marginBottom: 6,
                  }}
                >
                  {c.title}
                </div>
                <div
                  style={{
                    fontSize: '0.9375rem',
                    color: 'var(--ppt-fg-secondary)',
                    lineHeight: 1.5,
                    marginBottom: 12,
                  }}
                >
                  {c.description}
                </div>
                <div
                  style={{
                    fontSize: '0.9375rem',
                    fontWeight: 600,
                    color: 'var(--ppt-color-primary, #2563eb)',
                  }}
                >
                  {c.cta} →
                </div>
              </a>
            ))}
          </div>
        </section>

        <section
          style={{
            maxWidth: 900,
            margin: '0 auto',
            padding: '0 24px 64px',
            display: 'grid',
            gridTemplateColumns: 'repeat(auto-fit, minmax(280px, 1fr))',
            gap: 32,
          }}
        >
          {[CONTACT_OFFICE, CONTACT_HOURS].map((block) => (
            <div
              key={block.heading}
              style={{
                background: 'var(--ppt-bg-surface)',
                border: '1px solid var(--ppt-border-default)',
                borderRadius: 10,
                padding: 24,
              }}
            >
              <h2
                style={{
                  fontSize: '1.125rem',
                  fontWeight: 700,
                  color: 'var(--ppt-fg-primary)',
                  margin: '0 0 16px',
                }}
              >
                {block.heading}
              </h2>
              <dl style={{ margin: 0, display: 'grid', gap: 8 }}>
                {block.rows.map((row) => (
                  <div
                    key={row.label}
                    style={{
                      display: 'grid',
                      gridTemplateColumns: '120px 1fr',
                      gap: 12,
                      fontSize: '0.9375rem',
                    }}
                  >
                    <dt style={{ color: 'var(--ppt-fg-muted)' }}>{row.label}</dt>
                    <dd style={{ margin: 0, color: 'var(--ppt-fg-primary)' }}>{row.value}</dd>
                  </div>
                ))}
              </dl>
            </div>
          ))}
        </section>
      </main>
      <Footer />
    </div>
  );
}
