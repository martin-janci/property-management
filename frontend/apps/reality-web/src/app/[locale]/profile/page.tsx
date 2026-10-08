'use client';

/**
 * Public user / realtor profile page — Reality Portal.
 * Screen-map: docs/screens/reality/profile.md
 *
 * Wrapped in ProtectedRoute so anonymous visitors hit the SSO login flow.
 *
 * The identity + portfolio shown here are the REAL signed-in user's: the
 * identity strip comes from the SSO session (and the realtor profile when the
 * user has one), listings from `GET /api/v1/my/listings`, and reviews from
 * `GET /api/v1/realtors/{id}/reviews`. This page previously rendered a shared
 * `MOCK_PROFILE` ("Tomáš Novotný") to every visitor, which was both a UX bug
 * and a privacy footgun — see docs/screens/reality/profile.md Agent Log.
 *
 * The "Aktivita" (activity) tab has no backing endpoint yet, so it shows an
 * explicit empty state instead of fabricated activity rows.
 */

import { useEffect, useState } from 'react';
import { ProtectedRoute } from '@/components/auth';
import { Footer, Header } from '@/components/ui';
import { useAuth } from '@/lib/auth-context';
import {
  getMyRealtorAnalytics,
  getMyRealtorProfile,
  getRealtorReviews,
  listMyListings,
  type MyListing,
  type RealtorAnalytics,
  type RealtorProfile,
  type RealtorReviewsResponse,
} from '@/lib/realtor-api';

type ProfileTab = 'listings' | 'activity' | 'reviews' | 'settings';

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
  draft: {
    bg: 'var(--ppt-bg-app, #f3f4f6)',
    color: 'var(--ppt-fg-secondary, #4b5563)',
    label: 'Koncept',
  },
  paused: {
    bg: 'var(--ppt-bg-app, #f3f4f6)',
    color: 'var(--ppt-fg-secondary, #4b5563)',
    label: 'Pozastavený',
  },
  archived: {
    bg: 'var(--ppt-bg-app, #f3f4f6)',
    color: 'var(--ppt-fg-muted, #9ca3af)',
    label: 'Archivovaný',
  },
};

const CURRENCY_SYMBOLS: Record<string, string> = { EUR: '€', CZK: 'Kč' };

const TABS: { id: ProfileTab; label: string }[] = [
  { id: 'listings', label: 'Moje inzeráty' },
  { id: 'activity', label: 'Aktivita' },
  { id: 'reviews', label: 'Hodnotenia' },
  { id: 'settings', label: 'Nastavenia' },
];

function formatPrice(price: number | string, currency: string): string {
  const value = Number(price);
  const symbol = CURRENCY_SYMBOLS[currency] ?? '';
  const amount = Number.isFinite(value) ? value.toLocaleString('sk-SK') : String(price);
  return symbol ? `${symbol}${amount}` : `${amount} ${currency}`;
}

function formatReviewDate(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  return date.toLocaleDateString('sk-SK', { year: 'numeric', month: 'long' });
}

function statusStyle(status: string): { bg: string; color: string; label: string } {
  return (
    STATUS_COLORS[status] ?? {
      bg: 'var(--ppt-bg-app, #f3f4f6)',
      color: 'var(--ppt-fg-secondary, #4b5563)',
      label: status,
    }
  );
}

function initials(name: string): string {
  const letters = name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((part) => part[0]?.toUpperCase() ?? '')
    .join('');
  return letters || '?';
}

export default function ProfilePage() {
  return (
    <ProtectedRoute>
      <ProfilePageContent />
    </ProtectedRoute>
  );
}

