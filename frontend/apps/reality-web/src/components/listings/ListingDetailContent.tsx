/**
 * ListingDetailContent Component
 *
 * Client-side presentation component for listing detail (Epic 44, Story 44.3).
 */

'use client';

import type { ListingDetail, ListingFeatures } from '@ppt/reality-api-client';
import { useTranslations } from 'next-intl';
import { Footer, Header } from '@/components/ui';
import { ContactForm } from './ContactForm';
import { PhotoGallery } from './PhotoGallery';

interface ListingDetailContentProps {
  listing: ListingDetail | null;
  jsonLd?: object;
}

function formatPrice(price: number, currency: string) {
  const value = Number.isFinite(price) ? price : 0;
  // A partial 200 body can omit `currency`; `Intl.NumberFormat` with
  // `style: 'currency'` throws ("Currency code is required…") on a missing
  // code, which would crash SSR. Fall back to a plain decimal format.
  if (typeof currency !== 'string' || currency.length === 0) {
    return new Intl.NumberFormat('en-US', { maximumFractionDigits: 0 }).format(value);
  }
  return new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency: currency,
    maximumFractionDigits: 0,
  }).format(value);
}

export function ListingNotFound() {
  const t = useTranslations('listing');
  return (
    <div className="page-container">
      <Header />
      <main className="main">
        <div className="not-found">
          <h1>{t('notFound')}</h1>
          <p>{t('notFoundDesc')}</p>
          <a href="/listings" className="back-link">
            {t('browseAll')}
          </a>
        </div>
      </main>
      <Footer />
      <style jsx>{`
        .page-container {
          min-height: 100vh;
          display: flex;
          flex-direction: column;
        }
        .main {
          flex: 1;
          display: flex;
          align-items: center;
          justify-content: center;
        }
        .not-found {
          text-align: center;
          padding: 64px 16px;
        }
        .not-found h1 {
          font-size: 2rem;
          color: var(--ppt-fg-primary);
          margin: 0 0 16px;
        }
        .not-found p {
          color: var(--ppt-fg-muted);
          margin: 0 0 24px;
        }
        .back-link {
          color: var(--ppt-color-primary);
          text-decoration: none;
        }
        .back-link:hover {
          text-decoration: underline;
        }
      `}</style>
    </div>
  );
}

