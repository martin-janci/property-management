/**
 * Portal Syndication React Query hooks (Epic 105 — Real Estate Portal Webhooks).
 */

import { keepPreviousData, useQuery } from '@tanstack/react-query';
import * as api from './api';
import type { SyndicationDashboardQuery } from './types';

export const syndicationKeys = {
  all: ['syndication'] as const,
  dashboard: (query: SyndicationDashboardQuery) =>
    [...syndicationKeys.all, 'dashboard', query] as const,
  stats: () => [...syndicationKeys.all, 'stats'] as const,
  listing: (listingId: string) => [...syndicationKeys.all, 'listing', listingId] as const,
  listingStatus: (listingId: string) =>
    [...syndicationKeys.all, 'listing-status', listingId] as const,
};

/** Organization syndication dashboard (per-listing rows + aggregate stats). */
export function useSyndicationDashboard(
  query: SyndicationDashboardQuery = {},
  options?: { enabled?: boolean }
) {
  return useQuery({
    queryKey: syndicationKeys.dashboard(query),
    queryFn: () => api.getSyndicationDashboard(query),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
  });
}

/** Organization-wide aggregated syndication statistics. */
export function useOrganizationSyndicationStats() {
  return useQuery({
    queryKey: syndicationKeys.stats(),
    queryFn: () => api.getOrganizationSyndicationStats(),
  });
}

/** Raw syndication records for a single listing. */
export function useListingSyndications(listingId: string) {
  return useQuery({
    queryKey: syndicationKeys.listing(listingId),
    queryFn: () => api.getListingSyndications(listingId),
    enabled: !!listingId,
  });
}

/** Per-portal syndication status dashboard for a single listing. */
export function useListingSyndicationStatus(listingId: string) {
  return useQuery({
    queryKey: syndicationKeys.listingStatus(listingId),
    queryFn: () => api.getListingSyndicationStatus(listingId),
    enabled: !!listingId,
  });
}
