/**
 * Reality Portal Price Map Hooks
 *
 * React Query hook for the price-map aggregation API (UC-31).
 * Mirrors the hand-written fetch-wrapper pattern used by the favorites /
 * saved-searches modules rather than the generated client.
 */

'use client';

import { useQuery } from '@tanstack/react-query';

import { getApiBase } from '../config';
import type { PriceMapParams, PriceMapResponse } from './types';

/**
 * Fetch city/district price aggregations from reality-server.
 *
 * The server caches each query combination for 1 hour; the client mirrors
 * that with a 1-hour `staleTime` so identical filter selections don't refetch.
 */
export function usePriceMap(params: PriceMapParams = {}) {
  const { mode = 'sale', propertyType, timeWindow, city } = params;

  return useQuery({
    queryKey: ['price-map', mode, propertyType ?? 'all', timeWindow ?? '3m', city ?? ''],
    queryFn: async (): Promise<PriceMapResponse> => {
      const search = new URLSearchParams();
      search.set('mode', mode);
      // 'all' means "no property-type filter" — omit so the server aggregates across types.
      if (propertyType && propertyType !== 'all') search.set('property_type', propertyType);
      if (timeWindow) search.set('time_window', timeWindow);
      if (city) search.set('city', city);

      const response = await fetch(`${getApiBase()}/api/v1/price-map?${search}`, {
        credentials: 'include',
      });
      if (!response.ok) throw new Error('Failed to fetch price map');
      return response.json();
    },
    staleTime: 60 * 60 * 1000, // 1h — mirrors the server-side cache TTL
  });
}
