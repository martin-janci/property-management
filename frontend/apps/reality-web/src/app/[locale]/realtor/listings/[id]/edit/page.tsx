'use client';

/**
 * Edit listing page (UC-51.5).
 */

import { useParams, useRouter } from 'next/navigation';
import { useTranslations } from 'next-intl';
import { useEffect, useState } from 'react';
import { ProtectedRoute } from '@/components/auth';
import { ListingForm } from '@/components/realtor/ListingForm';
import { Footer, Header } from '@/components/ui';
import {
  getMyListing,
  type ListingDraft,
  type ListingResponse,
  updateListing,
} from '@/lib/realtor-api';
import { useRealtorErrorMessage } from '@/lib/realtor-error';

function EditListingContent() {
  const t = useTranslations('pages.realtorListingEdit');
  const describeError = useRealtorErrorMessage();
  const params = useParams<{ id: string }>();
  const router = useRouter();
  const id = params?.id ?? '';

  const [listing, setListing] = useState<ListingResponse | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  // Raw thrown value; translated at render (locale-reactive, keeps the load
  // effect free of the unstable `describeError` closure).
  const [loadError, setLoadError] = useState<unknown>();
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
        if (!cancelled) setLoadError(err);
      })
      .finally(() => {
        if (!cancelled) setIsLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [id, t]);

  const handleSubmit = async (draft: ListingDraft) => {
    if (!id) return;
    setIsSubmitting(true);
    setSubmitError(undefined);
    try {
      const updated = await updateListing(id, draft);
      setListing(updated);
      setSavedAt(Date.now());
    } catch (err) {
      setSubmitError(describeError(err, 'errorSaveListing'));
    } finally {
      setIsSubmitting(false);
    }
  };

  if (!id) {
    return <p className="state">{t('missingId')}</p>;
  }

  return (
    <div className="container">
      <header className="head">
        <h1 className="title">{t('title')}</h1>
        <p className="subtitle">{t('subtitle')}</p>
        <button type="button" className="back" onClick={() => router.push('/realtor/listings')}>
          {t('back')}
        </button>
      </header>

      {isLoading && <p className="state">{t('loading')}</p>}
      {loadError != null && (
        <p className="state error">{describeError(loadError, 'errorLoadListing')}</p>
      )}
      {savedAt && (
        <p className="success" role="status">
          {t('saved')}
        </p>
      )}

      {listing && (
        <ListingForm
          initialValue={listing}
          submitLabel={t('submit')}
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
