/**
 * AgencyListings Component
 *
 * Manage listings for agency/realtors (Epic 45, Story 45.3).
 */

'use client';

import type { AgencyListing, AgencyListingStatus } from '@ppt/reality-api-client';
import { useAgencyListings, useMyAgency, useRealtors } from '@ppt/reality-api-client';
import Link from 'next/link';
import { useTranslations } from 'next-intl';
import { useState } from 'react';
import {
  AgencyLoadError,
  isNotFoundError,
  NoAgencyMessage,
  SectionError,
} from './AgencyErrorStates';

type StatusFilter = 'all' | AgencyListingStatus;

export function AgencyListings() {
  const t = useTranslations('agencyListings');
  const [statusFilter, setStatusFilter] = useState<StatusFilter>('all');
  const [realtorFilter, setRealtorFilter] = useState<string>('all');
  const [page, setPage] = useState(1);

  const {
    data: agency,
    isLoading: agencyLoading,
    isError: agencyError,
    error: agencyErrorObj,
    refetch: refetchAgency,
  } = useMyAgency();
  const { data: realtors } = useRealtors(agency?.id || '');
  const {
    data: listingsData,
    isLoading,
    isError: listingsError,
  } = useAgencyListings(agency?.id || '', {
    status: statusFilter === 'all' ? undefined : statusFilter,
    realtorId: realtorFilter === 'all' ? undefined : realtorFilter,
    page,
    limit: 20,
  });

  // Distinguish an agency-load failure from a genuine "no agency" state so a
  // transport/server error no longer renders the misleading empty state
  // ("No listings yet") one route over from Issue #2277 (Issue #2343).
  if (agencyError && !isNotFoundError(agencyErrorObj)) {
    return <AgencyLoadError onRetry={() => refetchAgency()} />;
  }
  if (!agencyLoading && !agency) {
    return <NoAgencyMessage />;
  }

  const statusOptions: { value: StatusFilter; label: string }[] = [
    { value: 'all', label: t('status.all') },
    { value: 'active', label: t('status.active') },
    { value: 'draft', label: t('status.draft') },
    { value: 'pending', label: t('status.pending') },
    { value: 'sold', label: t('status.sold') },
    { value: 'rented', label: t('status.rented') },
    { value: 'withdrawn', label: t('status.withdrawn') },
  ];

  return (
    <div className="agency-listings">
      {/* Header */}
      <div className="header">
        <div>
          <Link href="/agency" className="back-link">
            {t('back')}
          </Link>
          <h1 className="title">{t('title')}</h1>
          <p className="subtitle">{t('subtitle')}</p>
        </div>
        <Link href="/listings/create" className="create-button">
          <svg
            width="20"
            height="20"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            aria-hidden="true"
          >
            <line x1="12" y1="5" x2="12" y2="19" />
            <line x1="5" y1="12" x2="19" y2="12" />
          </svg>
          {t('createListing')}
        </Link>
      </div>

      {/* Filters */}
      <div className="filters">
        <div className="filter-group">
          <label htmlFor="status-filter">{t('filterStatus')}</label>
          <select
            id="status-filter"
            value={statusFilter}
            onChange={(e) => {
              setStatusFilter(e.target.value as StatusFilter);
              setPage(1);
            }}
          >
            {statusOptions.map((opt) => (
              <option key={opt.value} value={opt.value}>
                {opt.label}
              </option>
            ))}
          </select>
        </div>

        <div className="filter-group">
          <label htmlFor="realtor-filter">{t('filterRealtor')}</label>
          <select
            id="realtor-filter"
            value={realtorFilter}
            onChange={(e) => {
              setRealtorFilter(e.target.value);
              setPage(1);
            }}
          >
            <option value="all">{t('allRealtors')}</option>
            {realtors?.map((r) => (
              <option key={r.id} value={r.id}>
                {r.name}
              </option>
            ))}
          </select>
        </div>

        <div className="filter-stats">
          {listingsData && (
            <span>
              {t('showing', {
                count: listingsData.listings.length,
                total: listingsData.total,
              })}
            </span>
          )}
        </div>
      </div>

      {/* Listings Table */}
      <div className="table-container">
        {isLoading ? (
          <ListingsTableSkeleton />
        ) : listingsError ? (
          <SectionError message={t('loadError')} />
        ) : listingsData?.listings.length === 0 ? (
          <EmptyState />
        ) : (
          <table className="listings-table">
            <thead>
              <tr>
                <th>{t('colProperty')}</th>
                <th>{t('colType')}</th>
                <th>{t('colPrice')}</th>
                <th>{t('colStatus')}</th>
                <th>{t('colRealtor')}</th>
                <th>{t('colViews')}</th>
                <th>{t('colInquiries')}</th>
                <th>{t('colUpdated')}</th>
                <th>{t('colActions')}</th>
              </tr>
            </thead>
            <tbody>
              {listingsData?.listings.map((listing) => (
                <ListingRow key={listing.id} listing={listing} />
              ))}
            </tbody>
          </table>
        )}
      </div>

      {/* Pagination */}
      {listingsData && listingsData.total > 20 && (
        <div className="pagination">
          <button
            type="button"
            disabled={page === 1}
            onClick={() => setPage((p) => Math.max(1, p - 1))}
          >
            {t('previous')}
          </button>
          <span>{t('pageOf', { page, total: Math.ceil(listingsData.total / 20) })}</span>
          <button
            type="button"
            disabled={page >= Math.ceil(listingsData.total / 20)}
            onClick={() => setPage((p) => p + 1)}
          >
            {t('next')}
          </button>
        </div>
      )}

      <style jsx>{`
        .agency-listings {
          padding: 24px;
          max-width: 1400px;
          margin: 0 auto;
        }

        .header {
          display: flex;
          justify-content: space-between;
          align-items: flex-start;
          margin-bottom: 24px;
          flex-wrap: wrap;
          gap: 16px;
        }

        .back-link {
          font-size: 14px;
          color: var(--ppt-fg-muted);
          text-decoration: none;
          display: inline-block;
          margin-bottom: 8px;
        }

        .back-link:hover {
          color: var(--ppt-color-primary);
        }

        .title {
          font-size: 1.75rem;
          font-weight: bold;
          color: var(--ppt-fg-primary);
          margin: 0;
        }

        .subtitle {
          font-size: 1rem;
          color: var(--ppt-fg-muted);
          margin: 4px 0 0;
        }

        .create-button {
          display: flex;
          align-items: center;
          gap: 8px;
          padding: 12px 20px;
          background: var(--ppt-color-primary);
          color: var(--ppt-fg-on-accent);
          border: none;
          border-radius: 8px;
          font-size: 14px;
          font-weight: 500;
          text-decoration: none;
          cursor: pointer;
          transition: background 0.2s;
        }

        .create-button:hover {
          background: var(--ppt-color-primary-hover);
        }

        .filters {
          display: flex;
          gap: 16px;
          align-items: flex-end;
          margin-bottom: 24px;
          flex-wrap: wrap;
        }

        .filter-group {
          display: flex;
          flex-direction: column;
          gap: 6px;
        }

        .filter-group label {
          font-size: 13px;
          font-weight: 500;
          color: var(--ppt-fg-secondary);
        }

        .filter-group select {
          padding: 8px 32px 8px 12px;
          border: 1px solid var(--ppt-border-strong);
          border-radius: 8px;
          font-size: 14px;
          background: var(--ppt-bg-surface) url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' fill='none' viewBox='0 0 24 24' stroke='%236b7280'%3E%3Cpath stroke-linecap='round' stroke-linejoin='round' stroke-width='2' d='M19 9l-7 7-7-7'/%3E%3C/svg%3E") no-repeat right 8px center;
          background-size: 16px;
          appearance: none;
          cursor: pointer;
        }

        .filter-stats {
          margin-left: auto;
          font-size: 14px;
          color: var(--ppt-fg-muted);
        }

        .table-container {
          background: var(--ppt-bg-surface);
          border: 1px solid var(--ppt-border-default);
          border-radius: 12px;
          overflow: hidden;
        }

        .listings-table {
          width: 100%;
          border-collapse: collapse;
        }

        .listings-table th {
          text-align: left;
          padding: 12px 16px;
          font-size: 13px;
          font-weight: 600;
          color: var(--ppt-fg-muted);
          background: var(--ppt-bg-app);
          border-bottom: 1px solid var(--ppt-border-default);
        }

        .pagination {
          display: flex;
          justify-content: center;
          align-items: center;
          gap: 16px;
          margin-top: 24px;
        }

        .pagination button {
          padding: 8px 16px;
          border: 1px solid var(--ppt-border-strong);
          background: var(--ppt-bg-surface);
          border-radius: 6px;
          font-size: 14px;
          cursor: pointer;
        }

        .pagination button:disabled {
          opacity: 0.5;
          cursor: not-allowed;
        }

        .pagination span {
          font-size: 14px;
          color: var(--ppt-fg-muted);
        }
      `}</style>
    </div>
  );
}

