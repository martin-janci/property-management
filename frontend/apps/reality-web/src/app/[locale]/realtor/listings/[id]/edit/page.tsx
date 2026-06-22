'use client';

/**
 * Edit listing page (UC-51.5).
 */

import { useParams, useRouter } from 'next/navigation';
import { useEffect, useState } from 'react';
import { ProtectedRoute } from '@/components/auth';
import { ListingForm } from '@/components/realtor/ListingForm';
import { Footer, Header } from '@/components/ui';
import {
  getMyListing,
  type ListingDraft,
  type ListingResponse,
  RealtorApiError,
  updateListing,
} from '@/lib/realtor-api';

function EditListingContent() {
  const params = useParams<{ id: string }>();
  const router = useRouter();
  const id = params?.id ?? '';

  const [listing, setListing] = useState<ListingResponse | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [loadError, setLoadError] = useState<string>();
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [submitError, setSubmitError] = useState<string>();
  const [savedAt, setSavedAt] = useState<number>();

  useEffect(() => {
    if (!id) return;
    let cancelled = false;
    setIsLoading(true);
    getMyListing(id)
      .then((value) => {
        if (!cancelled) setListing(value);
      })
      .catch((err) => {
        if (!cancelled)
          setLoadError(err instanceof RealtorApiError ? err.message : 'Could not load listing.');
      })
      .finally(() => {
        if (!cancelled) setIsLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [id]);

  const handleSubmit = async (draft: ListingDraft) => {
    if (!id) return;
    setIsSubmitting(true);
    setSubmitError(undefined);
    try {
      const updated = await updateListing(id, draft);
      setListing(updated);
      setSavedAt(Date.now());
    } catch (err) {
      setSubmitError(err instanceof RealtorApiError ? err.message : 'Could not save listing.');
    } finally {
      setIsSubmitting(false);
    }
  };

  if (!id) {
    return <p className="state">Missing listing id.</p>;
  }

  return (
    <div className="container">
      <header className="head">
        <h1 className="title">Edit listing</h1>
        <p className="subtitle">Update the details of your listing.</p>
        <button type="button" className="back" onClick={() => router.push('/realtor/listings')}>
          ← Back to listings
        </button>
      </header>

      {isLoading && <p className="state">Loading listing…</p>}
      {loadError && <p className="state error">{loadError}</p>}
      {savedAt && (
        <p className="success" role="status">
          Listing saved.
        </p>
      )}

      {listing && (
        <ListingForm
          initialValue={listing}
          submitLabel="Save changes"
          isSubmitting={isSubmitting}
          generalError={submitError}
          onSubmit={handleSubmit}
        />
      )}

      <style jsx>{`
        .container { max-width: 960px; margin: 0 auto; padding: 32px 16px; }
        .head { margin-bottom: 24px; }
        .title { font-size: 2rem; font-weight: 700; color: var(--ppt-fg-primary); margin: 0 0 4px; }
        .subtitle { color: var(--ppt-fg-muted); margin: 0 0 8px; }
        .back { background: none; border: none; color: var(--ppt-color-primary); cursor: pointer; padding: 0; font-size: 14px; }
        .back:hover { text-decoration: underline; }
        .state { padding: 24px; text-align: center; color: var(--ppt-fg-muted); }
        .error { color: var(--ppt-color-danger-hover); }
        .success {
          margin-bottom: 16px; padding: 12px 16px; background: var(--ppt-color-success-light); color: var(--ppt-color-success-dark);
          border: 1px solid var(--ppt-color-success); border-radius: 8px; font-size: 14px;
        }
      `}</style>
    </div>
  );
}

export default function EditListingPage() {
  return (
    <div className="page">
      <Header />
      <main>
        <ProtectedRoute>
          <EditListingContent />
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
