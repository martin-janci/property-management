'use client';

/**
 * Public user / realtor profile page — Reality Portal.
 * Screen-map: docs/screens/reality/profile.md
 */

import { useState } from 'react';
import { Footer, Header } from '@/components/ui';
import { MOCK_ACTIVITY, MOCK_LISTINGS, MOCK_PROFILE, MOCK_REVIEWS, type ProfileTab } from './_mock';

// TODO: replace tabs with @ppt/ui-kit/SegmentedControl once available
// TODO: replace listing cards with @ppt/ui-kit/ListingCard once available
// TODO: replace status badges with @ppt/ui-kit/StatusPill once available

const STATUS_COLORS: Record<string, { bg: string; color: string; label: string }> = {
  active: {
    bg: 'var(--ppt-color-success-light, #d1fae5)',
    color: 'var(--ppt-color-success-dark, #047857)',
    label: 'Aktívny',
  },
  sold: {
    bg: 'var(--ppt-color-info-light, #dbeafe)',
    color: 'var(--ppt-color-info-dark, #1e40af)',
    label: 'Predaný',
  },
  rented: {
    bg: 'var(--ppt-color-warning-light, #fef3c7)',
    color: 'var(--ppt-color-warning-dark, #b45309)',
    label: 'Prenajatý',
  },
};

const ACTIVITY_ICONS: Record<string, string> = {
  listing_viewed: '👁️',
  inquiry_sent: '✉️',
  favorite_added: '❤️',
  search_saved: '🔔',
};

const TABS: { id: ProfileTab; label: string }[] = [
  { id: 'listings', label: 'Moje inzeráty' },
  { id: 'activity', label: 'Aktivita' },
  { id: 'reviews', label: 'Hodnotenia' },
  { id: 'settings', label: 'Nastavenia' },
];

