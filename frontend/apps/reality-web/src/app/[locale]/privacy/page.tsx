'use client';

import { useTranslations } from 'next-intl';
import { Footer, Header } from '@/components/ui';
import { PRIVACY_META, PRIVACY_SECTIONS } from './_content';

export default function PrivacyPage() {
  const t = useTranslations('pages.privacy');

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
      <main
        style={{
          flex: 1,
          width: '100%',
          maxWidth: 800,
          margin: '0 auto',
          padding: '48px 24px',
          boxSizing: 'border-box',
        }}
      >
        <h1
          style={{
            fontSize: '2rem',
            fontWeight: 800,
            color: 'var(--ppt-fg-primary)',
            margin: '0 0 8px',
          }}
        >
          {t('title')}
        </h1>
        <p
          style={{
            fontSize: '0.875rem',
            color: 'var(--ppt-fg-muted)',
            margin: '0 0 32px',
          }}
        >
          Účinné od: {PRIVACY_META.effective} · Naposledy aktualizované: {PRIVACY_META.lastUpdated}
        </p>

        <nav
          aria-label="Obsah"
          style={{
            background: 'var(--ppt-bg-surface)',
            border: '1px solid var(--ppt-border-default)',
            borderRadius: 8,
            padding: 16,
            marginBottom: 32,
          }}
        >
          <div
            style={{
              fontSize: '0.875rem',
              fontWeight: 600,
              color: 'var(--ppt-fg-primary)',
              marginBottom: 8,
            }}
          >
            Obsah
          </div>
          <ol
            style={{
              margin: 0,
              padding: 0,
              listStyle: 'none',
              display: 'grid',
              gap: 4,
              fontSize: '0.9375rem',
            }}
          >
            {PRIVACY_SECTIONS.map((s) => (
              <li key={s.id}>
                <a
                  href={`#${s.id}`}
                  style={{ color: 'var(--ppt-color-primary, #2563eb)', textDecoration: 'none' }}
                >
                  {s.heading}
                </a>
              </li>
            ))}
          </ol>
        </nav>

        {PRIVACY_SECTIONS.map((section) => (
          <section
            key={section.id}
            id={section.id}
            style={{ marginBottom: 32, scrollMarginTop: 24 }}
          >
            <h2
              style={{
                fontSize: '1.25rem',
                fontWeight: 700,
                color: 'var(--ppt-fg-primary)',
                margin: '0 0 12px',
              }}
            >
              {section.heading}
            </h2>
            {section.paragraphs?.map((para, idx) => (
              <p
                key={`${section.id}-p-${idx}`}
                style={{
                  color: 'var(--ppt-fg-secondary)',
                  lineHeight: 1.7,
                  margin: '0 0 12px',
                }}
              >
                {para}
              </p>
            ))}
            {section.bullets && (
              <ul style={{ margin: '8px 0 0 20px', padding: 0, color: 'var(--ppt-fg-secondary)' }}>
                {section.bullets.map((b, idx) => (
                  <li key={`${section.id}-b-${idx}`} style={{ marginBottom: 6, lineHeight: 1.6 }}>
                    {b}
                  </li>
                ))}
              </ul>
            )}
          </section>
        ))}

        <p
          style={{
            fontSize: '0.8125rem',
            color: 'var(--ppt-fg-muted)',
            fontStyle: 'italic',
            marginTop: 32,
            paddingTop: 16,
            borderTop: '1px solid var(--ppt-border-default)',
          }}
        >
          Pozn.: Tento dokument predstavuje pracovnú verziu pripravenú pre dizajnovú integráciu a
          pred uvedením do produkcie podlieha posúdeniu zodpovednou osobou (DPO) a právnym
          oddelením.
        </p>
      </main>
      <Footer />
    </div>
  );
}
