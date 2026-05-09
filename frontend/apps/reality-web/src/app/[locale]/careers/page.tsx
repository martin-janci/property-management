'use client';

/**
 * Careers page — Reality Portal job listings.
 * Screen-map: docs/screens/reality/careers.md
 */

import { Footer, Header } from '@/components/ui';
import { MOCK_BENEFITS, MOCK_POSITIONS } from './_mock';

// TODO: replace with @ppt/ui-kit/Stepper, @ppt/ui-kit/StatusPill once available

export default function CareersPage() {
  return (
    <div
      data-i18n="pages.careers.root"
      style={{ minHeight: '100vh', display: 'flex', flexDirection: 'column', background: 'var(--ppt-bg-app)' }}
    >
      <Header />

      <main style={{ flex: 1 }}>
        {/* Hero */}
        <section
          style={{
            background: 'linear-gradient(135deg, var(--ppt-brand-800, #1e3a5f) 0%, var(--ppt-brand-600, #2563eb) 100%)',
            color: '#fff',
            padding: '80px 24px',
            textAlign: 'center',
          }}
        >
          <h1
            data-i18n="pages.careers.hero.title"
            style={{ fontSize: 'clamp(1.75rem, 4vw, 3rem)', fontWeight: 800, lineHeight: 1.2, margin: '0 0 20px' }}
          >
            Stavajme realitný trh, ktorý dáva zmysel.
          </h1>
          <p style={{ fontSize: '1.125rem', opacity: 0.85, maxWidth: 640, margin: '0 auto 32px' }}>
            Pridajte sa k nášmu tímu a pomôžte nám meniť spôsob, akým ľudia nakupujú, predávajú a prenajímajú
            nehnuteľnosti na Slovensku a v strednej Európe.
          </p>
          <a
            href="#open-positions"
            style={{
              display: 'inline-block',
              padding: '14px 32px',
              background: '#fff',
              color: 'var(--ppt-brand-700, #1d4ed8)',
              borderRadius: 8,
              fontWeight: 700,
              textDecoration: 'none',
              fontSize: '1rem',
            }}
          >
            Zobraziť voľné pozície
          </a>
        </section>

        {/* Benefits grid */}
        <section style={{ maxWidth: 1200, margin: '0 auto', padding: '64px 24px' }}>
          <h2 style={{ fontSize: '1.75rem', fontWeight: 700, color: 'var(--ppt-fg-primary)', marginBottom: 40, textAlign: 'center' }}>
            Prečo Reality Portál?
          </h2>
          <div
            style={{
              display: 'grid',
              gridTemplateColumns: 'repeat(auto-fill, minmax(280px, 1fr))',
              gap: 24,
            }}
          >
            {MOCK_BENEFITS.map((benefit) => (
              <div
                key={benefit.title}
                style={{
                  background: 'var(--ppt-bg-surface)',
                  borderRadius: 12,
                  padding: '28px 24px',
                  boxShadow: '0 1px 4px rgba(0,0,0,.08)',
                }}
              >
                <div style={{ fontSize: '2rem', marginBottom: 12 }}>{benefit.icon}</div>
                <h3 style={{ fontSize: '1.125rem', fontWeight: 600, color: 'var(--ppt-fg-primary)', margin: '0 0 8px' }}>
                  {benefit.title}
                </h3>
                <p style={{ fontSize: '0.9rem', color: 'var(--ppt-fg-secondary)', lineHeight: 1.6, margin: 0 }}>
                  {benefit.description}
                </p>
              </div>
            ))}
          </div>
        </section>

        {/* Open positions */}
        <section id="open-positions" style={{ background: 'var(--ppt-bg-subtle, #f8fafc)', padding: '64px 24px' }}>
          <div style={{ maxWidth: 900, margin: '0 auto' }}>
            <h2 style={{ fontSize: '1.75rem', fontWeight: 700, color: 'var(--ppt-fg-primary)', marginBottom: 32 }}>
              Voľné pozície
            </h2>
            <ul style={{ listStyle: 'none', padding: 0, margin: 0, display: 'flex', flexDirection: 'column', gap: 12 }}>
              {MOCK_POSITIONS.map((pos) => (
                <li
                  key={pos.id}
                  style={{
                    background: 'var(--ppt-bg-surface)',
                    borderRadius: 10,
                    padding: '20px 24px',
                    display: 'flex',
                    justifyContent: 'space-between',
                    alignItems: 'center',
                    boxShadow: '0 1px 3px rgba(0,0,0,.06)',
                    gap: 16,
                    flexWrap: 'wrap',
                  }}
                >
                  <div>
                    <div style={{ fontWeight: 600, fontSize: '1.0625rem', color: 'var(--ppt-fg-primary)', marginBottom: 4 }}>
                      {pos.title}
                    </div>
                    <div style={{ fontSize: '0.875rem', color: 'var(--ppt-fg-secondary)' }}>
                      {pos.team} · {pos.location}
                    </div>
                  </div>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 12, flexShrink: 0 }}>
                    {/* TODO: replace with @ppt/ui-kit/StatusPill once available */}
                    <span
                      style={{
                        fontSize: '0.8125rem',
                        padding: '4px 10px',
                        borderRadius: 99,
                        background: 'var(--ppt-color-success-light, #d1fae5)',
                        color: 'var(--ppt-color-success-dark, #047857)',
                        fontWeight: 500,
                      }}
                    >
                      {pos.type === 'fulltime' ? 'Plný úväzok' : pos.type === 'parttime' ? 'Čiastočný úväzok' : 'Kontrakt'}
                    </span>
                    <button
                      type="button"
                      style={{
                        padding: '8px 18px',
                        background: 'var(--ppt-color-primary, #2563eb)',
                        color: '#fff',
                        border: 'none',
                        borderRadius: 6,
                        cursor: 'pointer',
                        fontSize: '0.875rem',
                        fontWeight: 600,
                      }}
                    >
                      Uchádzať sa
                    </button>
                  </div>
                </li>
              ))}
            </ul>
          </div>
        </section>

        {/* CTA band */}
        <section style={{ maxWidth: 900, margin: '0 auto', padding: '64px 24px', textAlign: 'center' }}>
          <h2 style={{ fontSize: '1.5rem', fontWeight: 700, color: 'var(--ppt-fg-primary)', marginBottom: 12 }}>
            Nenašli ste vhodnú pozíciu?
          </h2>
          <p style={{ color: 'var(--ppt-fg-secondary)', marginBottom: 24 }}>
            Pošlite nám otvorený životopis – radi sa s vami porozprávame.
          </p>
          <a
            href="mailto:kariera@rlt.sk"
            style={{
              display: 'inline-block',
              padding: '12px 28px',
              background: 'var(--ppt-color-primary, #2563eb)',
              color: '#fff',
              borderRadius: 8,
              fontWeight: 600,
              textDecoration: 'none',
            }}
          >
            Poslať životopis
          </a>
        </section>
      </main>

      <Footer />
    </div>
  );
}
