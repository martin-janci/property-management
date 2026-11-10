'use client';

/**
 * Agent / realtor public profile page — Reality Portal.
 * Route: /[locale]/realtor/[id]  (the `[id]` segment is the realtor's user id)
 * Screen-map: docs/screens/reality/agent-profile.md
 *
 * Data is fetched client-side via TanStack Query against the real
 * reality-server endpoints (mirrors the agency/[slug] public-profile pattern):
 *   - `GET /api/v1/realtors/{id}/profile`  → public realtor profile
 *   - `GET /api/v1/realtors/{id}/reviews`  → realtor reviews
 * There is no realtor-scoped public "listings" endpoint and no
 * realtor-direct inquiry endpoint (inquiries are listing-scoped:
 * `POST /api/v1/inquiries/contact/{listing_id}`), so the listings grid is
 * omitted and the contact modal surfaces the realtor's real contact channels
 * (mailto / tel) instead of faking a submit. Missing fields degrade to a
 * not-found / empty state rather than being fabricated.
 */

import { useQuery } from '@tanstack/react-query';
import { useParams } from 'next/navigation';
import { useLocale, useTranslations } from 'next-intl';
import { useMemo, useState } from 'react';
import { Footer, Header } from '@/components/ui';
import { getRealtorPublicProfile, getRealtorReviews, RealtorApiError } from '@/lib/realtor-api';

const PAGE_BG: React.CSSProperties = {
  minHeight: '100vh',
  display: 'flex',
  flexDirection: 'column',
  background: 'var(--ppt-bg-app)',
};

function formatMonthYear(iso: string | null | undefined, locale: string): string {
  if (!iso) return '—';
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return '—';
  return new Intl.DateTimeFormat(locale, { month: 'long', year: 'numeric' }).format(d);
}

