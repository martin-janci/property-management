/**
 * Saved Searches Page
 *
 * User's saved search filters with alert settings (Epic 44, Story 44.5).
 */

'use client';

import type { SavedSearch } from '@ppt/reality-api-client';
import {
  useDeleteSavedSearch,
  useSavedSearches,
  useToggleSearchAlert,
} from '@ppt/reality-api-client';
import Link from 'next/link';
import { useTranslations } from 'next-intl';
import { useState } from 'react';
import { ProtectedRoute } from '@/components/auth';
import { Footer, Header } from '@/components/ui';

function SavedSearchCard({ search }: { search: SavedSearch }) {
  const t = useTranslations('pages.savedSearches');
  const deleteSearch = useDeleteSavedSearch();
  const toggleAlert = useToggleSearchAlert();
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);

  const handleToggleAlert = () => {
    toggleAlert.mutate({ id: search.id, enabled: !search.alertsEnabled });
  };

  const handleDelete = () => {
    deleteSearch.mutate(search.id);
    setShowDeleteConfirm(false);
  };

  const getFilterSummary = () => {
    const parts: string[] = [];

    if (search.filters.transactionType) {
      parts.push(search.filters.transactionType === 'sale' ? t('forSale') : t('forRent'));
    }

    if (search.filters.propertyType?.length) {
      parts.push(search.filters.propertyType.join(', '));
    }

    if (search.filters.city) {
      parts.push(search.filters.city);
    }

    if (search.filters.priceMin || search.filters.priceMax) {
      const min = search.filters.priceMin?.toLocaleString() ?? '0';
      const max = search.filters.priceMax?.toLocaleString() ?? '+';
      parts.push(`€${min} - €${max}`);
    }

    return parts.length > 0 ? parts.join(' • ') : t('allListings');
  };

  const searchUrl = `/listings?${new URLSearchParams(
    Object.entries(search.filters)
      .filter(([, v]) => v !== undefined)
      .map(([k, v]) => [k, Array.isArray(v) ? v.join(',') : String(v)])
  ).toString()}`;

  return (
    <div className="search-card">
      <div className="card-header">
        <div className="card-info">
          <h3 className="card-title">{search.name}</h3>
          <p className="card-summary">{getFilterSummary()}</p>
        </div>
        <Link href={searchUrl} className="view-button">
          {t('view')}
        </Link>
      </div>

      <div className="card-footer">
        <div className="alert-toggle">
          <label className="toggle-label">
            <input
              type="checkbox"
              checked={search.alertsEnabled}
              onChange={handleToggleAlert}
              className="toggle-checkbox"
              disabled={toggleAlert.isPending}
            />
            <span className="toggle-text">
              {search.alertsEnabled ? t('alertsOn') : t('alertsOff')}
            </span>
          </label>
          {search.alertsEnabled && search.alertFrequency && (
            <span className="frequency-badge">{t(`frequency.${search.alertFrequency}`)}</span>
          )}
        </div>

        <div className="card-actions">
          {search.newListingsCount !== undefined && search.newListingsCount > 0 && (
            <span className="new-badge">{t('newCount', { count: search.newListingsCount })}</span>
          )}
          {showDeleteConfirm ? (
            <div className="delete-confirm">
              <span>{t('deleteConfirm')}</span>
              <button type="button" className="confirm-yes" onClick={handleDelete}>
                {t('confirmYes')}
              </button>
              <button
                type="button"
                className="confirm-no"
                onClick={() => setShowDeleteConfirm(false)}
              >
                {t('confirmNo')}
              </button>
            </div>
          ) : (
            <button
              type="button"
              className="delete-button"
              onClick={() => setShowDeleteConfirm(true)}
              aria-label={t('deleteAriaLabel')}
            >
              <svg
                width="16"
                height="16"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                aria-hidden="true"
              >
                <polyline points="3 6 5 6 21 6" />
                <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
              </svg>
            </button>
          )}
        </div>
      </div>

      <style jsx>{`
        .search-card {
          background: var(--ppt-bg-surface);
          border-radius: 12px;
          padding: 20px;
          box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
        }

        .card-header {
          display: flex;
          justify-content: space-between;
          align-items: flex-start;
          gap: 16px;
          margin-bottom: 16px;
        }

        .card-title {
          font-size: 1rem;
          font-weight: 600;
          color: var(--ppt-fg-primary);
          margin: 0 0 4px;
        }

        .card-summary {
          font-size: 14px;
          color: var(--ppt-fg-muted);
          margin: 0;
        }

        .view-button {
          padding: 6px 16px;
          background: var(--ppt-color-primary);
          color: var(--ppt-fg-on-accent);
          text-decoration: none;
          border-radius: 6px;
          font-size: 14px;
          font-weight: 500;
          flex-shrink: 0;
        }

        .view-button:hover {
          background: var(--ppt-color-primary-hover);
        }

        .card-footer {
          display: flex;
          justify-content: space-between;
          align-items: center;
          padding-top: 16px;
          border-top: 1px solid var(--ppt-bg-subtle);
        }

        .alert-toggle {
          display: flex;
          align-items: center;
          gap: 8px;
        }

        .toggle-label {
          display: flex;
          align-items: center;
          gap: 8px;
          cursor: pointer;
        }

        .toggle-checkbox {
          width: 16px;
          height: 16px;
          accent-color: var(--ppt-color-primary);
        }

        .toggle-text {
          font-size: 14px;
          color: var(--ppt-fg-secondary);
        }

        .frequency-badge {
          padding: 2px 8px;
          background: #e0e7ff;
          color: #4f46e5;
          border-radius: 4px;
          font-size: 12px;
          font-weight: 500;
          text-transform: capitalize;
        }

        .card-actions {
          display: flex;
          align-items: center;
          gap: 12px;
        }

        .new-badge {
          padding: 2px 8px;
          background: var(--ppt-color-success-light);
          color: var(--ppt-color-success);
          border-radius: 4px;
          font-size: 12px;
          font-weight: 500;
        }

        .delete-button {
          padding: 6px;
          background: transparent;
          border: none;
          color: var(--ppt-fg-subtle);
          cursor: pointer;
          border-radius: 4px;
        }

        .delete-button:hover {
          color: var(--ppt-color-danger-hover);
          background: var(--ppt-color-danger-light);
        }

        .delete-confirm {
          display: flex;
          align-items: center;
          gap: 8px;
          font-size: 14px;
          color: var(--ppt-fg-secondary);
        }

        .confirm-yes,
        .confirm-no {
          padding: 4px 8px;
          border: none;
          border-radius: 4px;
          font-size: 12px;
          cursor: pointer;
        }

        .confirm-yes {
          background: var(--ppt-color-danger-hover);
          color: var(--ppt-fg-on-accent);
        }

        .confirm-no {
          background: var(--ppt-border-default);
          color: var(--ppt-fg-secondary);
        }
      `}</style>
    </div>
  );
}