export default function ProfilePage() {
  const [activeTab, setActiveTab] = useState<ProfileTab>('listings');
  const profile = MOCK_PROFILE;

  return (
    <div
      data-i18n="pages.profile.root"
      style={{
        minHeight: '100vh',
        display: 'flex',
        flexDirection: 'column',
        background: 'var(--ppt-bg-app)',
      }}
    >
      <Header />

      <main style={{ flex: 1 }}>
        {/* Cover */}
        <div
          style={{
            height: 200,
            background: `url(${profile.coverUrl}) center/cover no-repeat, linear-gradient(135deg, #1e3a5f, #2563eb)`,
            position: 'relative',
          }}
        />

        {/* Identity strip */}
        <div
          style={{
            background: 'var(--ppt-bg-surface)',
            borderBottom: '1px solid var(--ppt-border-default, #e5e7eb)',
          }}
        >
          <div
            style={{
              maxWidth: 1100,
              margin: '0 auto',
              padding: '0 24px 24px',
              position: 'relative',
            }}
          >
            {/* Avatar */}
            {/* eslint-disable-next-line @next/next/no-img-element */}
            <img
              src={profile.avatarUrl}
              alt={profile.name}
              style={{
                width: 100,
                height: 100,
                borderRadius: '50%',
                border: '4px solid var(--ppt-bg-surface)',
                position: 'relative',
                top: -50,
                marginBottom: -30,
                objectFit: 'cover',
              }}
            />
            <div
              style={{
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'flex-start',
                gap: 16,
                flexWrap: 'wrap',
              }}
            >
              <div>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 4 }}>
                  <h1
                    style={{
                      fontSize: '1.5rem',
                      fontWeight: 800,
                      color: 'var(--ppt-fg-primary)',
                      margin: 0,
                    }}
                  >
                    {profile.name}
                  </h1>
                  {profile.verified && (
                    /* TODO: replace with @ppt/ui-kit/StatusPill once available */
                    <span
                      title="Overený maklér"
                      style={{
                        display: 'inline-flex',
                        alignItems: 'center',
                        gap: 4,
                        padding: '3px 10px',
                        background: 'var(--ppt-color-success-light, #d1fae5)',
                        color: 'var(--ppt-color-success-dark, #047857)',
                        borderRadius: 99,
                        fontSize: '0.75rem',
                        fontWeight: 600,
                      }}
                    >
                      ✓ Overený
                    </span>
                  )}
                </div>
                <p
                  style={{
                    color: 'var(--ppt-fg-secondary)',
                    margin: '0 0 6px',
                    fontSize: '0.9375rem',
                  }}
                >
                  {profile.bio}
                </p>
                <p
                  style={{
                    color: 'var(--ppt-fg-muted, #9ca3af)',
                    fontSize: '0.8125rem',
                    margin: 0,
                  }}
                >
                  Člen od {profile.memberSince}
                </p>
              </div>

              {/* Stats rail */}
              <div style={{ display: 'flex', gap: 32, flexWrap: 'wrap' }}>
                {[
                  { label: 'Aktívne inzeráty', value: profile.stats.activeListings },
                  { label: 'Uzatvorené obchody', value: profile.stats.completedDeals },
                  { label: 'Hodnotenia', value: profile.stats.reviews },
                  { label: 'Priem. hodnotenie', value: `${profile.stats.avgRating} ★` },
                ].map((stat) => (
                  <div key={stat.label} style={{ textAlign: 'center' }}>
                    <div
                      style={{
                        fontSize: '1.5rem',
                        fontWeight: 800,
                        color: 'var(--ppt-fg-primary)',
                      }}
                    >
                      {stat.value}
                    </div>
                    <div
                      style={{
                        fontSize: '0.75rem',
                        color: 'var(--ppt-fg-muted, #9ca3af)',
                        whiteSpace: 'nowrap',
                      }}
                    >
                      {stat.label}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>

        {/* Tabs */}
        <div
          style={{
            background: 'var(--ppt-bg-surface)',
            borderBottom: '1px solid var(--ppt-border-default, #e5e7eb)',
            position: 'sticky',
            top: 0,
            zIndex: 10,
          }}
        >
          <div
            style={{ maxWidth: 1100, margin: '0 auto', padding: '0 24px', display: 'flex', gap: 0 }}
          >
            {/* TODO: replace with @ppt/ui-kit/SegmentedControl once available */}
            {TABS.map((tab) => (
              <button
                key={tab.id}
                type="button"
                onClick={() => setActiveTab(tab.id)}
                style={{
                  padding: '14px 20px',
                  background: 'none',
                  border: 'none',
                  borderBottom:
                    activeTab === tab.id
                      ? '2px solid var(--ppt-color-primary, #2563eb)'
                      : '2px solid transparent',
                  color:
                    activeTab === tab.id
                      ? 'var(--ppt-color-primary, #2563eb)'
                      : 'var(--ppt-fg-secondary)',
                  fontWeight: activeTab === tab.id ? 700 : 400,
                  cursor: 'pointer',
                  fontSize: '0.9375rem',
                  whiteSpace: 'nowrap',
                }}
              >
                {tab.label}
              </button>
            ))}
          </div>
        </div>

        {/* Tab content */}
        <div style={{ maxWidth: 1100, margin: '0 auto', padding: '32px 24px' }}>
          {activeTab === 'listings' && (
            <div
              style={{
                display: 'grid',
                gridTemplateColumns: 'repeat(auto-fill, minmax(280px, 1fr))',
                gap: 20,
              }}
            >
              {MOCK_LISTINGS.map((listing) => (
                /* TODO: replace with @ppt/ui-kit/ListingCard once available */
                <div
                  key={listing.id}
                  style={{
                    background: 'var(--ppt-bg-surface)',
                    borderRadius: 12,
                    overflow: 'hidden',
                    boxShadow: '0 1px 4px rgba(0,0,0,.07)',
                  }}
                >
                  {/* eslint-disable-next-line @next/next/no-img-element */}
                  <img
                    src={listing.imageUrl}
                    alt={listing.title}
                    style={{ width: '100%', height: 180, objectFit: 'cover' }}
                  />
                  <div style={{ padding: 16 }}>
                    <div
                      style={{
                        display: 'flex',
                        justifyContent: 'space-between',
                        alignItems: 'flex-start',
                        gap: 8,
                      }}
                    >
                      <h3
                        style={{
                          fontSize: '1rem',
                          fontWeight: 700,
                          color: 'var(--ppt-fg-primary)',
                          margin: '0 0 4px',
                          lineHeight: 1.3,
                        }}
                      >
                        {listing.title}
                      </h3>
                      <span
                        style={{
                          flexShrink: 0,
                          padding: '3px 8px',
                          borderRadius: 99,
                          fontSize: '0.75rem',
                          fontWeight: 600,
                          background: STATUS_COLORS[listing.status]?.bg,
                          color: STATUS_COLORS[listing.status]?.color,
                        }}
                      >
                        {STATUS_COLORS[listing.status]?.label}
                      </span>
                    </div>
                    <p
                      style={{
                        fontSize: '0.875rem',
                        color: 'var(--ppt-fg-secondary)',
                        margin: '0 0 8px',
                      }}
                    >
                      {listing.location}
                    </p>
                    <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                      <span style={{ fontWeight: 700, color: 'var(--ppt-fg-primary)' }}>
                        {listing.currency}
                        {listing.price.toLocaleString('sk-SK')}
                      </span>
                      <span style={{ fontSize: '0.875rem', color: 'var(--ppt-fg-muted, #9ca3af)' }}>
                        {listing.area} m² · {listing.rooms} izby
                      </span>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}

          {activeTab === 'activity' && (
            <div style={{ maxWidth: 680 }}>
              <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
                {MOCK_ACTIVITY.map((entry) => (
                  <div
                    key={entry.id}
                    style={{
                      background: 'var(--ppt-bg-surface)',
                      borderRadius: 10,
                      padding: '16px',
                      display: 'flex',
                      gap: 14,
                      alignItems: 'center',
                      boxShadow: '0 1px 3px rgba(0,0,0,.05)',
                    }}
                  >
                    <span style={{ fontSize: '1.375rem', flexShrink: 0 }}>
                      {ACTIVITY_ICONS[entry.type]}
                    </span>
                    <div style={{ flex: 1 }}>
                      <div style={{ color: 'var(--ppt-fg-primary)', fontSize: '0.9375rem' }}>
                        {entry.description}
                      </div>
                      <div
                        style={{
                          color: 'var(--ppt-fg-muted, #9ca3af)',
                          fontSize: '0.8125rem',
                          marginTop: 3,
                        }}
                      >
                        {entry.timestamp}
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {activeTab === 'reviews' && (
            <div style={{ maxWidth: 680, display: 'flex', flexDirection: 'column', gap: 16 }}>
              {MOCK_REVIEWS.map((review) => (
                <div
                  key={review.id}
                  style={{
                    background: 'var(--ppt-bg-surface)',
                    borderRadius: 10,
                    padding: '20px',
                    boxShadow: '0 1px 3px rgba(0,0,0,.05)',
                  }}
                >
                  <div
                    style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 8 }}
                  >
                    <span style={{ fontWeight: 600, color: 'var(--ppt-fg-primary)' }}>
                      {review.reviewer}
                    </span>
                    <div style={{ display: 'flex', gap: 2 }}>
                      {Array.from({ length: 5 }).map((_, i) => (
                        <span
                          key={`star-${review.id}-${i}`}
                          style={{
                            color: i < review.rating ? '#facc15' : '#d1d5db',
                            fontSize: '1rem',
                          }}
                        >
                          ★
                        </span>
                      ))}
                    </div>
                  </div>
                  <p
                    style={{
                      color: 'var(--ppt-fg-secondary)',
                      margin: '0 0 8px',
                      lineHeight: 1.65,
                    }}
                  >
                    {review.comment}
                  </p>
                  <span style={{ fontSize: '0.8125rem', color: 'var(--ppt-fg-muted, #9ca3af)' }}>
                    {review.date}
                  </span>
                </div>
              ))}
            </div>
          )}

          {activeTab === 'settings' && (
            <div style={{ maxWidth: 560 }}>
              <p style={{ color: 'var(--ppt-fg-secondary)' }}>
                Nastavenia profilu sú dostupné v sekcii{' '}
                <a href="/account/profile" style={{ color: 'var(--ppt-color-primary, #2563eb)' }}>
                  Môj účet
                </a>
                .
              </p>
            </div>
          )}
        </div>
      </main>

      <Footer />
    </div>
  );
}
