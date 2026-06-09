/**
 * Reality Portal Price Map hooks (UC-31: Price Map Page).
 *
 * React Query hook over reality-server `GET /api/v1/price-map`, which
 * aggregates live avg €/m² from the listings table (server-cached 1h).
 */

'use client';

import { useQuery } from '@tanstack/react-query';

import { getApiBase } from '../config';
import type { PriceMapFilters, PriceMapResponse } from './types';

function buildPriceMapQuery(filters: PriceMapFilters): string {
  const params = new URLSearchParams();
  params.set('mode', filters.mode ?? 'sale');
  params.set('time_window', filters.timeWindow ?? '12m');
  if (filters.propertyType) params.set('property_type', filters.propertyType);
  if (filters.city) params.set('city', filters.city);
  return params.toString();
}

/**
 * Fetch live city-level price aggregations for the price map.
 *
 * The server already caches per query for an hour, so a short client
 * `staleTime` is enough to avoid refetch storms while filters change.
 */
export function usePriceMap(filters: PriceMapFilters = {}) {
  return useQuery({
    queryKey: ['price-map', filters],
    queryFn: async (): Promise<PriceMapResponse> => {
      const response = await fetch(
        `${getApiBase()}/api/v1/price-map?${buildPriceMapQuery(filters)}`,
        { credentials: 'include' }
      );
      if (!response.ok) {
        throw new Error(`Failed to fetch price map (${response.status})`);
      }
      return response.json();
    },
    staleTime: 5 * 60 * 1000,
  });
}
