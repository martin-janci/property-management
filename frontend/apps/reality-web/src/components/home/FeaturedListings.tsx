/**
 * FeaturedListings Component
 *
 * Featured listings section for homepage (Epic 44, Story 44.1).
 */

'use client';

import type { ListingSummary } from '@ppt/reality-api-client';
import { useFeaturedListings, useToggleFavorite } from '@ppt/reality-api-client';
import Link from 'next/link';
import { useTranslations } from 'next-intl';
import { ListingCard } from '../listings/ListingCard';

export function FeaturedListings() {
  const t = useTranslations('home');
  const { data, isLoading, error } = useFeaturedListings();
  const toggleFavorite = useToggleFavorite();

  const handleToggleFavorite = (listingId: string, isFavorite: boolean) => {
    toggleFavorite.mutate({ listingId, isFavorite });
  };

  if (isLoading) {
    // Skeleton renders the same three-section layout the real component
    // produces below (sale / rent / new). Each card mirrors ListingCard's
    // 4:3 image + content footer so the swap to real listings is invisible.
    return (
      <>
        {[0, 1, 2].map((s) => (
          <section
            key={`skel-section-${s}`}
            className={`featured-section ${s % 2 === 1 ? 'alt' : ''}`}
          >
            <div className="container">
              <div className="section-header">
                <div className="skeleton-title" />
                <div className="skeleton-link" />
              </div>
              <div className="grid">
                {[1, 2, 3, 4].map((i) => (
                  <div key={`skel-card-${s}-${i}`} className="skeleton-card">
                    <div className="skeleton-image" />
                    <div className="skeleton-body">
                      <div className="skeleton-line skeleton-price" />
                      <div className="skeleton-line skeleton-title-line" />
                      <div className="skeleton-line skeleton-meta" />
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </section>
        ))}
        <style jsx>{`
          .featured-section { padding: 64px 16px; background: var(--ppt-bg-app); }
          .featured-section.alt { background: var(--ppt-bg-surface); }
          .container { max-width: 1280px; margin: 0 auto; }
          .section-header {
            display: flex; justify-content: space-between; align-items: center;
            margin-bottom: 32px;
          }
          .skeleton-title {
            height: 28px; width: 200px; border-radius: 6px;
            background: var(--ppt-border-default); opacity: 0.6;
          }
          .skeleton-link {
            height: 16px; width: 70px; border-radius: 4px;
            background: var(--ppt-border-default); opacity: 0.5;
          }
          .grid {
            display: grid;
            grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
            gap: 24px;
          }
          .skeleton-card {
            background: var(--ppt-bg-surface);
            border: 1px solid var(--ppt-border-default);
            border-radius: var(--ppt-radius-lg, 12px);
            overflow: hidden;
            opacity: 0.6;
          }
          .skeleton-image {
            aspect-ratio: 4 / 3;
            background: var(--ppt-border-default);
          }
          .skeleton-body { padding: 12px 14px 14px; display: flex; flex-direction: column; gap: 8px; }
          .skeleton-line { background: var(--ppt-border-default); border-radius: 4px; height: 12px; }
          .skeleton-price { height: 18px; width: 50%; }
          .skeleton-title-line { width: 80%; }
          .skeleton-meta { width: 40%; height: 10px; }
        `}</style>
      </>
    );
  }

  if (error || !data) {
    return null;
  }

  const sections: { title: string; listings: ListingSummary[]; link: string }[] = [
    {
      title: t('featuredForSale'),
      listings: data.sale,
      link: '/listings?transactionType=sale',
    },
    {
      title: t('featuredForRent'),
      listings: data.rent,
      link: '/listings?transactionType=rent',
    },
    {
      title: t('newListings'),
      listings: data.new,
      link: '/listings?sortBy=createdAt&sortOrder=desc',
    },
  ];

  // Empty-state: no featured listings at all (e.g. fresh DB with no inventory).
  // Without this CTA the homepage was hero → footer with nothing in between
  // because each section's `length > 0` filter hid them all.
  const allEmpty = sections.every((s) => s.listings.length === 0);
  if (allEmpty) {
    return (
      <section className="featured-section">
        <div className="container">
          <div className="empty-state">
            <div className="empty-icon" aria-hidden="true">
              🔍
            </div>
            <h2 className="empty-title">Práve teraz nemáme odporúčané ponuky</h2>
            <p className="empty-body">
              Naše inzeráty sa pridávajú každý deň. Prejdite na všetky aktuálne ponuky alebo
              pridajte vlastný inzerát zadarmo.
            </p>
            <div className="empty-actions">
              <Link href="/listings" className="empty-cta-primary">
                Prehliadnuť všetky ponuky
              </Link>
              <Link href="/sell" className="empty-cta-secondary">
                Pridať vlastný inzerát
              </Link>
            </div>
          </div>
        </div>
        <style jsx>{`
          .featured-section { padding: 64px 16px; background: var(--ppt-bg-app); }
          .container { max-width: 720px; margin: 0 auto; }
          .empty-state {
            text-align: center;
            background: var(--ppt-bg-surface);
            border: 1px solid var(--ppt-border-default);
            border-radius: 12px;
            padding: 48px 32px;
          }
          .empty-icon { font-size: 48px; margin-bottom: 12px; }
          .empty-title {
            font-size: 1.5rem; font-weight: 700; color: var(--ppt-fg-primary);
            margin: 0 0 12px;
          }
          .empty-body {
            font-size: 1rem; color: var(--ppt-fg-secondary); line-height: 1.6;
            margin: 0 auto 24px; max-width: 520px;
          }
          .empty-actions { display: flex; gap: 12px; justify-content: center; flex-wrap: wrap; }
          .empty-cta-primary, .empty-cta-secondary {
            padding: 12px 20px; border-radius: 8px; font-weight: 600;
            text-decoration: none; transition: opacity .15s;
          }
          .empty-cta-primary {
            background: var(--ppt-color-primary, #2563eb);
            color: var(--ppt-fg-on-accent, #fff);
          }
          .empty-cta-primary:hover { opacity: .9; }
          .empty-cta-secondary {
            background: transparent;
            color: var(--ppt-color-primary, #2563eb);
            border: 1px solid var(--ppt-color-primary, #2563eb);
          }
          .empty-cta-secondary:hover { background: var(--ppt-color-primary-light, #dbeafe); }
        `}</style>
      </section>
    );
  }

  return (
    <>
      {sections.map(
        (section) =>
          section.listings.length > 0 && (
            <section key={section.title} className="featured-section">
              <div className="container">
                <div className="section-header">
                  <h2 className="section-title">{section.title}</h2>
                  <Link href={section.link} className="view-all-link">
                    {t('viewAll')}
                    <svg
                      width="16"
                      height="16"
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      strokeWidth="2"
                      aria-hidden="true"
                    >
                      <path d="M5 12h14M12 5l7 7-7 7" />
                    </svg>
                  </Link>
                </div>
                <div className="grid">
                  {section.listings.slice(0, 4).map((listing) => (
                    <ListingCard
                      key={listing.id}
                      listing={listing}
                      onToggleFavorite={handleToggleFavorite}
                    />
                  ))}
                </div>
              </div>

              <style jsx>{`
                .featured-section {
                  padding: 64px 16px;
                  background: var(--ppt-bg-app);
                }

                .featured-section:nth-child(even) {
                  background: var(--ppt-bg-surface);
                }

                .container {
                  max-width: 1280px;
                  margin: 0 auto;
                }

                .section-header {
                  display: flex;
                  justify-content: space-between;
                  align-items: center;
                  margin-bottom: 32px;
                }

                .section-title {
                  font-size: 1.5rem;
                  font-weight: bold;
                  color: var(--ppt-fg-primary);
                  margin: 0;
                }

                .view-all-link {
                  display: flex;
                  align-items: center;
                  gap: 4px;
                  color: var(--ppt-color-primary);
                  text-decoration: none;
                  font-weight: 500;
                  font-size: 14px;
                }

                .view-all-link:hover {
                  text-decoration: underline;
                }

                .grid {
                  display: grid;
                  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
                  gap: 24px;
                }
              `}</style>
            </section>
          )
      )}
    </>
  );
}
