/**
 * Inquiries Page
 *
 * User's inquiry history (Epic 44, Story 44.6).
 */

'use client';

import type { Inquiry, InquiryStatus } from '@ppt/reality-api-client';
import { useCancelInquiry, useMyInquiries } from '@ppt/reality-api-client';
import Link from 'next/link';
import { useTranslations } from 'next-intl';
import { useState } from 'react';
import { ProtectedRoute } from '@/components/auth';
import { Footer, Header } from '@/components/ui';

const statusConfig: Record<InquiryStatus, { label: string; color: string; bg: string }> = {
  pending: {
    label: 'Pending',
    color: 'var(--ppt-color-warning-dark)',
    bg: 'var(--ppt-color-warning-light)',
  },
  responded: {
    label: 'Responded',
    color: 'var(--ppt-color-primary-hover)',
    bg: 'var(--ppt-color-primary-soft-bg)',
  },
  scheduled: { label: 'Scheduled', color: '#6d28d9', bg: '#ede9fe' },
  completed: {
    label: 'Completed',
    color: 'var(--ppt-color-success-dark)',
    bg: 'var(--ppt-color-success-light)',
  },
  cancelled: {
    label: 'Cancelled',
    color: 'var(--ppt-color-danger-dark)',
    bg: 'var(--ppt-color-danger-light)',
  },
};