export default function AgentProfilePage() {
  const t = useTranslations('pages.agentProfile');
  const locale = useLocale();
  const params = useParams<{ id: string }>();
  const id = params?.id ?? '';

  const [contactOpen, setContactOpen] = useState(false);
  const [message, setMessage] = useState('');

  const {
    data: profile,
    isLoading,
    isError,
    error,
  } = useQuery({
    queryKey: ['realtor-profile', id],
    queryFn: () => getRealtorPublicProfile(id),
    enabled: !!id,
    retry: (count, err) => !(err instanceof RealtorApiError && err.status === 404) && count < 1,
  });

  const { data: reviewsData } = useQuery({
    queryKey: ['realtor-reviews', id],
    queryFn: () => getRealtorReviews(id),
    enabled: !!id,
  });

  const reviews = reviewsData?.reviews ?? [];
  const reviewCount = reviewsData?.total ?? reviews.length;
  const avgRating = reviewsData?.avg_rating ?? null;

  // The public profile contract carries no display name (see realtor-api.ts),
  // so fall back to the tagline or a generic label — never a fabricated name.
  const displayName = profile?.tagline?.trim() || t('defaultName');

  const showEmail = !!(profile?.show_email && profile?.email_public);
  const showPhone = !!(profile?.show_phone && profile?.phone);

  const mailtoHref = useMemo(() => {
    if (!showEmail || !profile?.email_public) return null;
    const subject = encodeURIComponent(t('mailtoSubject'));
    const body = encodeURIComponent(message);
    return `mailto:${profile.email_public}?subject=${subject}${body ? `&body=${body}` : ''}`;
  }, [showEmail, profile?.email_public, message, t]);

  if (isLoading) {
    return (
      <div style={PAGE_BG}>
        <Header />
        <main style={{ flex: 1, padding: 48, textAlign: 'center' }}>{t('loading')}</main>
        <Footer />
      </div>
    );
  }

  const notFound =
    (isError && error instanceof RealtorApiError && error.status === 404) || !profile;

  if (isError || notFound) {
    return (
      <div style={PAGE_BG}>
        <Header />
        <main style={{ flex: 1, padding: 48, textAlign: 'center' }}>
          <h1 style={{ fontSize: '1.25rem', fontWeight: 700, color: 'var(--ppt-fg-primary)' }}>
            {t('notFoundTitle')}
          </h1>
          <p style={{ color: 'var(--ppt-fg-secondary)', marginTop: 8 }}>{t('notFoundBody')}</p>
        </main>
        <Footer />
      </div>
    );
  }

  const specializations = profile.specializations ?? [];
  const languages = profile.languages ?? [];

  const stats: { label: string; value: string | number }[] = [
    { label: t('activeListings'), value: profile.active_listings },
    { label: t('totalListings'), value: profile.total_listings },
    { label: t('views'), value: profile.total_views },
  ];
  if (reviewCount > 0) {
    stats.push({
      label: t('rating'),
      value: `${avgRating != null ? avgRating.toFixed(1) : '—'} ★ (${reviewCount})`,
    });
  }

  return (
    <div data-i18n="pages.agent-profile.root" style={PAGE_BG}>
      <Header />

      <main style={{ flex: 1 }}>
        {/* Cover */}
        <div
          style={{
            height: 220,
            background: 'linear-gradient(135deg, #1e3a5f, #2563eb)',
          }}
        />

        {/* Identity */}
        <div
          style={{
            background: 'var(--ppt-bg-surface)',
            borderBottom: '1px solid var(--ppt-border-default, #e5e7eb)',
          }}
        >
          <div style={{ maxWidth: 1100, margin: '0 auto', padding: '0 24px 28px' }}>
            <div
              style={{
                display: 'flex',
                gap: 24,
                alignItems: 'flex-end',
                marginTop: -48,
                marginBottom: 20,
                flexWrap: 'wrap',
              }}
            >
              {profile.photo_url ? (
                // eslint-disable-next-line @next/next/no-img-element
                <img
                  src={profile.photo_url}
                  alt={displayName}
                  style={{
                    width: 96,
                    height: 96,
                    borderRadius: '50%',
                    border: '4px solid var(--ppt-bg-surface)',
                    objectFit: 'cover',
                    flexShrink: 0,
                  }}
                />
              ) : (
                <div
                  aria-hidden
                  style={{
                    width: 96,
                    height: 96,
                    borderRadius: '50%',
                    border: '4px solid var(--ppt-bg-surface)',
                    background: 'var(--ppt-bg-subtle, #e2e8f0)',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    fontSize: '2rem',
                    flexShrink: 0,
                  }}
                >
                  🏠
                </div>
              )}
              <div style={{ flex: 1, paddingBottom: 4 }}>
                <div
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 8,
                    flexWrap: 'wrap',
                    marginBottom: 3,
                  }}
                >
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
                  {profile.license_verified_at && (
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
                      ✓ {t('verified')}
                    </span>
                  )}
                </div>
                {profile.experience_years != null && (
                  <div
                    style={{
                      color: 'var(--ppt-fg-secondary)',
                      marginBottom: 4,
                      fontSize: '0.9375rem',
                    }}
                  >
                    {t('experience', { count: profile.experience_years })}
                  </div>
                )}
              </div>
              <button
                type="button"
                onClick={() => setContactOpen(true)}
                disabled={!profile.accept_inquiries && !showEmail && !showPhone}
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
                {t('contact')}
              </button>
            </div>

            {/* Stats band */}
            <div style={{ display: 'flex', gap: 32, flexWrap: 'wrap' }}>
              {stats.map((stat) => (
                <div key={stat.label}>
                  <div
                    style={{
                      fontSize: '1.375rem',
                      fontWeight: 800,
                      color: 'var(--ppt-fg-primary)',
                    }}
                  >
                    {stat.value}
                  </div>
                  <div style={{ fontSize: '0.8125rem', color: 'var(--ppt-fg-muted, #9ca3af)' }}>
                    {stat.label}
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>

        {/* Main content */}
        <div
          style={{
            maxWidth: 1100,
            margin: '0 auto',
            padding: '36px 24px',
            display: 'grid',
            gridTemplateColumns: '1fr 300px',
            gap: 36,
            alignItems: 'start',
          }}
        >
          {/* Left column */}
          <div>
            {/* Bio */}
            {profile.bio && (
              <section style={{ marginBottom: 40 }}>
                <h2
                  style={{
                    fontSize: '1.125rem',
                    fontWeight: 700,
                    color: 'var(--ppt-fg-primary)',
                    marginBottom: 12,
                  }}
                >
                  {t('about')}
                </h2>
                <p style={{ color: 'var(--ppt-fg-secondary)', lineHeight: 1.7 }}>{profile.bio}</p>
              </section>
            )}

            {/* Reviews */}
            <section>
              <h2
                style={{
                  fontSize: '1.125rem',
                  fontWeight: 700,
                  color: 'var(--ppt-fg-primary)',
                  marginBottom: 16,
                }}
              >
                {t('reviews')} ({reviewCount})
              </h2>
              {reviews.length === 0 ? (
                <p style={{ color: 'var(--ppt-fg-secondary)' }}>{t('noReviews')}</p>
              ) : (
                <div style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
                  {reviews.map((review) => (
                    <div
                      key={review.id}
                      style={{
                        background: 'var(--ppt-bg-surface)',
                        borderRadius: 10,
                        padding: '18px 20px',
                        boxShadow: '0 1px 3px rgba(0,0,0,.05)',
                      }}
                    >
                      <div
                        style={{
                          display: 'flex',
                          justifyContent: 'space-between',
                          alignItems: 'center',
                          marginBottom: 8,
                          gap: 10,
                          flexWrap: 'wrap',
                        }}
                      >
                        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                          <span style={{ fontWeight: 600, color: 'var(--ppt-fg-primary)' }}>
                            {review.reviewer_name}
                          </span>
                          {review.verified_buyer && (
                            <span
                              style={{
                                fontSize: '0.75rem',
                                color: 'var(--ppt-color-success-dark, #047857)',
                              }}
                            >
                              ✓ {t('reviewVerified')}
                            </span>
                          )}
                        </div>
                        <div style={{ display: 'flex', gap: 1 }}>
                          {Array.from({ length: 5 }).map((_, i) => (
                            <span
                              key={`star-${review.id}-${i}`}
                              style={{
                                color: i < review.rating ? '#facc15' : '#d1d5db',
                                fontSize: '0.9375rem',
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
                      <span
                        style={{ fontSize: '0.8125rem', color: 'var(--ppt-fg-muted, #9ca3af)' }}
                      >
                        {formatMonthYear(review.created_at, locale)}
                      </span>
                    </div>
                  ))}
                </div>
              )}
            </section>
          </div>

          {/* Sidebar */}
          <aside>
            <div
              style={{
                background: 'var(--ppt-bg-surface)',
                borderRadius: 12,
                padding: '24px',
                boxShadow: '0 1px 4px rgba(0,0,0,.07)',
                position: 'sticky',
                top: 24,
              }}
            >
              <h2
                style={{
                  fontSize: '1rem',
                  fontWeight: 700,
                  color: 'var(--ppt-fg-primary)',
                  margin: '0 0 16px',
                }}
              >
                {t('contactDetails')}
              </h2>
              {!showPhone && !showEmail ? (
                <p
                  style={{
                    fontSize: '0.875rem',
                    color: 'var(--ppt-fg-secondary)',
                    marginBottom: 16,
                  }}
                >
                  {t('noContact')}
                </p>
              ) : (
                <div
                  style={{ display: 'flex', flexDirection: 'column', gap: 10, marginBottom: 20 }}
                >
                  {showPhone && (
                    <a
                      href={`tel:${profile.phone}`}
                      style={{
                        display: 'flex',
                        gap: 10,
                        alignItems: 'center',
                        color: 'var(--ppt-fg-primary)',
                        textDecoration: 'none',
                        fontSize: '0.9375rem',
                      }}
                    >
                      <span>📞</span> {profile.phone}
                    </a>
                  )}
                  {showEmail && (
                    <a
                      href={`mailto:${profile.email_public}`}
                      style={{
                        display: 'flex',
                        gap: 10,
                        alignItems: 'center',
                        color: 'var(--ppt-fg-primary)',
                        textDecoration: 'none',
                        fontSize: '0.9375rem',
                        wordBreak: 'break-all',
                      }}
                    >
                      <span>✉️</span> {profile.email_public}
                    </a>
                  )}
                </div>
              )}

              {profile.license_number && (
                <div
                  style={{
                    fontSize: '0.8125rem',
                    color: 'var(--ppt-fg-muted, #9ca3af)',
                    marginBottom: 6,
                  }}
                >
                  {t('license', { number: profile.license_number })}
                </div>
              )}
              <div
                style={{
                  fontSize: '0.8125rem',
                  color: 'var(--ppt-fg-muted, #9ca3af)',
                  marginBottom: 16,
                }}
              >
                {t('memberSince', { date: formatMonthYear(profile.created_at, locale) })}
              </div>

              {specializations.length > 0 && (
                <div style={{ marginBottom: 14 }}>
                  <div
                    style={{
                      fontSize: '0.875rem',
                      fontWeight: 600,
                      color: 'var(--ppt-fg-primary)',
                      marginBottom: 6,
                    }}
                  >
                    {t('specializations')}
                  </div>
                  <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6 }}>
                    {specializations.map((s) => (
                      <span
                        key={s}
                        style={{
                          padding: '3px 10px',
                          background: 'var(--ppt-bg-subtle, #f1f5f9)',
                          borderRadius: 99,
                          fontSize: '0.8125rem',
                          color: 'var(--ppt-fg-secondary)',
                        }}
                      >
                        {s}
                      </span>
                    ))}
                  </div>
                </div>
              )}

              {languages.length > 0 && (
                <div style={{ marginBottom: 20 }}>
                  <div
                    style={{
                      fontSize: '0.875rem',
                      fontWeight: 600,
                      color: 'var(--ppt-fg-primary)',
                      marginBottom: 6,
                    }}
                  >
                    {t('languages')}
                  </div>
                  <div style={{ fontSize: '0.875rem', color: 'var(--ppt-fg-secondary)' }}>
                    {languages.join(' · ')}
                  </div>
                </div>
              )}

              {(showEmail || showPhone) && (
                <button
                  type="button"
                  onClick={() => setContactOpen(true)}
                  style={{
                    width: '100%',
                    padding: '12px',
                    background: 'var(--ppt-color-primary, #2563eb)',
                    color: '#fff',
                    border: 'none',
                    borderRadius: 8,
                    fontWeight: 700,
                    cursor: 'pointer',
                  }}
                >
                  {t('sendMessage')}
                </button>
              )}
            </div>
          </aside>
        </div>
      </main>

      {/* Contact modal — no realtor-direct inquiry endpoint exists, so we hand
          off to the agent's real contact channel (mailto) rather than faking a
          submit. */}
      {contactOpen && (
        <div
          role="dialog"
          aria-modal="true"
          aria-label={t('modalAriaLabel')}
          style={{
            position: 'fixed',
            inset: 0,
            background: 'rgba(0,0,0,.5)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            zIndex: 1000,
            padding: 24,
          }}
          onClick={(e) => {
            if (e.target === e.currentTarget) setContactOpen(false);
          }}
        >
          <div
            style={{
              background: 'var(--ppt-bg-surface)',
              borderRadius: 14,
              padding: '32px',
              width: '100%',
              maxWidth: 480,
            }}
          >
            <h2
              style={{
                fontSize: '1.25rem',
                fontWeight: 700,
                color: 'var(--ppt-fg-primary)',
                margin: '0 0 20px',
              }}
            >
              {t('modalTitle', { name: displayName })}
            </h2>

            {showEmail ? (
              <div style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
                <textarea
                  rows={5}
                  placeholder={t('modalPlaceholder')}
                  value={message}
                  onChange={(e) => setMessage(e.target.value)}
                  style={{
                    width: '100%',
                    padding: '11px 14px',
                    borderRadius: 8,
                    border: '1px solid var(--ppt-border-default, #e5e7eb)',
                    fontSize: '0.9375rem',
                    resize: 'vertical',
                    boxSizing: 'border-box',
                    outline: 'none',
                  }}
                />
                <p
                  style={{
                    fontSize: '0.8125rem',
                    color: 'var(--ppt-fg-muted, #9ca3af)',
                    margin: 0,
                  }}
                >
                  {t('modalEmailHint')}
                </p>
                <div style={{ display: 'flex', gap: 10, justifyContent: 'flex-end' }}>
                  <button
                    type="button"
                    onClick={() => setContactOpen(false)}
                    style={{
                      padding: '10px 20px',
                      background: 'var(--ppt-bg-app)',
                      border: '1px solid var(--ppt-border-default, #e5e7eb)',
                      borderRadius: 8,
                      cursor: 'pointer',
                      fontWeight: 600,
                    }}
                  >
                    {t('modalCancel')}
                  </button>
                  <a
                    href={mailtoHref ?? '#'}
                    onClick={() => setContactOpen(false)}
                    style={{
                      padding: '10px 24px',
                      background: 'var(--ppt-color-primary, #2563eb)',
                      color: '#fff',
                      border: 'none',
                      borderRadius: 8,
                      fontWeight: 700,
                      cursor: 'pointer',
                      textDecoration: 'none',
                    }}
                  >
                    {t('modalSendEmail')}
                  </a>
                </div>
              </div>
            ) : (
              <div style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
                <p style={{ color: 'var(--ppt-fg-secondary)', margin: 0, lineHeight: 1.6 }}>
                  {showPhone ? t('modalPhoneIntro') : t('modalNoContact')}
                </p>
                {showPhone && (
                  <a
                    href={`tel:${profile.phone}`}
                    style={{
                      fontSize: '1.0625rem',
                      fontWeight: 700,
                      color: 'var(--ppt-color-primary, #2563eb)',
                      textDecoration: 'none',
                    }}
                  >
                    📞 {profile.phone}
                  </a>
                )}
                <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
                  <button
                    type="button"
                    onClick={() => setContactOpen(false)}
                    style={{
                      padding: '10px 20px',
                      background: 'var(--ppt-bg-app)',
                      border: '1px solid var(--ppt-border-default, #e5e7eb)',
                      borderRadius: 8,
                      cursor: 'pointer',
                      fontWeight: 600,
                    }}
                  >
                    {t('modalClose')}
                  </button>
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      <Footer />
    </div>
  );
}
