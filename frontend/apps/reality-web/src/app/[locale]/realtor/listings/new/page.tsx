'use client';

/**
 * Create listing page (UC-51.4).
 */

import { useRouter } from 'next/navigation';
import { useState } from 'react';
import { ProtectedRoute } from '@/components/auth';
import { ListingForm } from '@/components/realtor/ListingForm';
import { Footer, Header } from '@/components/ui';
import { createListing, type ListingDraft, RealtorApiError } from '@/lib/realtor-api';

function CreateListingContent() {
  const router = useRouter();
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string>();

  const handleSubmit = async (draft: ListingDraft) => {
    setIsSubmitting(true);
    setError(undefined);
    try {
      const created = await createListing(draft);
      router.replace(`/realtor/listings/${created.id}/edit`);
    } catch (err) {
      setError(
        err instanceof RealtorApiError ? err.message : 'Could not create listing. Please try again.'
      );
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="container">
      <header className="head">
        <h1 className="title">Create listing</h1>
        <p className="subtitle">Publish a new property to Reality Portal.</p>
      </header>
      <ListingForm
        submitLabel="Publish listing"
        isSubmitting={isSubmitting}
        generalError={error}
        onSubmit={handleSubmit}
      />
      <style jsx>{`
        .container { max-width: 960px; margin: 0 auto; padding: 32px 16px; }
        .head { margin-bottom: 24px; }
        .title { font-size: 2rem; font-weight: 700; color: var(--ppt-fg-primary); margin: 0 0 4px; }
        .subtitle { color: var(--ppt-fg-muted); margin: 0; }
      `}</style>
    </div>
  );
}

export default function CreateListingPage() {
  return (
    <div className="page">
      <Header />
      <main>
        <ProtectedRoute>
          <CreateListingContent />
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
