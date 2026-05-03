/**
 * FeaturedListings Component
 *
 * Featured listings section for homepage (Epic 44, Story 44.1).
 */

'use client';

import type { ListingSummary } from '@ppt/reality-api-client';
import { useFeaturedListings, useToggleFavorite } from '@ppt/reality-api-client';
import { useTranslations } from 'next-intl';
import Link from 'next/link';
import { ListingCard } from '../listings/ListingCard';

export function FeaturedListings() {
  const t = useTranslations('home');
  const { data, isLoading, error } = useFeaturedListings();
  const toggleFavorite = useToggleFavorite();

  const handleToggleFavorite = (listingId: string, isFavorite: boolean) => {
    toggleFavorite.mutate({ listingId, isFavorite });
  };

  if (isLoading) {
    return (
      <section className="featured-section">
        <div className="container">
          <div className="skeleton-header" />
          <div className="grid">
            {[1, 2, 3, 4].map((i) => (
              <div key={`skeleton-${i}`} className="skeleton-card" />
            ))}
          </div>
        </div>
        <style jsx>{`
          .featured-section {
            padding: 64px 16px;
            background: var(--ppt-bg-app);
          }
          .container {
            max-width: 1280px;
            margin: 0 auto;
          }
          .skeleton-header {
            height: 40px;
            width: 300px;
            background: var(--ppt-border-default);
            border-radius: 8px;
            margin-bottom: 32px;
          }
          .grid {
            display: grid;
            grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
            gap: 24px;
          }
          .skeleton-card {
            height: 320px;
            background: var(--ppt-border-default);
            border-radius: 12px;
          }
        `}</style>
      </section>
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
