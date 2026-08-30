/**
 * Favorites Page
 *
 * User's saved favorite listings (Epic 44, Story 44.5).
 */

'use client';

import { useFavorites, useRemoveFavorite, useUpdateFavorite } from '@ppt/reality-api-client';
import { useTranslations } from 'next-intl';
import { useState } from 'react';
import { ProtectedRoute } from '@/components/auth';
import { ListingCard } from '@/components/listings';
import { Footer, Header } from '@/components/ui';
import { Link } from '@/i18n/routing';

function FavoritesContent() {
  const t = useTranslations('pages.favorites');
  const [page, setPage] = useState(1);
  const { data, isLoading, error } = useFavorites(page, 12);
  const removeFavorite = useRemoveFavorite();
  const updateFavorite = useUpdateFavorite();

  const handleRemoveFavorite = (listingId: string) => {
    removeFavorite.mutate(listingId);
  };

  const handleToggleWatch = (listingId: string, enabled: boolean) => {
    updateFavorite.mutate({ listingId, data: { price_alert_enabled: enabled } });
  };

  if (isLoading) {
    return (
      <div className="favorites-grid loading">
        {[1, 2, 3, 4, 5, 6].map((i) => (
          <div key={`fav-skeleton-${i}`} className="skeleton-card" />
        ))}
        <style jsx>{`
          .favorites-grid {
            display: grid;
            grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
            gap: 24px;
          }
          .skeleton-card {
            height: 320px;
            background: var(--ppt-border-default);
            border-radius: 12px;
            animation: pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
          }
          @keyframes pulse {
            0%, 100% { opacity: 1; }
            50% { opacity: 0.5; }
          }
        `}</style>
      </div>
    );
  }

  if (error) {
    return (
      <div className="error-state">
        <p>{t('error')}</p>
        <style jsx>{`
          .error-state {
            padding: 64px 24px;
            text-align: center;
            color: var(--ppt-color-danger-hover);
          }
        `}</style>
      </div>
    );
  }

  if (!data || data.data.length === 0) {
    return (
      <div className="empty-state">
        <svg
          width="64"
          height="64"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="1"
          aria-hidden="true"
        >
          <path d="M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z" />
        </svg>
        <h2 className="empty-title">{t('emptyTitle')}</h2>
        <p className="empty-text">{t('emptyText')}</p>
        <Link href="/listings" className="browse-link">
          {t('browseListings')}
        </Link>
        <style jsx>{`
          .empty-state {
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            padding: 64px 24px;
            text-align: center;
            color: var(--ppt-fg-muted);
          }
          .empty-title {
            font-size: 1.5rem;
            font-weight: 600;
            color: var(--ppt-fg-primary);
            margin: 24px 0 8px;
          }
          .empty-text {
            margin: 0 0 24px;
            max-width: 400px;
          }
          .browse-link {
            padding: 12px 24px;
            background: var(--ppt-color-primary);
            color: var(--ppt-fg-on-accent);
            text-decoration: none;
            border-radius: 8px;
            font-weight: 600;
          }
          .browse-link:hover {
            background: var(--ppt-color-primary-hover);
          }
        `}</style>
      </div>
    );
  }

  return (
    <>
      <div className="favorites-grid">
        {data.data.map((favorite) => {
          // Backend default for `price_alert_enabled` is `true`; treat an
          // absent field as watching so the toggle reflects server default.
          const watching = favorite.price_alert_enabled ?? true;
          return (
            <div key={favorite.id} className="favorite-cell">
              <ListingCard
                listing={{ ...favorite.listing, isFavorite: true }}
                onToggleFavorite={() => handleRemoveFavorite(favorite.listingId)}
              />
              <label className="watch-toggle">
                <input
                  type="checkbox"
                  checked={watching}
                  disabled={
                    updateFavorite.isPending &&
                    updateFavorite.variables?.listingId === favorite.listingId
                  }
                  onChange={(e) => handleToggleWatch(favorite.listingId, e.target.checked)}
                />
                <span>{t('watchPrice')}</span>
              </label>
            </div>
          );
        })}
      </div>

      {data.totalPages > 1 && (
        <div className="pagination">
          <button
            type="button"
            disabled={page === 1}
            onClick={() => setPage((p) => p - 1)}
            className="page-button"
          >
            Previous
          </button>
          <span className="page-info">
            Page {page} of {data.totalPages}
          </span>
          <button
            type="button"
            disabled={page >= data.totalPages}
            onClick={() => setPage((p) => p + 1)}
            className="page-button"
          >
            Next
          </button>
        </div>
      )}

      <style jsx>{`
        .favorites-grid {
          display: grid;
          grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
          gap: 24px;
        }
        .favorite-cell {
          display: flex;
          flex-direction: column;
          gap: 8px;
        }
        .watch-toggle {
          display: inline-flex;
          align-items: center;
          gap: 8px;
          font-size: 13px;
          color: var(--ppt-fg-muted);
          cursor: pointer;
        }
        .watch-toggle input {
          cursor: pointer;
        }
        .watch-toggle input:disabled {
          cursor: not-allowed;
        }
        .pagination {
          display: flex;
          justify-content: center;
          align-items: center;
          gap: 16px;
          margin-top: 32px;
        }
        .page-button {
          padding: 8px 16px;
          background: var(--ppt-bg-surface);
          border: 1px solid var(--ppt-border-default);
          border-radius: 8px;
          font-size: 14px;
          cursor: pointer;
        }
        .page-button:disabled {
          opacity: 0.5;
          cursor: not-allowed;
        }
        .page-button:hover:not(:disabled) {
          background: var(--ppt-bg-app);
        }
        .page-info {
          font-size: 14px;
          color: var(--ppt-fg-muted);
        }
      `}</style>
    </>
  );
}

export default function FavoritesPage() {
  const t = useTranslations('pages.favorites');
  return (
    <div className="page-container">
      <Header />
      <main className="main">
        <div className="container">
          <div className="page-head">
            <h1 className="page-title">{t('h1')}</h1>
            <Link href="/favorites/alerts" className="alerts-link">
              {t('priceAlertsLink')}
            </Link>
          </div>
          <ProtectedRoute>
            <FavoritesContent />
          </ProtectedRoute>
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
          padding: 32px 0;
        }
        .container {
          max-width: 1280px;
          margin: 0 auto;
          padding: 0 16px;
        }
        .page-head {
          display: flex;
          align-items: center;
          justify-content: space-between;
          gap: 16px;
          flex-wrap: wrap;
          margin: 0 0 32px;
        }
        .page-title {
          font-size: 2rem;
          font-weight: bold;
          color: var(--ppt-fg-primary);
          margin: 0;
        }
        .alerts-link {
          padding: 8px 16px;
          background: var(--ppt-bg-surface);
          border: 1px solid var(--ppt-border-default);
          border-radius: 8px;
          font-size: 14px;
          font-weight: 600;
          color: var(--ppt-fg-primary);
          text-decoration: none;
        }
        .alerts-link:hover { background: var(--ppt-bg-app); }
      `}</style>
    </div>
  );
}
