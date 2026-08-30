'use client';

/**
 * Realtor "My listings" page (UC-51.3).
 *
 * Lists listings the signed-in realtor manages.
 */

import type { AgencyListing } from '@ppt/reality-api-client';
import { useMyListings } from '@ppt/reality-api-client';
import Link from 'next/link';
import { useTranslations } from 'next-intl';
import { ProtectedRoute } from '@/components/auth';
import { Footer, Header } from '@/components/ui';

function formatPrice(value: number, currency: string) {
  try {
    return new Intl.NumberFormat('en', { style: 'currency', currency }).format(value);
  } catch {
    return `${value} ${currency}`;
  }
}

function ListingCard({ listing }: { listing: AgencyListing }) {
  return (
    <li className="card">
      <div className="meta">
        <Link href={`/listings/${listing.slug}`} className="title">
          {listing.title}
        </Link>
        <span className={`status status-${listing.status}`}>{listing.status}</span>
      </div>
      <div className="info">
        <span>{formatPrice(listing.price, listing.currency)}</span>
        <span>·</span>
        <span>{listing.transactionType === 'sale' ? 'For sale' : 'For rent'}</span>
        <span>·</span>
        <span>{listing.views} views</span>
        <span>·</span>
        <span>{listing.inquiries} inquiries</span>
      </div>
      <div className="actions">
        <Link href={`/realtor/listings/${listing.id}/edit`} className="link">
          Edit
        </Link>
      </div>
      <style jsx>{`
        .card {
          background: var(--ppt-fg-on-accent); padding: 16px; border-radius: 12px;
          box-shadow: 0 1px 3px rgba(0,0,0,.08); display: grid; gap: 8px;
        }
        .meta { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
        .title { font-weight: 600; color: var(--ppt-fg-primary); text-decoration: none; }
        .title:hover { text-decoration: underline; }
        .status {
          font-size: 11px; text-transform: uppercase; letter-spacing: .5px;
          padding: 2px 8px; border-radius: 999px; background: var(--ppt-border-default); color: var(--ppt-fg-secondary);
        }
        .status-active { background: var(--ppt-color-success-light); color: var(--ppt-color-success-dark); }
        .status-draft { background: var(--ppt-color-warning-light); color: var(--ppt-color-warning-dark); }
        .info { font-size: 13px; color: var(--ppt-fg-muted); display: flex; gap: 6px; flex-wrap: wrap; }
        .actions { display: flex; justify-content: flex-end; }
        .link { color: var(--ppt-color-primary); text-decoration: none; font-size: 14px; font-weight: 500; }
        .link:hover { text-decoration: underline; }
      `}</style>
    </li>
  );
}

function MyListingsContent() {
  const t = useTranslations('pages.realtorListings');
  const { data, isLoading, error } = useMyListings();
  const listings = data?.listings ?? [];

  return (
    <div className="container">
      <header className="head">
        <div>
          <h1 className="title">My listings</h1>
          <p className="subtitle">Manage the properties you have published.</p>
        </div>
        <Link href="/realtor/listings/new" className="primary">
          + New listing
        </Link>
      </header>

      {isLoading && <p className="state">Loading…</p>}
      {error && <p className="state error">{t('error')}</p>}
      {!isLoading && !error && listings.length === 0 && (
        <div className="empty">
          <p>{t('empty')}</p>
          <Link href="/realtor/listings/new" className="primary">
            {t('createFirst')}
          </Link>
        </div>
      )}

      <ul className="list">
        {listings.map((listing) => (
          <ListingCard key={listing.id} listing={listing} />
        ))}
      </ul>

      <style jsx>{`
        .container { max-width: 960px; margin: 0 auto; padding: 32px 16px; }
        .head { display: flex; justify-content: space-between; align-items: center; margin-bottom: 24px; gap: 16px; flex-wrap: wrap; }
        .title { font-size: 2rem; font-weight: 700; color: var(--ppt-fg-primary); margin: 0 0 4px; }
        .subtitle { color: var(--ppt-fg-muted); margin: 0; }
        .primary {
          background: var(--ppt-color-primary); color: var(--ppt-fg-on-accent); padding: 10px 20px; border-radius: 8px;
          text-decoration: none; font-weight: 600;
        }
        .primary:hover { background: var(--ppt-color-primary-hover); }
        .state { padding: 32px; text-align: center; color: var(--ppt-fg-muted); }
        .error { color: var(--ppt-color-danger-hover); }
        .empty {
          padding: 48px 24px; text-align: center; color: var(--ppt-fg-muted); background: var(--ppt-fg-on-accent);
          border-radius: 12px; display: flex; flex-direction: column; gap: 16px; align-items: center;
        }
        .list { display: flex; flex-direction: column; gap: 12px; padding: 0; margin: 0; list-style: none; }
      `}</style>
    </div>
  );
}

export default function MyListingsPage() {
  return (
    <div className="page">
      <Header />
      <main>
        <ProtectedRoute>
          <MyListingsContent />
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