function ListingRow({ listing }: { listing: AgencyListing }) {
  const t = useTranslations('agencyListings');
  const statusConfig: Record<AgencyListingStatus, { color: string; bg: string }> = {
    active: {
      color: 'var(--ppt-color-success)',
      bg: 'var(--ppt-color-success-light)',
    },
    draft: { color: 'var(--ppt-fg-muted)', bg: 'var(--ppt-border-default)' },
    pending: {
      color: 'var(--ppt-color-warning)',
      bg: 'var(--ppt-color-warning-light)',
    },
    sold: { color: '#8b5cf6', bg: '#ede9fe' },
    rented: { color: '#06b6d4', bg: '#cffafe' },
    withdrawn: {
      color: 'var(--ppt-color-danger)',
      bg: 'var(--ppt-color-danger-light)',
    },
  };

  const status = statusConfig[listing.status];

  const formatPrice = (price: number, currency: string) =>
    new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency,
      maximumFractionDigits: 0,
    }).format(price);

  return (
    <tr className="listing-row">
      <td className="property-cell">
        <div className="property-info">
          {listing.primaryPhotoUrl && (
            <img src={listing.primaryPhotoUrl} alt={listing.title} className="property-image" />
          )}
          <div>
            <span className="property-title">{listing.title}</span>
          </div>
        </div>
      </td>
      <td>
        <span className="type-badge">
          {listing.transactionType === 'sale' ? t('typeSale') : t('typeRent')}
        </span>
      </td>
      <td className="price-cell">{formatPrice(listing.price, listing.currency)}</td>
      <td>
        <span className="status-badge" style={{ color: status.color, backgroundColor: status.bg }}>
          {t(`status.${listing.status}`)}
        </span>
      </td>
      <td>{listing.realtorName}</td>
      <td>{listing.views}</td>
      <td>{listing.inquiries}</td>
      <td className="date-cell">{new Date(listing.updatedAt).toLocaleDateString()}</td>
      <td>
        <div className="actions">
          <Link href={`/listings/${listing.slug}`} className="action-link">
            {t('view')}
          </Link>
          <Link href={`/listings/${listing.slug}/edit`} className="action-link">
            {t('edit')}
          </Link>
        </div>
      </td>

      <style jsx>{`
        .listing-row {
          border-bottom: 1px solid var(--ppt-bg-subtle);
        }

        .listing-row:hover {
          background: var(--ppt-bg-app);
        }

        .listing-row td {
          padding: 16px;
          font-size: 14px;
          color: var(--ppt-fg-secondary);
        }

        .property-cell {
          max-width: 300px;
        }

        .property-info {
          display: flex;
          align-items: center;
          gap: 12px;
        }

        .property-image {
          width: 48px;
          height: 36px;
          object-fit: cover;
          border-radius: 6px;
        }

        .property-title {
          font-weight: 500;
          color: var(--ppt-fg-primary);
          display: -webkit-box;
          -webkit-line-clamp: 1;
          -webkit-box-orient: vertical;
          overflow: hidden;
        }

        .type-badge {
          padding: 4px 10px;
          background: var(--ppt-bg-subtle);
          border-radius: 4px;
          font-size: 12px;
          font-weight: 500;
          color: var(--ppt-fg-secondary);
        }

        .price-cell {
          font-weight: 600;
          color: var(--ppt-fg-primary);
        }

        .status-badge {
          padding: 4px 10px;
          border-radius: 12px;
          font-size: 12px;
          font-weight: 500;
        }

        .date-cell {
          color: var(--ppt-fg-muted);
        }

        .actions {
          display: flex;
          gap: 12px;
        }

        .action-link {
          font-size: 14px;
          color: var(--ppt-color-primary);
          text-decoration: none;
        }

        .action-link:hover {
          text-decoration: underline;
        }
      `}</style>
    </tr>
  );
}

