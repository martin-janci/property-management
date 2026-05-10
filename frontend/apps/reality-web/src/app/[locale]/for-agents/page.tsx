'use client';

/**
 * For-agents landing page — Reality Portal.
 * Screen-map: docs/screens/reality/for-agents.md
 */

import { Footer, Header } from '@/components/ui';
import { MOCK_FEATURES, MOCK_PRICING } from './_mock';

// TODO: replace pricing cards with @ppt/ui-kit/Card once available

export default function ForAgentsPage() {
  return (
    <div
      data-i18n="pages.for-agents.root"
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
            background: 'linear-gradient(135deg, #0f172a 0%, #1e3a5f 100%)',
            padding: '80px 24px',
            textAlign: 'center',
          }}
        >
          <span
            style={{
              display: 'inline-block',
              padding: '5px 14px',
              borderRadius: 99,
              background: 'rgba(59,130,246,.2)',
              color: '#93c5fd',
              fontSize: '0.8125rem',
              fontWeight: 600,
              marginBottom: 20,
              textTransform: 'uppercase',
              letterSpacing: '.6px',
            }}
          >
            Pre maklérov &amp; agentúry
          </span>
          <h1
            style={{
              fontSize: 'clamp(1.75rem, 4vw, 3rem)',
              fontWeight: 800,
              color: '#fff',
              lineHeight: 1.2,
              maxWidth: 720,
              margin: '0 auto 20px',
            }}
          >
            Predávajte viac s Reality Portálom
          </h1>
          <p
            style={{
              fontSize: '1.125rem',
              color: 'rgba(255,255,255,.7)',
              maxWidth: 560,
              margin: '0 auto 36px',
            }}
          >
            Nástroje, analytika a dosah na milióny kupujúcich a nájomníkov na jednom mieste.
          </p>
          <div style={{ display: 'flex', gap: 14, justifyContent: 'center', flexWrap: 'wrap' }}>
            <a
              href="#pricing"
              style={{
                padding: '14px 32px',
                background: 'var(--ppt-color-primary, #2563eb)',
                color: '#fff',
                borderRadius: 8,
                fontWeight: 700,
                textDecoration: 'none',
                fontSize: '1rem',
              }}
            >
              Začnite zadarmo
            </a>
            <a
              href="#features"
              style={{
                padding: '14px 32px',
                background: 'rgba(255,255,255,.1)',
                color: '#fff',
                borderRadius: 8,
                fontWeight: 600,
                textDecoration: 'none',
                fontSize: '1rem',
                border: '1px solid rgba(255,255,255,.2)',
              }}
            >
              Zistiť viac
            </a>
          </div>
        </section>

        {/* Features grid */}
        <section id="features" style={{ maxWidth: 1100, margin: '0 auto', padding: '80px 24px' }}>
          <h2
            style={{
              fontSize: '1.75rem',
              fontWeight: 700,
              color: 'var(--ppt-fg-primary)',
              textAlign: 'center',
              marginBottom: 48,
            }}
          >
            Čo získate
          </h2>
          <div
            style={{
              display: 'grid',
              gridTemplateColumns: 'repeat(auto-fill, minmax(280px, 1fr))',
              gap: 24,
            }}
          >
            {MOCK_FEATURES.map((feature) => (
              <div
                key={feature.id}
                style={{
                  background: 'var(--ppt-bg-surface)',
                  borderRadius: 12,
                  padding: '28px 24px',
                  boxShadow: '0 1px 4px rgba(0,0,0,.07)',
                }}
              >
                <div style={{ fontSize: '2rem', marginBottom: 14 }}>{feature.icon}</div>
                <h3
                  style={{
                    fontSize: '1.0625rem',
                    fontWeight: 700,
                    color: 'var(--ppt-fg-primary)',
                    margin: '0 0 8px',
                  }}
                >
                  {feature.title}
                </h3>
                <p
                  style={{
                    fontSize: '0.9rem',
                    color: 'var(--ppt-fg-secondary)',
                    lineHeight: 1.65,
                    margin: 0,
                  }}
                >
                  {feature.description}
                </p>
              </div>
            ))}
          </div>
        </section>

        {/* Pricing */}
        <section
          id="pricing"
          style={{ background: 'var(--ppt-bg-subtle, #f8fafc)', padding: '80px 24px' }}
        >
          <div style={{ maxWidth: 1000, margin: '0 auto' }}>
            <h2
              style={{
                fontSize: '1.75rem',
                fontWeight: 700,
                color: 'var(--ppt-fg-primary)',
                textAlign: 'center',
                marginBottom: 8,
              }}
            >
              Cenové plány
            </h2>
            <p style={{ color: 'var(--ppt-fg-secondary)', textAlign: 'center', marginBottom: 48 }}>
              30 dní zadarmo · Bez záväzkov · Zrušte kedykoľvek
            </p>
            {/* TODO: replace with @ppt/ui-kit/Card once available */}
            <div
              style={{
                display: 'grid',
                gridTemplateColumns: 'repeat(auto-fit, minmax(260px, 1fr))',
                gap: 24,
                alignItems: 'start',
              }}
            >
              {MOCK_PRICING.map((plan) => (
                <div
                  key={plan.id}
                  style={{
                    background: plan.highlighted
                      ? 'var(--ppt-color-primary, #2563eb)'
                      : 'var(--ppt-bg-surface)',
                    borderRadius: 14,
                    padding: '32px 24px',
                    boxShadow: plan.highlighted
                      ? '0 8px 24px rgba(37,99,235,.35)'
                      : '0 1px 4px rgba(0,0,0,.08)',
                    position: 'relative',
                    transform: plan.highlighted ? 'scale(1.03)' : 'none',
                    color: plan.highlighted ? '#fff' : 'inherit',
                  }}
                >
                  {plan.highlighted && (
                    <span
                      style={{
                        position: 'absolute',
                        top: -12,
                        left: '50%',
                        transform: 'translateX(-50%)',
                        background: '#facc15',
                        color: '#713f12',
                        padding: '4px 14px',
                        borderRadius: 99,
                        fontSize: '0.75rem',
                        fontWeight: 700,
                        whiteSpace: 'nowrap',
                      }}
                    >
                      Najpopulárnejší
                    </span>
                  )}
                  <div style={{ fontSize: '1.125rem', fontWeight: 700, marginBottom: 4 }}>
                    {plan.name}
                  </div>
                  <div
                    style={{
                      fontSize: '2.5rem',
                      fontWeight: 800,
                      lineHeight: 1.1,
                      marginBottom: 4,
                    }}
                  >
                    {plan.currency}
                    {plan.price}
                  </div>
                  <div style={{ fontSize: '0.875rem', opacity: 0.7, marginBottom: 24 }}>
                    / {plan.period}
                  </div>
                  <ul
                    style={{
                      listStyle: 'none',
                      padding: 0,
                      margin: '0 0 28px',
                      display: 'flex',
                      flexDirection: 'column',
                      gap: 10,
                    }}
                  >
                    {plan.features.map((f) => (
                      <li
                        key={f}
                        style={{
                          display: 'flex',
                          gap: 8,
                          alignItems: 'flex-start',
                          fontSize: '0.9375rem',
                        }}
                      >
                        <span
                          style={{
                            color: plan.highlighted
                              ? 'rgba(255,255,255,.8)'
                              : 'var(--ppt-color-success, #10b981)',
                            flexShrink: 0,
                          }}
                        >
                          ✓
                        </span>
                        {f}
                      </li>
                    ))}
                  </ul>
                  <button
                    type="button"
                    style={{
                      width: '100%',
                      padding: '12px',
                      background: plan.highlighted ? '#fff' : 'var(--ppt-color-primary, #2563eb)',
                      color: plan.highlighted ? 'var(--ppt-color-primary, #2563eb)' : '#fff',
                      border: 'none',
                      borderRadius: 8,
                      fontWeight: 700,
                      fontSize: '0.9375rem',
                      cursor: 'pointer',
                    }}
                  >
                    Vyskúšať zadarmo
                  </button>
                </div>
              ))}
            </div>
          </div>
        </section>

        {/* CTA band */}
        <section
          style={{ maxWidth: 800, margin: '0 auto', padding: '72px 24px', textAlign: 'center' }}
        >
          <h2
            style={{
              fontSize: '1.75rem',
              fontWeight: 800,
              color: 'var(--ppt-fg-primary)',
              marginBottom: 12,
            }}
          >
            Máte otázky?
          </h2>
          <p style={{ color: 'var(--ppt-fg-secondary)', marginBottom: 28 }}>
            Náš tím je pripravený pomôcť vám vybrať správny plán a nastaviť účet.
          </p>
          <a
            href="/contact"
            style={{
              display: 'inline-block',
              padding: '14px 32px',
              background: 'var(--ppt-color-primary, #2563eb)',
              color: '#fff',
              borderRadius: 8,
              fontWeight: 700,
              textDecoration: 'none',
              fontSize: '1rem',
            }}
          >
            Kontaktovať predaj
          </a>
        </section>
      </main>

      <Footer />
    </div>
  );
}
