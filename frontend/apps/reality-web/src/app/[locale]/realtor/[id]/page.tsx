'use client';

/**
 * Agent / realtor public profile page — Reality Portal.
 * Route: /[locale]/realtor/[id]
 * Screen-map: docs/screens/reality/agent-profile.md
 */

import { useState } from 'react';
import { Footer, Header } from '@/components/ui';
import { MOCK_AGENT, MOCK_AGENT_LISTINGS, MOCK_AGENT_REVIEWS } from './_mock';

// TODO: replace listing cards with @ppt/ui-kit/ListingCard once available
// TODO: replace status badges with @ppt/ui-kit/StatusPill once available

export default function AgentProfilePage() {
  const [contactOpen, setContactOpen] = useState(false);
  const [message, setMessage] = useState('');
  const [messageSent, setMessageSent] = useState(false);
  const agent = MOCK_AGENT;

  const handleSendMessage = (e: React.FormEvent) => {
    e.preventDefault();
    if (message.trim()) {
      setMessageSent(true);
      setContactOpen(false);
    }
  };

  return (
    <div
      data-i18n="pages.agent-profile.root"
      style={{ minHeight: '100vh', display: 'flex', flexDirection: 'column', background: 'var(--ppt-bg-app)' }}
    >
      <Header />

      <main style={{ flex: 1 }}>
        {/* Cover */}
        <div
          style={{
            height: 220,
            background: `url(${agent.coverUrl}) center/cover no-repeat, linear-gradient(135deg, #1e3a5f, #2563eb)`,
          }}
        />

        {/* Identity */}
        <div style={{ background: 'var(--ppt-bg-surface)', borderBottom: '1px solid var(--ppt-border-default, #e5e7eb)' }}>
          <div style={{ maxWidth: 1100, margin: '0 auto', padding: '0 24px 28px' }}>
            <div style={{ display: 'flex', gap: 24, alignItems: 'flex-end', marginTop: -48, marginBottom: 20, flexWrap: 'wrap' }}>
              {/* eslint-disable-next-line @next/next/no-img-element */}
              <img
                src={agent.avatarUrl}
                alt={agent.name}
                style={{
                  width: 96,
                  height: 96,
                  borderRadius: '50%',
                  border: '4px solid var(--ppt-bg-surface)',
                  objectFit: 'cover',
                  flexShrink: 0,
                }}
              />
              <div style={{ flex: 1, paddingBottom: 4 }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8, flexWrap: 'wrap', marginBottom: 3 }}>
                  <h1 style={{ fontSize: '1.5rem', fontWeight: 800, color: 'var(--ppt-fg-primary)', margin: 0 }}>{agent.name}</h1>
                  {agent.verified && (
                    /* TODO: replace with @ppt/ui-kit/StatusPill once available */
                    <span
                      style={{
                        padding: '3px 10px',
                        borderRadius: 99,
                        background: 'var(--ppt-color-success-light, #d1fae5)',
                        color: 'var(--ppt-color-success-dark, #047857)',
                        fontSize: '0.75rem',
                        fontWeight: 600,
                      }}
                    >
                      ✓ Overený maklér
                    </span>
                  )}
                </div>
                <div style={{ color: 'var(--ppt-fg-secondary)', marginBottom: 4, fontSize: '0.9375rem' }}>{agent.title}</div>
                <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                  {/* eslint-disable-next-line @next/next/no-img-element */}
                  <img src={agent.agencyLogoUrl} alt={agent.agencyName} style={{ width: 24, height: 24, borderRadius: 4 }} />
                  <span style={{ fontSize: '0.875rem', color: 'var(--ppt-fg-secondary)' }}>{agent.agencyName}</span>
                </div>
              </div>
              <button
                type="button"
                onClick={() => setContactOpen(true)}
                style={{
                  padding: '12px 28px',
                  background: 'var(--ppt-color-primary, #2563eb)',
                  color: '#fff',
                  border: 'none',
                  borderRadius: 8,
                  fontWeight: 700,
                  cursor: 'pointer',
                  fontSize: '0.9375rem',
                  flexShrink: 0,
                }}
              >
                Kontaktovať
              </button>
            </div>

            {/* Stats band */}
            <div style={{ display: 'flex', gap: 32, flexWrap: 'wrap' }}>
              {[
                { label: 'Aktívne inzeráty', value: agent.stats.activeListings },
                { label: 'Predaných (12m)', value: agent.stats.soldLastYear },
                { label: 'Priem. dni na trhu', value: agent.stats.avgDaysOnMarket },
                { label: 'Hodnotenie', value: `${agent.stats.avgRating} ★ (${agent.stats.reviewCount})` },
              ].map((stat) => (
                <div key={stat.label}>
                  <div style={{ fontSize: '1.375rem', fontWeight: 800, color: 'var(--ppt-fg-primary)' }}>{stat.value}</div>
                  <div style={{ fontSize: '0.8125rem', color: 'var(--ppt-fg-muted, #9ca3af)' }}>{stat.label}</div>
                </div>
              ))}
            </div>
          </div>
        </div>

        {/* Main content */}
        <div style={{ maxWidth: 1100, margin: '0 auto', padding: '36px 24px', display: 'grid', gridTemplateColumns: '1fr 300px', gap: 36, alignItems: 'start' }}>
          {/* Left column */}
          <div>
            {/* Bio */}
            <section style={{ marginBottom: 40 }}>
              <h2 style={{ fontSize: '1.125rem', fontWeight: 700, color: 'var(--ppt-fg-primary)', marginBottom: 12 }}>O makléri</h2>
              <p style={{ color: 'var(--ppt-fg-secondary)', lineHeight: 1.7 }}>{agent.bio}</p>
            </section>

            {/* Listings grid */}
            <section style={{ marginBottom: 40 }}>
              <h2 style={{ fontSize: '1.125rem', fontWeight: 700, color: 'var(--ppt-fg-primary)', marginBottom: 16 }}>
                Aktívne ponuky ({MOCK_AGENT_LISTINGS.length})
              </h2>
              {/* TODO: replace with @ppt/ui-kit/ListingCard once available */}
              <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(240px, 1fr))', gap: 16 }}>
                {MOCK_AGENT_LISTINGS.map((listing) => (
                  <div key={listing.id} style={{ background: 'var(--ppt-bg-surface)', borderRadius: 10, overflow: 'hidden', boxShadow: '0 1px 3px rgba(0,0,0,.07)' }}>
                    {/* eslint-disable-next-line @next/next/no-img-element */}
                    <img src={listing.imageUrl} alt={listing.title} style={{ width: '100%', height: 160, objectFit: 'cover' }} />
                    <div style={{ padding: '12px 14px' }}>
                      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', gap: 6, marginBottom: 4 }}>
                        <div style={{ fontWeight: 600, fontSize: '0.9rem', color: 'var(--ppt-fg-primary)', lineHeight: 1.3 }}>{listing.title}</div>
                        {/* TODO: replace with @ppt/ui-kit/StatusPill once available */}
                        <span style={{ padding: '2px 8px', borderRadius: 99, fontSize: '0.75rem', fontWeight: 600, background: listing.type === 'sale' ? 'var(--ppt-color-info-light, #dbeafe)' : 'var(--ppt-color-success-light, #d1fae5)', color: listing.type === 'sale' ? 'var(--ppt-color-info-dark, #1e40af)' : 'var(--ppt-color-success-dark, #047857)', flexShrink: 0 }}>
                          {listing.type === 'sale' ? 'Predaj' : 'Prenájom'}
                        </span>
                      </div>
                      <div style={{ fontSize: '0.8125rem', color: 'var(--ppt-fg-secondary)', marginBottom: 6 }}>{listing.location} · {listing.area} m²</div>
                      <div style={{ fontWeight: 700, color: 'var(--ppt-fg-primary)', fontSize: '0.9375rem' }}>
                        {listing.currency}{listing.price.toLocaleString('sk-SK')}
                        {listing.type === 'rent' ? '/mes' : ''}
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </section>

            {/* Reviews */}
            <section>
              <h2 style={{ fontSize: '1.125rem', fontWeight: 700, color: 'var(--ppt-fg-primary)', marginBottom: 16 }}>
                Hodnotenia ({MOCK_AGENT_REVIEWS.length})
              </h2>
              <div style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
                {MOCK_AGENT_REVIEWS.map((review) => (
                  <div key={review.id} style={{ background: 'var(--ppt-bg-surface)', borderRadius: 10, padding: '18px 20px', boxShadow: '0 1px 3px rgba(0,0,0,.05)' }}>
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 8, gap: 10, flexWrap: 'wrap' }}>
                      <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                        <span style={{ fontWeight: 600, color: 'var(--ppt-fg-primary)' }}>{review.reviewer}</span>
                        {review.verified && (
                          <span style={{ fontSize: '0.75rem', color: 'var(--ppt-color-success-dark, #047857)' }}>✓ Overené</span>
                        )}
                      </div>
                      <div style={{ display: 'flex', gap: 1 }}>
                        {Array.from({ length: 5 }).map((_, i) => (
                          <span key={`star-${review.id}-${i}`} style={{ color: i < review.rating ? '#facc15' : '#d1d5db', fontSize: '0.9375rem' }}>★</span>
                        ))}
                      </div>
                    </div>
                    <p style={{ color: 'var(--ppt-fg-secondary)', margin: '0 0 8px', lineHeight: 1.65 }}>{review.comment}</p>
                    <span style={{ fontSize: '0.8125rem', color: 'var(--ppt-fg-muted, #9ca3af)' }}>{review.date}</span>
                  </div>
                ))}
              </div>
            </section>
          </div>

          {/* Sidebar */}
          <aside>
            <div style={{ background: 'var(--ppt-bg-surface)', borderRadius: 12, padding: '24px', boxShadow: '0 1px 4px rgba(0,0,0,.07)', position: 'sticky', top: 24 }}>
              <h2 style={{ fontSize: '1rem', fontWeight: 700, color: 'var(--ppt-fg-primary)', margin: '0 0 16px' }}>Kontaktné údaje</h2>
              <div style={{ display: 'flex', flexDirection: 'column', gap: 10, marginBottom: 20 }}>
                <a href={`tel:${agent.phone}`} style={{ display: 'flex', gap: 10, alignItems: 'center', color: 'var(--ppt-fg-primary)', textDecoration: 'none', fontSize: '0.9375rem' }}>
                  <span>📞</span> {agent.phone}
                </a>
                <a href={`mailto:${agent.email}`} style={{ display: 'flex', gap: 10, alignItems: 'center', color: 'var(--ppt-fg-primary)', textDecoration: 'none', fontSize: '0.9375rem', wordBreak: 'break-all' }}>
                  <span>✉️</span> {agent.email}
                </a>
              </div>

              <div style={{ fontSize: '0.8125rem', color: 'var(--ppt-fg-muted, #9ca3af)', marginBottom: 6 }}>Licencia: {agent.licenseNumber}</div>
              <div style={{ fontSize: '0.8125rem', color: 'var(--ppt-fg-muted, #9ca3af)', marginBottom: 16 }}>Člen od: {agent.memberSince}</div>

              <div style={{ marginBottom: 14 }}>
                <div style={{ fontSize: '0.875rem', fontWeight: 600, color: 'var(--ppt-fg-primary)', marginBottom: 6 }}>Špecializácia</div>
                <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6 }}>
                  {agent.specializations.map((s) => (
                    <span key={s} style={{ padding: '3px 10px', background: 'var(--ppt-bg-subtle, #f1f5f9)', borderRadius: 99, fontSize: '0.8125rem', color: 'var(--ppt-fg-secondary)' }}>{s}</span>
                  ))}
                </div>
              </div>

              <div style={{ marginBottom: 20 }}>
                <div style={{ fontSize: '0.875rem', fontWeight: 600, color: 'var(--ppt-fg-primary)', marginBottom: 6 }}>Jazyky</div>
                <div style={{ fontSize: '0.875rem', color: 'var(--ppt-fg-secondary)' }}>{agent.languages.join(' · ')}</div>
              </div>

              <button
                type="button"
                onClick={() => setContactOpen(true)}
                style={{ width: '100%', padding: '12px', background: 'var(--ppt-color-primary, #2563eb)', color: '#fff', border: 'none', borderRadius: 8, fontWeight: 700, cursor: 'pointer' }}
              >
                Odoslať správu
              </button>

              {messageSent && (
                <p style={{ marginTop: 12, color: 'var(--ppt-color-success-dark, #047857)', fontSize: '0.875rem', textAlign: 'center' }}>
                  ✓ Správa odoslaná!
                </p>
              )}
            </div>
          </aside>
        </div>
      </main>

      {/* Contact modal */}
      {/* TODO: replace with @ppt/ui-kit/ModalDrawer once available */}
      {contactOpen && (
        <div
          role="dialog"
          aria-modal="true"
          aria-label="Kontaktovať makléra"
          style={{ position: 'fixed', inset: 0, background: 'rgba(0,0,0,.5)', display: 'flex', alignItems: 'center', justifyContent: 'center', zIndex: 1000, padding: 24 }}
          onClick={(e) => { if (e.target === e.currentTarget) setContactOpen(false); }}
        >
          <div style={{ background: 'var(--ppt-bg-surface)', borderRadius: 14, padding: '32px', width: '100%', maxWidth: 480 }}>
            <h2 style={{ fontSize: '1.25rem', fontWeight: 700, color: 'var(--ppt-fg-primary)', margin: '0 0 20px' }}>Kontaktovať {agent.name}</h2>
            <form onSubmit={handleSendMessage} style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
              <textarea
                rows={5}
                required
                placeholder="Dobrý deň, mám záujem o Vaše služby..."
                value={message}
                onChange={(e) => setMessage(e.target.value)}
                style={{ width: '100%', padding: '11px 14px', borderRadius: 8, border: '1px solid var(--ppt-border-default, #e5e7eb)', fontSize: '0.9375rem', resize: 'vertical', boxSizing: 'border-box', outline: 'none' }}
              />
              <div style={{ display: 'flex', gap: 10, justifyContent: 'flex-end' }}>
                <button type="button" onClick={() => setContactOpen(false)} style={{ padding: '10px 20px', background: 'var(--ppt-bg-app)', border: '1px solid var(--ppt-border-default, #e5e7eb)', borderRadius: 8, cursor: 'pointer', fontWeight: 600 }}>Zrušiť</button>
                <button type="submit" style={{ padding: '10px 24px', background: 'var(--ppt-color-primary, #2563eb)', color: '#fff', border: 'none', borderRadius: 8, fontWeight: 700, cursor: 'pointer' }}>Odoslať</button>
              </div>
            </form>
          </div>
        </div>
      )}

      <Footer />
    </div>
  );
}
