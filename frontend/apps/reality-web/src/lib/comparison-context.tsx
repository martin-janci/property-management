/**
 * Property comparison context for Reality Portal.
 *
 * Epic 51 - Story 51.1: Add to Comparison
 */

'use client';

import type { ListingSummary } from '@ppt/reality-api-client';
import { createContext, useCallback, useContext, useEffect, useRef, useState } from 'react';

const MAX_COMPARISON_ITEMS = 4;
const STORAGE_KEY = 'ppt-comparison';

/**
 * Runtime shape guard for a persisted comparison entry.
 *
 * `JSON.parse` only rejects malformed JSON — a tampered or corrupt stored value
 * that is still valid JSON (e.g. `{}`, `"x"`, `42`, `null`, `[{}]`) parses fine
 * but is not an array of listings. Feeding that straight into state makes the
 * array operations downstream (`.some`, `.map`, `.filter`, `.length`) throw and
 * crash the provider. We only ever key off `id`, so an entry counts as usable
 * when it is an object carrying a string `id`.
 */
function isStoredListing(value: unknown): value is ListingSummary {
  return (
    typeof value === 'object' &&
    value !== null &&
    typeof (value as { id?: unknown }).id === 'string'
  );
}

/**
 * Parse a raw localStorage string into a safe listing array. Any malformed
 * value (bad JSON, non-array, or an array with unusable entries) degrades
 * gracefully to the usable subset — never throws.
 */
function parseStoredListings(raw: string): ListingSummary[] {
  const parsed: unknown = JSON.parse(raw);
  if (!Array.isArray(parsed)) {
    return [];
  }
  return parsed.filter(isStoredListing);
}

interface ComparisonContextValue {
  listings: ListingSummary[];
  isInComparison: (listingId: string) => boolean;
  addToComparison: (listing: ListingSummary) => boolean;
  removeFromComparison: (listingId: string) => void;
  clearComparison: () => void;
  canAddMore: boolean;
  shareUrl: string | null;
  generateShareUrl: () => string;
}

const ComparisonContext = createContext<ComparisonContextValue | null>(null);

export function ComparisonProvider({ children }: { children: React.ReactNode }) {
  const [listings, setListings] = useState<ListingSummary[]>([]);
  const [shareUrl, setShareUrl] = useState<string | null>(null);
  // Ref to track current listings for synchronous access in addToComparison
  const listingsRef = useRef<ListingSummary[]>([]);

  // Load from localStorage on mount (SSR safety: check window exists)
  useEffect(() => {
    if (typeof window === 'undefined') return;
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      try {
        const parsed = parseStoredListings(stored);
        if (parsed.length > 0) {
          setListings(parsed);
        } else {
          // Nothing usable in the stored value — drop it.
          localStorage.removeItem(STORAGE_KEY);
        }
      } catch {
        // Invalid JSON, clear it
        localStorage.removeItem(STORAGE_KEY);
      }
    }
  }, []);

  // Save to localStorage when listings change (SSR safety: check window exists)
  useEffect(() => {
    // Keep ref in sync with state for synchronous access
    listingsRef.current = listings;

    if (typeof window === 'undefined') return;
    if (listings.length > 0) {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(listings));
    } else {
      localStorage.removeItem(STORAGE_KEY);
    }
    // Clear share URL when comparison changes
    setShareUrl(null);
  }, [listings]);

  const isInComparison = useCallback(
    (listingId: string) => {
      return listings.some((l) => l.id === listingId);
    },
    [listings]
  );

  const addToComparison = useCallback((listing: ListingSummary) => {
    // Use ref for synchronous check to determine return value
    const current = listingsRef.current;
    if (current.length >= MAX_COMPARISON_ITEMS) {
      return false;
    }
    if (current.some((l) => l.id === listing.id)) {
      return false;
    }
    // Update state (ref will be updated in useEffect)
    setListings((prev) => [...prev, listing]);
    return true;
  }, []);

  const removeFromComparison = useCallback((listingId: string) => {
    setListings((prev) => prev.filter((l) => l.id !== listingId));
  }, []);

  const clearComparison = useCallback(() => {
    setListings([]);
  }, []);

  const generateShareUrl = useCallback(() => {
    const ids = listings.map((l) => l.id).join(',');
    const baseUrl =
      typeof window !== 'undefined'
        ? window.location.origin
        : (process.env.NEXT_PUBLIC_APP_URL ?? '');
    const url = baseUrl ? `${baseUrl}/compare?ids=${ids}` : `/compare?ids=${ids}`;
    setShareUrl(url);
    return url;
  }, [listings]);

  const canAddMore = listings.length < MAX_COMPARISON_ITEMS;

  return (
    <ComparisonContext.Provider
      value={{
        listings,
        isInComparison,
        addToComparison,
        removeFromComparison,
        clearComparison,
        canAddMore,
        shareUrl,
        generateShareUrl,
      }}
    >
      {children}
    </ComparisonContext.Provider>
  );
}

export function useComparison() {
  const context = useContext(ComparisonContext);
  if (!context) {
    throw new Error('useComparison must be used within a ComparisonProvider');
  }
  return context;
}