function ProfilePageContent() {
  const { user } = useAuth();
  const [activeTab, setActiveTab] = useState<ProfileTab>('listings');

  const [realtor, setRealtor] = useState<RealtorProfile | null>(null);
  const [stats, setStats] = useState<RealtorAnalytics | null>(null);
  const [reviews, setReviews] = useState<RealtorReviewsResponse | null>(null);

  const [listings, setListings] = useState<MyListing[] | null>(null);
  const [listingsError, setListingsError] = useState<string>();

  // The caller's own listings (works for any signed-in portal user).
  useEffect(() => {
    let cancelled = false;
    listMyListings()
      .then((res) => {
        if (!cancelled) setListings(res.listings);
      })
      .catch(() => {
        if (!cancelled) {
          setListings([]);
          setListingsError('Nepodarilo sa načítať inzeráty.');
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  // Realtor profile + stats + reviews are optional: a plain portal user has no
  // realtor profile (404), in which case the identity falls back to the SSO
  // session and the realtor-only panels stay empty — never fabricated.
  useEffect(() => {
    let cancelled = false;
    getMyRealtorProfile()
      .then(async (profile) => {
        if (cancelled) return;
        setRealtor(profile);
        try {
          const res = await getRealtorReviews(profile.id);
          if (!cancelled) setReviews(res);
        } catch {
          // Reviews unavailable — leave the empty state in place.
        }
      })
      .catch(() => {
        // Not a realtor (or endpoint unavailable) — SSO identity is used.
      });
    getMyRealtorAnalytics()
      .then((value) => {
        if (!cancelled) setStats(value);
      })
      .catch(() => {
        // Stats unavailable — the stat rail hides what it cannot show.
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const displayName = realtor?.name ?? user?.name ?? user?.email ?? '';
  const avatarUrl = realtor?.photoUrl ?? user?.avatar_url ?? null;
  const bio = realtor?.bio ?? null;

  const statItems: { label: string; value: string | number }[] = [];
  if (stats) {
    statItems.push({ label: 'Aktívne inzeráty', value: stats.activeListings });
    statItems.push({ label: 'Všetky inzeráty', value: stats.totalListings });
  }
  if (reviews) {
    statItems.push({ label: 'Hodnotenia', value: reviews.total });
    if (typeof reviews.avg_rating === 'number') {
      statItems.push({ label: 'Priem. hodnotenie', value: `${reviews.avg_rating.toFixed(1)} ★` });
    }
  }

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
            background: 'linear-gradient(135deg, #1e3a5f, #2563eb)',
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
            {avatarUrl ? (
              /* eslint-disable-next-line @next/next/no-img-element */
              <img
                src={avatarUrl}
                alt={displayName}
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
            ) : (
              <div
                aria-hidden="true"
                style={{
                  width: 100,
                  height: 100,
                  borderRadius: '50%',
                  border: '4px solid var(--ppt-bg-surface)',
                  position: 'relative',
                  top: -50,
                  marginBottom: -30,
                  background: 'var(--ppt-color-primary, #2563eb)',
                  color: 'var(--ppt-fg-on-accent, #fff)',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  fontSize: '2rem',
                  fontWeight: 800,
                }}
              >
                {initials(displayName)}
              </div>
            )}
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
                    {displayName}
                  </h1>
                </div>
                {bio && (
                  <p
                    style={{
                      color: 'var(--ppt-fg-secondary)',
                      margin: '0 0 6px',
                      fontSize: '0.9375rem',
                    }}
                  >
                    {bio}
                  </p>
                )}
                {user?.email && (
                  <p
                    style={{
                      color: 'var(--ppt-fg-muted, #9ca3af)',
                      fontSize: '0.8125rem',
                      margin: 0,
                    }}
                  >
                    {user.email}
                  </p>
                )}
              </div>

              {/* Stats rail */}
              {statItems.length > 0 && (
                <div style={{ display: 'flex', gap: 32, flexWrap: 'wrap' }}>
                  {statItems.map((stat) => (
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
              )}
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
          {activeTab === 'listings' && <ListingsTab listings={listings} error={listingsError} />}

          {activeTab === 'activity' && (
            <EmptyState
              title="Aktivita zatiaľ nie je k dispozícii"
              body="Prehľad vašej aktivity bude dostupný, keď pripravíme príslušný prehľad."
            />
          )}

          {activeTab === 'reviews' && <ReviewsTab reviews={reviews} />}

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

function ListingsTab({ listings, error }: { listings: MyListing[] | null; error?: string }) {
  if (listings === null) {
    return <p style={{ color: 'var(--ppt-fg-muted, #9ca3af)' }}>Načítavam inzeráty…</p>;
  }
  if (error) {
    return (
      <p role="alert" style={{ color: 'var(--ppt-color-danger-dark, #b91c1c)' }}>
        {error}
      </p>
    );
  }
  if (listings.length === 0) {
    return (
      <EmptyState
        title="Zatiaľ nemáte žiadne inzeráty"
        body="Keď pridáte inzerát, zobrazí sa tu."
      />
    );
  }

  return (
    <div
      style={{
        display: 'grid',
        gridTemplateColumns: 'repeat(auto-fill, minmax(280px, 1fr))',
        gap: 20,
      }}
    >
      {listings.map((listing) => {
        const badge = statusStyle(listing.status);
        const area = listing.sizeSqm != null ? Number(listing.sizeSqm) : null;
        return (
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
            {/* No image on the my/listings contract yet — neutral placeholder. */}
            <div
              aria-hidden="true"
              style={{
                width: '100%',
                height: 180,
                background: 'linear-gradient(135deg, #e5e7eb, #cbd5e1)',
              }}
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
                    background: badge.bg,
                    color: badge.color,
                  }}
                >
                  {badge.label}
                </span>
              </div>
              <p
                style={{
                  fontSize: '0.875rem',
                  color: 'var(--ppt-fg-secondary)',
                  margin: '0 0 8px',
                }}
              >
                {listing.city}
              </p>
              <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                <span style={{ fontWeight: 700, color: 'var(--ppt-fg-primary)' }}>
                  {formatPrice(listing.price, listing.currency)}
                </span>
                <span style={{ fontSize: '0.875rem', color: 'var(--ppt-fg-muted, #9ca3af)' }}>
                  {area != null ? `${area} m²` : null}
                  {area != null && listing.rooms != null ? ' · ' : null}
                  {listing.rooms != null ? `${listing.rooms} izby` : null}
                </span>
              </div>
            </div>
          </div>
        );
      })}
    </div>
  );
}

function ReviewsTab({ reviews }: { reviews: RealtorReviewsResponse | null }) {
  if (!reviews || reviews.reviews.length === 0) {
    return (
      <EmptyState
        title="Zatiaľ žiadne hodnotenia"
        body="Hodnotenia od klientov sa zobrazia tu, keď ich dostanete."
      />
    );
  }

  return (
    <div style={{ maxWidth: 680, display: 'flex', flexDirection: 'column', gap: 16 }}>
      {reviews.reviews.map((review) => (
        <div
          key={review.id}
          style={{
            background: 'var(--ppt-bg-surface)',
            borderRadius: 10,
            padding: '20px',
            boxShadow: '0 1px 3px rgba(0,0,0,.05)',
          }}
        >
          <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 8 }}>
            <span style={{ fontWeight: 600, color: 'var(--ppt-fg-primary)' }}>
              {review.reviewer_name}
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
          {review.body && (
            <p
              style={{
                color: 'var(--ppt-fg-secondary)',
                margin: '0 0 8px',
                lineHeight: 1.65,
              }}
            >
              {review.body}
            </p>
          )}
          <span style={{ fontSize: '0.8125rem', color: 'var(--ppt-fg-muted, #9ca3af)' }}>
            {formatReviewDate(review.created_at)}
          </span>
        </div>
      ))}
    </div>
  );
}

function EmptyState({ title, body }: { title: string; body: string }) {
  return (
    <div
      style={{
        maxWidth: 520,
        margin: '0 auto',
        textAlign: 'center',
        padding: '48px 24px',
        color: 'var(--ppt-fg-muted, #9ca3af)',
      }}
    >
      <p
        style={{
          fontSize: '1.0625rem',
          fontWeight: 600,
          color: 'var(--ppt-fg-secondary)',
          margin: '0 0 6px',
        }}
      >
        {title}
      </p>
      <p style={{ margin: 0 }}>{body}</p>
    </div>
  );
}
