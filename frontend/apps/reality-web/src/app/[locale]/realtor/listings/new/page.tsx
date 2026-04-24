'use client';

/**
 * Create listing page (UC-51.4).
 */

import { ProtectedRoute } from '@/components/auth';
import { Footer, Header } from '@/components/ui';
import { ListingForm } from '@/components/realtor/ListingForm';
import { RealtorApiError, createListing, type ListingDraft } from '@/lib/realtor-api';
import { useRouter } from 'next/navigation';
import { useState } from 'react';

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
        .title { font-size: 2rem; font-weight: 700; color: #111827; margin: 0 0 4px; }
        .subtitle { color: #6b7280; margin: 0; }
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
        .page { min-height: 100vh; display: flex; flex-direction: column; background: #f9fafb; }
        main { flex: 1; }
      `}</style>
    </div>
  );
}
