/**
 * Handles shared comparison URLs by loading listings from IDs.
 *
 * Epic 51 - Story 51.3: Share Comparison
 */

'use client';

import type { ListingSummary } from '@ppt/reality-api-client';
import { useTranslations } from 'next-intl';
import { useEffect, useState } from 'react';

import { useComparison } from '../../lib/comparison-context';

interface ComparisonUrlHandlerProps {
  sharedIds: string[];
}

export function ComparisonUrlHandler({ sharedIds }: ComparisonUrlHandlerProps) {
  const t = useTranslations('comparison');
  const { listings, addToComparison } = useComparison();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    // Skip if no shared IDs or if we already have listings
    if (sharedIds.length === 0 || listings.length > 0) {
      return;
    }

    const loadSharedListings = async () => {
      setLoading(true);
      setError(null);

      try {
        // Fetch each listing by ID
        const fetchPromises = sharedIds.slice(0, 4).map(async (id) => {
          const response = await fetch(`/api/listings/${id}`);
          if (!response.ok) {
            throw new Error(`Failed to load listing ${id}`);
          }
          return response.json() as Promise<ListingSummary>;
        });

        const loadedListings = await Promise.all(fetchPromises);

        // Add each listing to comparison
        for (const listing of loadedListings) {
          if (listing) {
            addToComparison(listing);
          }
        }
      } catch (err) {
        setError(t('loadError'));
        console.error('Error loading shared listings:', err);
      } finally {
        setLoading(false);
      }
    };

    loadSharedListings();
  }, [sharedIds, listings.length, addToComparison, t]);

  if (loading) {
    return (
      <div className="loading-shared">
        <div className="spinner" />
        <p>{t('loading')}</p>
        <style jsx>{`
          .loading-shared {
            display: flex;
            flex-direction: column;
            align-items: center;
            padding: 24px;
            color: var(--ppt-fg-muted);
          }
          .spinner {
            width: 32px;
            height: 32px;
            border: 3px solid var(--ppt-border-default);
            border-top-color: var(--ppt-color-primary);
            border-radius: 50%;
            animation: spin 0.8s linear infinite;
            margin-bottom: 12px;
          }
          @keyframes spin {
            to {
              transform: rotate(360deg);
            }
          }
        `}</style>
      </div>
    );
  }

  if (error) {
    return (
      <div className="error-message" role="alert">
        <p>{error}</p>
        <style jsx>{`
          .error-message {
            background: var(--ppt-color-danger-light);
            border: 1px solid var(--ppt-color-danger-light);
            border-radius: 8px;
            padding: 16px;
            margin-bottom: 24px;
            color: var(--ppt-color-danger-hover);
          }
          p {
            margin: 0;
          }
        `}</style>
      </div>
    );
  }

  return null;
}