function SavedSearchesContent() {
  const t = useTranslations('pages.savedSearches');
  const { data: searches, isLoading, error } = useSavedSearches();

  if (isLoading) {
    return (
      <div className="searches-list loading">
        {[1, 2, 3].map((i) => (
          <div key={`search-skeleton-${i}`} className="skeleton-card" />
        ))}
        <style jsx>{`
          .searches-list {
            display: flex;
            flex-direction: column;
            gap: 16px;
          }
          .skeleton-card {
            height: 120px;
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

  if (!searches || searches.length === 0) {
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
          <circle cx="11" cy="11" r="8" />
          <path d="m21 21-4.35-4.35" />
          <path d="M11 8v6M8 11h6" />
        </svg>
        <h2 className="empty-title">{t('emptyTitle')}</h2>
        <p className="empty-text">{t('emptyText')}</p>
        <Link href="/listings" className="browse-link">
          {t('startSearching')}
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
    <div className="searches-list">
      {searches.map((search) => (
        <SavedSearchCard key={search.id} search={search} />
      ))}
      <style jsx>{`
        .searches-list {
          display: flex;
          flex-direction: column;
          gap: 16px;
          max-width: 800px;
        }
      `}</style>
    </div>
  );
}

export default function SavedSearchesPage() {
  const t = useTranslations('pages.savedSearches');
  return (
    <div className="page-container">
      <Header />
      <main className="main">
        <div className="container">
          <h1 className="page-title">{t('title')}</h1>
          <p className="page-subtitle">{t('subtitle')}</p>
          <ProtectedRoute>
            <SavedSearchesContent />
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
        .page-title {
          font-size: 2rem;
          font-weight: bold;
          color: var(--ppt-fg-primary);
          margin: 0 0 8px;
        }
        .page-subtitle {
          color: var(--ppt-fg-muted);
          margin: 0 0 32px;
        }
      `}</style>
    </div>
  );
}
