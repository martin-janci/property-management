'use client';

/**
 * Public agency profile page — Reality Portal (Issue #978).
 * Route: /[locale]/agency/[slug]
 *
 * Mirrors the realtor/[id] public-profile pattern: a 'use client' component
 * with inline styles + hardcoded Slovak copy (the sibling profile does not use
 * next-intl). Data is fetched client-side via @ppt/reality-api-client hooks.
 */

import { useAgencyBySlug, useAgencyListings, useRealtors } from '@ppt/reality-api-client';
import { useParams } from 'next/navigation';
import { Footer, Header } from '@/components/ui';
import { Link } from '@/i18n/routing';

const PAGE_BG: React.CSSProperties = {
  minHeight: '100vh',
  display: 'flex',
  flexDirection: 'column',
  background: 'var(--ppt-bg-app)',
};

export default function AgencyProfilePage() {
  const params = useParams<{ slug: string }>();
  const slug = params?.slug ?? '';

  const { data: agency, isLoading, isError } = useAgencyBySlug(slug);
  const { data: realtors } = useRealtors(agency?.id ?? '');
  const { data: listingsData } = useAgencyListings(agency?.id ?? '', { status: 'active' });

  if (isLoading) {
    return (
      <div style={PAGE_BG}>
        <Header />
        <main style={{ flex: 1, padding: 48, textAlign: 'center' }}>Načítavam…</main>
        <Footer />
      </div>
    );
  }

  if (isError || !agency) {
    return (
      <div style={PAGE_BG}>
        <Header />
        <main style={{ flex: 1, padding: 48, textAlign: 'center' }}>Kancelária sa nenašla.</main>
        <Footer />
      </div>
    );
  }

  const team = realtors ?? [];
  const listings = listingsData?.listings ?? [];

  return (
    <div data-i18n="pages.agency-profile.root" style={PAGE_BG}>
      <Header />

      <main style={{ flex: 1 }}>
        {/* Cover */}
        <div
          style={{
            height: 200,
            background: agency.primaryColor
              ? `linear-gradient(135deg, ${agency.primaryColor}, ${agency.secondaryColor ?? '#2563eb'})`
              : 'linear-gradient(135deg, #1e3a5f, #2563eb)',
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
                flexWrap: 'wrap',
              }}
            >
              {agency.logoUrl && (
                // eslint-disable-next-line @next/next/no-img-element
                <img
                  src={agency.logoUrl}
                  alt={agency.name}
                  style={{
                    width: 96,
                    height: 96,
                    borderRadius: 16,
                    border: '4px solid var(--ppt-bg-surface)',
                    objectFit: 'cover',
                    background: '#fff',
                    flexShrink: 0,
                  }}
                />
              )}
              <div style={{ flex: 1, paddingBottom: 4 }}>
                <h1
                  style={{
                    fontSize: '1.5rem',
                    fontWeight: 800,
                    color: 'var(--ppt-fg-primary)',
                    margin: '0 0 4px',
                  }}
                >
                  {agency.name}
                  {agency.verifiedAt && (
                    <span
                      style={{
                        marginLeft: 8,
                        padding: '3px 10px',
                        borderRadius: 99,
                        background: 'var(--ppt-color-success-light, #d1fae5)',
                        color: 'var(--ppt-color-success-dark, #047857)',
                        fontSize: '0.75rem',
                        fontWeight: 600,
                      }}
                    >
                      ✓ Overená
                    </span>
                  )}
                </h1>
                {agency.address?.city && (
                  <div style={{ color: 'var(--ppt-fg-secondary)', fontSize: '0.9375rem' }}>
                    📍 {agency.address.city}
                    {agency.address.country ? `, ${agency.address.country}` : ''}
                  </div>
                )}
              </div>
            </div>
          </div>
        </div>

        {/* Content */}
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
          <div>
            {/* About */}
            {agency.description && (
              <section style={{ marginBottom: 40 }}>
                <h2 style={{ fontSize: '1.125rem', fontWeight: 700, marginBottom: 12 }}>
                  O kancelárii
                </h2>
                <p style={{ color: 'var(--ppt-fg-secondary)', lineHeight: 1.7 }}>
                  {agency.description}
                </p>
              </section>
            )}

            {/* Team */}
            {team.length > 0 && (
              <section style={{ marginBottom: 40 }}>
                <h2 style={{ fontSize: '1.125rem', fontWeight: 700, marginBottom: 16 }}>
                  Náš tím ({team.length})
                </h2>
                <div
                  style={{
                    display: 'grid',
                    gridTemplateColumns: 'repeat(auto-fill, minmax(200px, 1fr))',
                    gap: 16,
                  }}
                >
                  {team.map((member) => (
                    <Link
                      key={member.id}
                      href={`/realtor/${member.id}`}
                      style={{
                        background: 'var(--ppt-bg-surface)',
                        borderRadius: 10,
                        padding: 16,
                        textAlign: 'center',
                        textDecoration: 'none',
                        color: 'inherit',
                        boxShadow: '0 1px 3px rgba(0,0,0,.07)',
                      }}
                    >
                      {/* eslint-disable-next-line @next/next/no-img-element */}
                      <img
                        src={member.photoUrl ?? '/avatar-placeholder.png'}
                        alt={member.name}
                        style={{
                          width: 64,
                          height: 64,
                          borderRadius: '50%',
                          objectFit: 'cover',
                          margin: '0 auto 8px',
                          display: 'block',
                        }}
                      />
                      <div style={{ fontWeight: 600, fontSize: '0.9rem' }}>{member.name}</div>
                      {member.title && (
                        <div style={{ fontSize: '0.8125rem', color: 'var(--ppt-fg-secondary)' }}>
                          {member.title}
                        </div>
                      )}
                    </Link>
                  ))}
                </div>
              </section>
            )}

            {/* Active listings */}
            <section>
              <h2 style={{ fontSize: '1.125rem', fontWeight: 700, marginBottom: 16 }}>
                Aktívne ponuky ({listings.length})
              </h2>
              {listings.length === 0 ? (
                <p style={{ color: 'var(--ppt-fg-secondary)' }}>Žiadne aktívne ponuky.</p>
              ) : (
                <div
                  style={{
                    display: 'grid',
                    gridTemplateColumns: 'repeat(auto-fill, minmax(240px, 1fr))',
                    gap: 16,
                  }}
                >
                  {listings.map((listing) => (
                    <Link
                      key={listing.id}
                      href={`/listings/${listing.slug}`}
                      style={{
                        background: 'var(--ppt-bg-surface)',
                        borderRadius: 10,
                        overflow: 'hidden',
                        textDecoration: 'none',
                        color: 'inherit',
                        boxShadow: '0 1px 3px rgba(0,0,0,.07)',
                      }}
                    >
                      {/* eslint-disable-next-line @next/next/no-img-element */}
                      <img
                        src={listing.primaryPhotoUrl ?? '/listing-placeholder.png'}
                        alt={listing.title}
                        style={{ width: '100%', height: 160, objectFit: 'cover' }}
                      />
                      <div style={{ padding: '12px 14px' }}>
                        <div style={{ fontWeight: 600, fontSize: '0.9rem', marginBottom: 4 }}>
                          {listing.title}
                        </div>
                        <div style={{ fontWeight: 700, fontSize: '0.9375rem' }}>
                          {listing.price.toLocaleString('sk-SK')} {listing.currency}
                          {listing.transactionType === 'rent' ? '/mes' : ''}
                        </div>
                      </div>
                    </Link>
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
                padding: 24,
                boxShadow: '0 1px 4px rgba(0,0,0,.07)',
                position: 'sticky',
                top: 24,
              }}
            >
              <h2 style={{ fontSize: '1rem', fontWeight: 700, margin: '0 0 16px' }}>Kontakt</h2>
              <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
                {agency.phone && (
                  <a
                    href={`tel:${agency.phone}`}
                    style={{ color: 'inherit', textDecoration: 'none' }}
                  >
                    📞 {agency.phone}
                  </a>
                )}
                <a
                  href={`mailto:${agency.email}`}
                  style={{ color: 'inherit', textDecoration: 'none', wordBreak: 'break-all' }}
                >
                  ✉️ {agency.email}
                </a>
                {agency.website && (
                  <a
                    href={agency.website}
                    target="_blank"
                    rel="noreferrer"
                    style={{ color: 'var(--ppt-color-primary, #2563eb)' }}
                  >
                    🌐 Navštíviť web
                  </a>
                )}
              </div>
              {agency.licenseNumber && (
                <div
                  style={{
                    marginTop: 16,
                    fontSize: '0.8125rem',
                    color: 'var(--ppt-fg-muted, #9ca3af)',
                  }}
                >
                  Licencia: {agency.licenseNumber}
                </div>
              )}
            </div>
          </aside>
        </div>
      </main>

      <Footer />
    </div>
  );
}