function InquiryCard({ inquiry }: { inquiry: Inquiry }) {
  const cancelInquiry = useCancelInquiry();
  const [showCancelConfirm, setShowCancelConfirm] = useState(false);

  const status = statusConfig[inquiry.status];
  const canCancel = inquiry.status === 'pending' || inquiry.status === 'responded';

  const handleCancel = () => {
    cancelInquiry.mutate(inquiry.id);
    setShowCancelConfirm(false);
  };

  return (
    <div className="inquiry-card">
      <div className="card-header">
        <div className="listing-info">
          {inquiry.listingPhoto && (
            // Thumbnail is decorative: the listing title link beside it
            // already names the listing for screen readers.
            <img src={inquiry.listingPhoto} alt="" aria-hidden="true" className="listing-photo" />
          )}
          <div>
            <Link href={`/listings/${inquiry.listingId}`} className="listing-title">
              {inquiry.listingTitle}
            </Link>
            <p className="inquiry-date">
              {new Date(inquiry.createdAt).toLocaleDateString()} • {inquiry.type.replace('_', ' ')}
            </p>
          </div>
        </div>
        <span className="status-badge" style={{ color: status.color, background: status.bg }}>
          {status.label}
        </span>
      </div>

      <div className="card-content">
        <p className="message">{inquiry.message}</p>

        {inquiry.scheduledViewing && (
          <div className="scheduled-viewing">
            <svg
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              aria-hidden="true"
            >
              <rect x="3" y="4" width="18" height="18" rx="2" ry="2" />
              <line x1="16" y1="2" x2="16" y2="6" />
              <line x1="8" y1="2" x2="8" y2="6" />
              <line x1="3" y1="10" x2="21" y2="10" />
            </svg>
            <span>
              Viewing scheduled: {new Date(inquiry.scheduledViewing.date).toLocaleDateString()} at{' '}
              {inquiry.scheduledViewing.timeSlot}
            </span>
          </div>
        )}

        {inquiry.agentResponse && (
          <div className="agent-response">
            <p className="response-label">Agent response:</p>
            <p className="response-text">{inquiry.agentResponse}</p>
            {inquiry.respondedAt && (
              <p className="response-date">{new Date(inquiry.respondedAt).toLocaleDateString()}</p>
            )}
          </div>
        )}
      </div>

      {canCancel && (
        <div className="card-footer">
          {showCancelConfirm ? (
            <div className="cancel-confirm">
              <span>Cancel this inquiry?</span>
              <button type="button" className="confirm-yes" onClick={handleCancel}>
                Yes, cancel
              </button>
              <button
                type="button"
                className="confirm-no"
                onClick={() => setShowCancelConfirm(false)}
              >
                No
              </button>
            </div>
          ) : (
            <button
              type="button"
              className="cancel-button"
              onClick={() => setShowCancelConfirm(true)}
            >
              Cancel inquiry
            </button>
          )}
        </div>
      )}

      <style jsx>{`
        .inquiry-card {
          background: var(--ppt-bg-surface);
          border-radius: 12px;
          overflow: hidden;
          box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
        }

        .card-header {
          display: flex;
          justify-content: space-between;
          align-items: flex-start;
          padding: 16px 20px;
          border-bottom: 1px solid var(--ppt-bg-subtle);
        }

        .listing-info {
          display: flex;
          gap: 12px;
        }

        .listing-photo {
          width: 64px;
          height: 48px;
          border-radius: 6px;
          object-fit: cover;
        }

        .listing-title {
          font-weight: 600;
          color: var(--ppt-fg-primary);
          text-decoration: none;
          display: block;
          margin-bottom: 4px;
        }

        .listing-title:hover {
          color: var(--ppt-color-primary);
        }

        .inquiry-date {
          font-size: 13px;
          color: var(--ppt-fg-muted);
          margin: 0;
          text-transform: capitalize;
        }

        .status-badge {
          padding: 4px 12px;
          border-radius: 20px;
          font-size: 12px;
          font-weight: 600;
          flex-shrink: 0;
        }

        .card-content {
          padding: 16px 20px;
        }

        .message {
          color: var(--ppt-fg-secondary);
          margin: 0;
          line-height: 1.6;
        }

        .scheduled-viewing {
          display: flex;
          align-items: center;
          gap: 8px;
          margin-top: 16px;
          padding: 12px;
          background: #ede9fe;
          border-radius: 8px;
          color: #6d28d9;
          font-size: 14px;
        }

        .agent-response {
          margin-top: 16px;
          padding: 12px;
          background: var(--ppt-bg-app);
          border-radius: 8px;
          border-left: 3px solid var(--ppt-color-primary);
        }

        .response-label {
          font-size: 12px;
          font-weight: 600;
          color: var(--ppt-fg-muted);
          margin: 0 0 4px;
          text-transform: uppercase;
        }

        .response-text {
          color: var(--ppt-fg-secondary);
          margin: 0;
          line-height: 1.6;
        }

        .response-date {
          font-size: 12px;
          color: var(--ppt-fg-subtle);
          margin: 8px 0 0;
        }

        .card-footer {
          padding: 12px 20px;
          border-top: 1px solid var(--ppt-bg-subtle);
          background: var(--ppt-bg-app);
        }

        .cancel-button {
          padding: 6px 12px;
          background: transparent;
          border: 1px solid var(--ppt-border-default);
          border-radius: 6px;
          font-size: 13px;
          color: var(--ppt-fg-muted);
          cursor: pointer;
        }

        .cancel-button:hover {
          border-color: var(--ppt-color-danger-hover);
          color: var(--ppt-color-danger-hover);
        }

        .cancel-confirm {
          display: flex;
          align-items: center;
          gap: 12px;
          font-size: 14px;
          color: var(--ppt-fg-secondary);
        }

        .confirm-yes,
        .confirm-no {
          padding: 6px 12px;
          border: none;
          border-radius: 6px;
          font-size: 13px;
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

function InquiriesContent() {
  const t = useTranslations('pages.inquiries');
  const [statusFilter, setStatusFilter] = useState<InquiryStatus | undefined>(undefined);
  const [page, setPage] = useState(1);
  const { data, isLoading, error } = useMyInquiries(statusFilter, page, 10);

  if (isLoading) {
    return (
      <div className="inquiries-list loading">
        {[1, 2, 3].map((i) => (
          <div key={`inquiry-skeleton-${i}`} className="skeleton-card" />
        ))}
        <style jsx>{`
          .inquiries-list {
            display: flex;
            flex-direction: column;
            gap: 16px;
          }
          .skeleton-card {
            height: 180px;
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

  return (
    <>
      {/* Status Filter */}
      <div className="filters">
        <button
          type="button"
          className={`filter-button ${!statusFilter ? 'active' : ''}`}
          onClick={() => {
            setStatusFilter(undefined);
            setPage(1);
          }}
        >
          All
        </button>
        {(Object.keys(statusConfig) as InquiryStatus[]).map((status) => (
          <button
            key={status}
            type="button"
            className={`filter-button ${statusFilter === status ? 'active' : ''}`}
            onClick={() => {
              setStatusFilter(status);
              setPage(1);
            }}
          >
            {statusConfig[status].label}
          </button>
        ))}
      </div>

      {!data || data.data.length === 0 ? (
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
            <path d="M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z" />
            <polyline points="22,6 12,13 2,6" />
          </svg>
          <h2 className="empty-title">{t('emptyTitle')}</h2>
          <p className="empty-text">{t('emptyText')}</p>
          <Link href="/listings" className="browse-link">
            {t('browseListings')}
          </Link>
        </div>
      ) : (
        <>
          <div className="inquiries-list">
            {data.data.map((inquiry) => (
              <InquiryCard key={inquiry.id} inquiry={inquiry} />
            ))}
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
        </>
      )}

      <style jsx>{`
        .filters {
          display: flex;
          gap: 8px;
          flex-wrap: wrap;
          margin-bottom: 24px;
        }

        .filter-button {
          padding: 8px 16px;
          background: var(--ppt-bg-surface);
          border: 1px solid var(--ppt-border-default);
          border-radius: 20px;
          font-size: 14px;
          color: var(--ppt-fg-muted);
          cursor: pointer;
        }

        .filter-button:hover {
          border-color: var(--ppt-color-primary);
          color: var(--ppt-color-primary);
        }

        .filter-button.active {
          background: var(--ppt-color-primary);
          border-color: var(--ppt-color-primary);
          color: var(--ppt-fg-on-accent);
        }

        .inquiries-list {
          display: flex;
          flex-direction: column;
          gap: 16px;
          max-width: 800px;
        }

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

export default function InquiriesPage() {
  return (
    <div className="page-container">
      <Header />
      <main className="main">
        <div className="container">
          <h1 className="page-title">My Inquiries</h1>
          <p className="page-subtitle">Track your property inquiries and viewing requests.</p>
          <ProtectedRoute>
            <InquiriesContent />
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
