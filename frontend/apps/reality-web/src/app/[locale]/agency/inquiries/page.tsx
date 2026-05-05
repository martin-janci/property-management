'use client';

/**
 * Agency Inquiries page (UC-49.5).
 *
 * Lists inquiries received for the signed-in user's listings/agency. Reuses
 * `useMyInquiries` since the API filters by the requesting principal; a
 * dedicated `/agencies/{id}/inquiries` hook can replace it later.
 */

import { ProtectedRoute } from '@/components/auth';
import { Footer, Header } from '@/components/ui';
import type { Inquiry, InquiryStatus } from '@ppt/reality-api-client';
import { useMyInquiries } from '@ppt/reality-api-client';
import Link from 'next/link';
import { useState } from 'react';

const STATUS_FILTERS: ReadonlyArray<{ value: InquiryStatus | 'all'; label: string }> = [
  { value: 'all', label: 'All' },
  { value: 'pending', label: 'Pending' },
  { value: 'responded', label: 'Responded' },
  { value: 'scheduled', label: 'Scheduled' },
  { value: 'completed', label: 'Completed' },
  { value: 'cancelled', label: 'Cancelled' },
];

function statusBadgeColor(status: InquiryStatus): string {
  switch (status) {
    case 'pending':
      return 'var(--ppt-color-warning)';
    case 'responded':
      return 'var(--ppt-color-primary)';
    case 'scheduled':
      return '#7c3aed';
    case 'completed':
      return 'var(--ppt-color-success-hover)';
    case 'cancelled':
      return 'var(--ppt-fg-muted)';
  }
}

function InquiryRow({ inquiry }: { inquiry: Inquiry }) {
  return (
    <li className="row">
      <div className="row-main">
        <Link href={`/listings/${inquiry.listingId}`} className="title">
          {inquiry.listingTitle}
        </Link>
        <p className="meta">
          {inquiry.name} · {inquiry.email}
          {inquiry.phone ? ` · ${inquiry.phone}` : ''}
        </p>
        <p className="message">{inquiry.message}</p>
      </div>
      <div className="row-side">
        <span className="badge" style={{ background: statusBadgeColor(inquiry.status) }}>
          {inquiry.status}
        </span>
        <span className="time">{new Date(inquiry.createdAt).toLocaleDateString()}</span>
      </div>
      <style jsx>{`
        .row {
          display: flex; gap: 16px; padding: 16px; background: var(--ppt-bg-surface); border-radius: 8px;
          box-shadow: 0 1px 3px rgba(0,0,0,.06); align-items: flex-start;
        }
        .row-main { flex: 1; min-width: 0; }
        .title { display: block; font-weight: 600; color: var(--ppt-fg-primary); text-decoration: none; }
        .title:hover { text-decoration: underline; }
        .meta { font-size: 13px; color: var(--ppt-fg-muted); margin: 4px 0 8px; }
        .message {
          font-size: 14px; color: var(--ppt-fg-secondary); margin: 0;
          display: -webkit-box; -webkit-line-clamp: 3; -webkit-box-orient: vertical; overflow: hidden;
        }
        .row-side { display: flex; flex-direction: column; align-items: flex-end; gap: 6px; flex-shrink: 0; }
        .badge { color: var(--ppt-fg-on-accent); font-size: 11px; padding: 2px 10px; border-radius: 999px; text-transform: uppercase; letter-spacing: .5px; }
        .time { font-size: 12px; color: var(--ppt-fg-subtle); }
      `}</style>
    </li>
  );
}

function AgencyInquiriesContent() {
  const [statusFilter, setStatusFilter] = useState<InquiryStatus | 'all'>('all');
  const { data, isLoading, error } = useMyInquiries(
    statusFilter === 'all' ? undefined : statusFilter
  );

  const inquiries = data?.data ?? [];

  return (
    <div className="container">
      <header className="head">
        <h1 className="title">Agency inquiries</h1>
        <p className="subtitle">Inquiries received for your agency's listings.</p>
      </header>

      <div className="filters">
        {STATUS_FILTERS.map((filter) => (
          <button
            type="button"
            key={filter.value}
            className={`chip ${statusFilter === filter.value ? 'chip-active' : ''}`}
            onClick={() => setStatusFilter(filter.value)}
          >
            {filter.label}
          </button>
        ))}
      </div>

      {isLoading && <p className="state">Loading inquiries…</p>}
      {error && <p className="state error">Failed to load inquiries.</p>}
      {!isLoading && !error && inquiries.length === 0 && (
        <p className="state">No inquiries match this filter.</p>
      )}

      <ul className="list">
        {inquiries.map((inquiry) => (
          <InquiryRow key={inquiry.id} inquiry={inquiry} />
        ))}
      </ul>

      <style jsx>{`
        .container { max-width: 960px; margin: 0 auto; padding: 32px 16px; }
        .head { margin-bottom: 16px; }
        .title { font-size: 2rem; font-weight: 700; color: var(--ppt-fg-primary); margin: 0 0 4px; }
        .subtitle { color: var(--ppt-fg-muted); margin: 0; }
        .filters { display: flex; gap: 8px; margin: 16px 0 24px; flex-wrap: wrap; }
        .chip {
          padding: 6px 14px; border-radius: 999px; border: 1px solid var(--ppt-border-strong);
          background: var(--ppt-bg-surface); color: var(--ppt-fg-secondary); font-size: 14px; cursor: pointer;
        }
        .chip:hover { border-color: var(--ppt-fg-subtle); }
        .chip-active { background: var(--ppt-color-primary); color: var(--ppt-fg-on-accent); border-color: var(--ppt-color-primary); }
        .state { padding: 32px; text-align: center; color: var(--ppt-fg-muted); }
        .error { color: var(--ppt-color-danger-hover); }
        .list { display: flex; flex-direction: column; gap: 12px; padding: 0; margin: 0; list-style: none; }
      `}</style>
    </div>
  );
}

export default function AgencyInquiriesPage() {
  return (
    <div className="page">
      <Header />
      <main>
        <ProtectedRoute>
          <AgencyInquiriesContent />
        </ProtectedRoute>
      </main>
      <Footer />
      <style jsx>{`
        .page { min-height: 100vh; display: flex; flex-direction: column; background: var(--ppt-bg-app); }
        main { flex: 1; }
      `}</style>
    </div>
  );
}