function ListingsTableSkeleton() {
  return (
    <div className="skeleton">
      {[1, 2, 3, 4, 5].map((i) => (
        <div key={`skel-${i}`} className="skeleton-row" />
      ))}
      <style jsx>{`
        .skeleton {
          padding: 16px;
        }
        .skeleton-row {
          height: 64px;
          background: var(--ppt-border-default);
          border-radius: 8px;
          margin-bottom: 8px;
        }
      `}</style>
    </div>
  );
}

function EmptyState() {
  const t = useTranslations('agencyListings');
  return (
    <div className="empty-state">
      <svg
        width="64"
        height="64"
        viewBox="0 0 24 24"
        fill="none"
        stroke="var(--ppt-fg-subtle)"
        strokeWidth="1.5"
        aria-hidden="true"
      >
        <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
        <polyline points="9 22 9 12 15 12 15 22" />
      </svg>
      <h3>{t('emptyTitle')}</h3>
      <p>{t('emptyText')}</p>
      <Link href="/listings/create" className="create-button">
        {t('createListing')}
      </Link>
      <style jsx>{`
        .empty-state {
          display: flex;
          flex-direction: column;
          align-items: center;
          padding: 64px 24px;
          text-align: center;
        }
        h3 {
          font-size: 1.25rem;
          color: var(--ppt-fg-primary);
          margin: 24px 0 8px;
        }
        p {
          color: var(--ppt-fg-muted);
          margin: 0 0 24px;
        }
        .create-button {
          padding: 12px 24px;
          background: var(--ppt-color-primary);
          color: var(--ppt-fg-on-accent);
          border-radius: 8px;
          text-decoration: none;
          font-weight: 500;
        }
      `}</style>
    </div>
  );
}