export function ListingDetailContent({ listing, jsonLd }: ListingDetailContentProps) {
  const t = useTranslations('listing');
  const tFeatures = useTranslations('features');
  const tNav = useTranslations('nav');

  if (!listing) {
    return <ListingNotFound />;
  }

  const getFeatureLabel = (key: keyof ListingFeatures): string => {
    return tFeatures(key);
  };

  // `listing.features` / `listing.photos` are typed as present, but this
  // component is exported and a partial/malformed 200 body (from getListing's
  // upstream, or another caller) can carry *wrong-typed* values, not just
  // null/undefined. `features: "x"` makes `Object.entries` emit per-character
  // garbage entries, and a non-array `photos` (e.g. `{}`) makes `PhotoGallery`
  // throw on `.length` / `.map` — the same SSR-500 this guard exists to
  // prevent (#2276 / #2341). Type-guard both, not just nullish-coalesce.
  const featuresObj =
    listing.features && typeof listing.features === 'object' && !Array.isArray(listing.features)
      ? listing.features
      : {};
  const activeFeatures = Object.entries(featuresObj)
    .filter(([, value]) => value === true)
    .map(([key]) => key as keyof ListingFeatures);

  const photos = Array.isArray(listing.photos) ? listing.photos : [];

  return (
    <div className="page-container">
      {/* JSON-LD structured data for SEO */}
      {jsonLd && (
        <script
          type="application/ld+json"
          // biome-ignore lint/security/noDangerouslySetInnerHtml: JSON-LD for SEO requires dangerouslySetInnerHTML
          dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLd) }}
        />
      )}
      <Header />
      <main className="main">
        <div className="container">
          {/* Breadcrumb */}
          <nav className="breadcrumb" aria-label="Breadcrumb">
            <a href="/">{tNav('home')}</a>
            <span className="separator">/</span>
            <a href="/listings">{tNav('allListings')}</a>
            <span className="separator">/</span>
            <span className="current">{listing.address?.city}</span>
          </nav>

          <div className="content-grid">
            {/* Main Content */}
            <div className="main-content">
              {/* Photo Gallery */}
              <PhotoGallery photos={photos} title={listing.title} />

              {/* Header */}
              <div className="listing-header">
                <div className="badges">
                  <span className={`badge ${listing.transactionType}`}>
                    {listing.transactionType === 'sale' ? t('forSale') : t('forRent')}
                  </span>
                  <span className="badge type">{listing.propertyType}</span>
                </div>
                <h1 className="title">{listing.title}</h1>
                <p className="address">
                  <svg
                    width="18"
                    height="18"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    aria-hidden="true"
                  >
                    <path d="M21 10c0 7-9 13-9 13s-9-6-9-13a9 9 0 0 1 18 0z" />
                    <circle cx="12" cy="10" r="3" />
                  </svg>
                  {listing.address?.street && `${listing.address.street}, `}
                  {listing.address?.city}
                  {listing.address?.district && `, ${listing.address.district}`}
                </p>
                <div className="price-row">
                  <span className="price">{formatPrice(listing.price, listing.currency)}</span>
                  {listing.transactionType === 'rent' && (
                    <span className="price-suffix">{t('perMonth')}</span>
                  )}
                  {listing.pricePerSqm && (
                    <span className="price-per-sqm">
                      ({formatPrice(listing.pricePerSqm, listing.currency)}/m²)
                    </span>
                  )}
                </div>
              </div>

              {/* Key Details */}
              <div className="key-details">
                {listing.rooms !== undefined && (
                  <div className="detail-item">
                    <span className="detail-value">{listing.rooms}</span>
                    <span className="detail-label">{t('rooms')}</span>
                  </div>
                )}
                {listing.bedrooms !== undefined && (
                  <div className="detail-item">
                    <span className="detail-value">{listing.bedrooms}</span>
                    <span className="detail-label">{t('bedrooms')}</span>
                  </div>
                )}
                {listing.bathrooms !== undefined && (
                  <div className="detail-item">
                    <span className="detail-value">{listing.bathrooms}</span>
                    <span className="detail-label">{t('bathrooms')}</span>
                  </div>
                )}
                <div className="detail-item">
                  <span className="detail-value">{listing.area}</span>
                  <span className="detail-label">{t('sqm')}</span>
                </div>
                {listing.floor !== undefined && (
                  <div className="detail-item">
                    <span className="detail-value">
                      {listing.floor}
                      {listing.totalFloors && `/${listing.totalFloors}`}
                    </span>
                    <span className="detail-label">{t('floor')}</span>
                  </div>
                )}
                {listing.yearBuilt !== undefined && (
                  <div className="detail-item">
                    <span className="detail-value">{listing.yearBuilt}</span>
                    <span className="detail-label">{t('built')}</span>
                  </div>
                )}
              </div>

              {/* Description */}
              <section className="section">
                <h2 className="section-title">{t('description')}</h2>
                <p className="description">{listing.description}</p>
              </section>

              {/* Features */}
              {activeFeatures.length > 0 && (
                <section className="section">
                  <h2 className="section-title">{t('features')}</h2>
                  <div className="features-grid">
                    {activeFeatures.map((feature) => (
                      <div key={feature} className="feature-item">
                        <svg
                          width="16"
                          height="16"
                          viewBox="0 0 24 24"
                          fill="none"
                          stroke="currentColor"
                          strokeWidth="2"
                          aria-hidden="true"
                        >
                          <polyline points="20 6 9 17 4 12" />
                        </svg>
                        <span>{getFeatureLabel(feature)}</span>
                      </div>
                    ))}
                  </div>
                </section>
              )}

              {/* Additional Info */}
              <section className="section">
                <h2 className="section-title">{t('additionalInfo')}</h2>
                <div className="info-grid">
                  {listing.energyRating && (
                    <div className="info-item">
                      <span className="info-label">{t('energyRating')}</span>
                      <span className="info-value">{listing.energyRating}</span>
                    </div>
                  )}
                  {listing.monthlyCharges !== undefined && (
                    <div className="info-item">
                      <span className="info-label">{t('monthlyCharges')}</span>
                      <span className="info-value">
                        {formatPrice(listing.monthlyCharges, listing.currency)}
                      </span>
                    </div>
                  )}
                  {listing.availableFrom && (
                    <div className="info-item">
                      <span className="info-label">{t('availableFrom')}</span>
                      <span className="info-value">
                        {new Date(listing.availableFrom).toLocaleDateString()}
                      </span>
                    </div>
                  )}
                  <div className="info-item">
                    <span className="info-label">{t('listed')}</span>
                    <span className="info-value">
                      {new Date(listing.createdAt).toLocaleDateString()}
                    </span>
                  </div>
                </div>
              </section>

              {/* Virtual Tour / Floor Plan */}
              {(listing.virtualTourUrl || listing.floorPlanUrl) && (
                <section className="section">
                  <h2 className="section-title">{t('additionalResources')}</h2>
                  <div className="resources">
                    {listing.virtualTourUrl && (
                      <a
                        href={listing.virtualTourUrl}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="resource-link"
                      >
                        <svg
                          width="20"
                          height="20"
                          viewBox="0 0 24 24"
                          fill="none"
                          stroke="currentColor"
                          strokeWidth="2"
                          aria-hidden="true"
                        >
                          <circle cx="12" cy="12" r="10" />
                          <polygon points="10 8 16 12 10 16 10 8" />
                        </svg>
                        {t('virtualTour')}
                      </a>
                    )}
                    {listing.floorPlanUrl && (
                      <a
                        href={listing.floorPlanUrl}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="resource-link"
                      >
                        <svg
                          width="20"
                          height="20"
                          viewBox="0 0 24 24"
                          fill="none"
                          stroke="currentColor"
                          strokeWidth="2"
                          aria-hidden="true"
                        >
                          <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
                          <line x1="3" y1="9" x2="21" y2="9" />
                          <line x1="9" y1="21" x2="9" y2="9" />
                        </svg>
                        {t('floorPlan')}
                      </a>
                    )}
                  </div>
                </section>
              )}
            </div>

            {/* Sidebar */}
            <div className="sidebar">
              {/* `agent` is not guaranteed on a partial 200 body; ContactForm
                  dereferences `agent.name` / `agent.avatarUrl`, so only render
                  it when the agent is present to avoid crashing SSR. */}
              {listing.agent && <ContactForm listingId={listing.id} agent={listing.agent} />}
            </div>
          </div>
        </div>
      </main>
      <Footer />

      <style jsx>{`
        .page-container {
          min-height: 100vh;
          display: flex;
          flex-direction: column;
          background: var(--ppt-bg-app);
        }

        .main {
          flex: 1;
          padding: 24px 0;
        }

        .container {
          max-width: 1280px;
          margin: 0 auto;
          padding: 0 16px;
        }

        .breadcrumb {
          font-size: 14px;
          color: var(--ppt-fg-muted);
          margin-bottom: 24px;
        }

        .breadcrumb a {
          color: var(--ppt-fg-muted);
          text-decoration: none;
        }

        .breadcrumb a:hover {
          color: var(--ppt-color-primary);
        }

        .separator {
          margin: 0 8px;
        }

        .current {
          color: var(--ppt-fg-primary);
        }

        .content-grid {
          display: grid;
          gap: 32px;
        }

        @media (min-width: 1024px) {
          .content-grid {
            grid-template-columns: 1fr 380px;
          }
        }

        .main-content {
          min-width: 0;
        }

        .listing-header {
          margin-top: 24px;
        }

        .badges {
          display: flex;
          gap: 8px;
          margin-bottom: 12px;
        }

        .badge {
          padding: 4px 12px;
          border-radius: 4px;
          font-size: 12px;
          font-weight: 600;
          text-transform: uppercase;
        }

        .badge.sale {
          background: var(--ppt-color-success);
          color: var(--ppt-fg-on-accent);
        }

        .badge.rent {
          background: var(--ppt-brand-500);
          color: var(--ppt-fg-on-accent);
        }

        .badge.type {
          background: var(--ppt-bg-subtle);
          color: var(--ppt-fg-secondary);
        }

        .title {
          font-size: 1.75rem;
          font-weight: bold;
          color: var(--ppt-fg-primary);
          margin: 0 0 12px;
        }

        .address {
          display: flex;
          align-items: center;
          gap: 8px;
          font-size: 1rem;
          color: var(--ppt-fg-muted);
          margin: 0 0 16px;
        }

        .price-row {
          display: flex;
          align-items: baseline;
          gap: 8px;
        }

        .price {
          font-size: 2rem;
          font-weight: bold;
          color: var(--ppt-fg-primary);
        }

        .price-suffix {
          font-size: 1rem;
          color: var(--ppt-fg-muted);
        }

        .price-per-sqm {
          font-size: 14px;
          color: var(--ppt-fg-subtle);
        }

        .key-details {
          display: grid;
          grid-template-columns: repeat(auto-fit, minmax(100px, 1fr));
          gap: 16px;
          padding: 24px;
          background: var(--ppt-bg-surface);
          border-radius: 12px;
          margin-top: 24px;
        }

        .detail-item {
          text-align: center;
        }

        .detail-value {
          display: block;
          font-size: 1.5rem;
          font-weight: 600;
          color: var(--ppt-fg-primary);
        }

        .detail-label {
          font-size: 14px;
          color: var(--ppt-fg-muted);
        }

        .section {
          padding: 24px;
          background: var(--ppt-bg-surface);
          border-radius: 12px;
          margin-top: 24px;
        }

        .section-title {
          font-size: 1.125rem;
          font-weight: 600;
          color: var(--ppt-fg-primary);
          margin: 0 0 16px;
        }

        .description {
          color: var(--ppt-fg-secondary);
          line-height: 1.7;
          white-space: pre-line;
          margin: 0;
        }

        .features-grid {
          display: grid;
          grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
          gap: 12px;
        }

        .feature-item {
          display: flex;
          align-items: center;
          gap: 8px;
          color: var(--ppt-fg-secondary);
        }

        .feature-item svg {
          color: var(--ppt-color-success);
        }

        .info-grid {
          display: grid;
          grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
          gap: 16px;
        }

        .info-item {
          display: flex;
          justify-content: space-between;
          padding: 12px 0;
          border-bottom: 1px solid var(--ppt-bg-subtle);
        }

        .info-label {
          color: var(--ppt-fg-muted);
        }

        .info-value {
          font-weight: 500;
          color: var(--ppt-fg-primary);
        }

        .resources {
          display: flex;
          gap: 16px;
          flex-wrap: wrap;
        }

        .resource-link {
          display: flex;
          align-items: center;
          gap: 8px;
          padding: 12px 20px;
          background: var(--ppt-bg-subtle);
          border-radius: 8px;
          color: var(--ppt-fg-secondary);
          text-decoration: none;
          font-weight: 500;
          transition: background 0.2s;
        }

        .resource-link:hover {
          background: var(--ppt-border-default);
        }

        .sidebar {
          position: sticky;
          top: 88px;
          height: fit-content;
        }
      `}</style>
    </div>
  );
}
