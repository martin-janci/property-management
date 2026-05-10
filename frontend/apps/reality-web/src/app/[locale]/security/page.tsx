'use client';

/**
 * Security / fraud-awareness page — Reality Portal.
 * Screen-map: docs/screens/reality/security.md
 */

import Link from 'next/link';
import { Footer, Header } from '@/components/ui';
import { MOCK_FRAUD_SCENARIOS, MOCK_GUARANTEES } from './_mock';

export default function SecurityPage() {
  return (
    <div
      data-i18n="pages.security.root"
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
            background: 'var(--ppt-bg-surface)',
            padding: '56px 24px',
            borderBottom: '1px solid var(--ppt-border-default, #e5e7eb)',
          }}
        >
          <div style={{ maxWidth: 800, margin: '0 auto', textAlign: 'center' }}>
            <div style={{ fontSize: '3rem', marginBottom: 16 }}>🔐</div>
            <h1
              style={{
                fontSize: 'clamp(1.75rem, 3vw, 2.5rem)',
                fontWeight: 800,
                color: 'var(--ppt-fg-primary)',
                marginBottom: 16,
              }}
            >
              Vaša bezpečnosť je naša priorita
            </h1>
            <p
              style={{
                fontSize: '1.0625rem',
                color: 'var(--ppt-fg-secondary)',
                lineHeight: 1.7,
                maxWidth: 640,
                margin: '0 auto',
              }}
            >
              Pomáhame vám rozpoznať podvody a obchodovať s nehnuteľnosťami v bezpečnom prostredí.
            </p>
          </div>
        </section>

        {/* Fraud scenarios */}
        <section style={{ maxWidth: 960, margin: '0 auto', padding: '64px 24px' }}>
          <h2
            style={{
              fontSize: '1.5rem',
              fontWeight: 700,
              color: 'var(--ppt-fg-primary)',
              marginBottom: 32,
            }}
          >
            Časté podvodné scenáre
          </h2>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 20 }}>
            {MOCK_FRAUD_SCENARIOS.map((scenario) => (
              <div
                key={scenario.id}
                style={{
                  background: 'var(--ppt-bg-surface)',
                  borderRadius: 12,
                  padding: '24px',
                  boxShadow: '0 1px 4px rgba(0,0,0,.06)',
                  borderLeft: '4px solid var(--ppt-color-danger, #ef4444)',
                }}
              >
                <div style={{ display: 'flex', gap: 16, alignItems: 'flex-start' }}>
                  <span style={{ fontSize: '1.75rem', lineHeight: 1, flexShrink: 0 }}>
                    {scenario.icon}
                  </span>
                  <div>
                    <h3
                      style={{
                        fontSize: '1.0625rem',
                        fontWeight: 700,
                        color: 'var(--ppt-fg-primary)',
                        margin: '0 0 8px',
                      }}
                    >
                      {scenario.title}
                    </h3>
                    <p
                      style={{
                        color: 'var(--ppt-fg-secondary)',
                        lineHeight: 1.65,
                        margin: '0 0 12px',
                      }}
                    >
                      {scenario.description}
                    </p>
                    <div
                      style={{
                        background: 'var(--ppt-color-warning-light, #fef3c7)',
                        color: 'var(--ppt-color-warning-dark, #b45309)',
                        borderRadius: 6,
                        padding: '8px 12px',
                        fontSize: '0.875rem',
                        fontWeight: 500,
                      }}
                    >
                      ⚠️ {scenario.warning}
                    </div>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </section>

        {/* Guarantees */}
        <section style={{ background: 'var(--ppt-bg-subtle, #f8fafc)', padding: '64px 24px' }}>
          <div style={{ maxWidth: 960, margin: '0 auto' }}>
            <h2
              style={{
                fontSize: '1.5rem',
                fontWeight: 700,
                color: 'var(--ppt-fg-primary)',
                marginBottom: 32,
                textAlign: 'center',
              }}
            >
              Naše záruky bezpečnosti
            </h2>
            <div
              style={{
                display: 'grid',
                gridTemplateColumns: 'repeat(auto-fill, minmax(200px, 1fr))',
                gap: 20,
              }}
            >
              {MOCK_GUARANTEES.map((guarantee) => (
                <div
                  key={guarantee.id}
                  style={{
                    background: 'var(--ppt-bg-surface)',
                    borderRadius: 12,
                    padding: '24px 20px',
                    boxShadow: '0 1px 4px rgba(0,0,0,.06)',
                    textAlign: 'center',
                  }}
                >
                  <div style={{ fontSize: '2rem', marginBottom: 12 }}>{guarantee.icon}</div>
                  <h3
                    style={{
                      fontSize: '1rem',
                      fontWeight: 700,
                      color: 'var(--ppt-fg-primary)',
                      margin: '0 0 8px',
                    }}
                  >
                    {guarantee.title}
                  </h3>
                  <p
                    style={{
                      fontSize: '0.875rem',
                      color: 'var(--ppt-fg-secondary)',
                      lineHeight: 1.6,
                      margin: 0,
                    }}
                  >
                    {guarantee.description}
                  </p>
                </div>
              ))}
            </div>
          </div>
        </section>

        {/* Report listing CTA */}
        <section
          style={{ maxWidth: 960, margin: '0 auto', padding: '64px 24px', textAlign: 'center' }}
        >
          <h2
            style={{
              fontSize: '1.5rem',
              fontWeight: 700,
              color: 'var(--ppt-fg-primary)',
              marginBottom: 12,
            }}
          >
            Narazili ste na podozrivú ponuku?
          </h2>
          <p
            style={{
              color: 'var(--ppt-fg-secondary)',
              marginBottom: 28,
              maxWidth: 560,
              margin: '0 auto 28px',
            }}
          >
            Nahláste ju okamžite. Náš tím to preverí do 24 hodín a ak je podozrenie opodstatnené,
            ponuku odstránime.
          </p>
          <Link
            href="/report"
            style={{
              display: 'inline-block',
              padding: '14px 32px',
              background: 'var(--ppt-color-danger, #ef4444)',
              color: '#fff',
              borderRadius: 8,
              fontWeight: 700,
              textDecoration: 'none',
              fontSize: '1rem',
            }}
          >
            Nahlásiť ponuku
          </Link>
        </section>
      </main>

      <Footer />
    </div>
  );
}
