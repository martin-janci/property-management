/**
 * Reality Portal Listing Hooks
 *
 * React Query hooks for property listings API (Epic 44).
 */

'use client';

import { useInfiniteQuery, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type {
  CategoryCount,
  FeaturedListingsResponse,
  ListingDetail,
  ListingFilters,
  PaginatedListings,
  SearchSuggestion,
} from './types';

const API_BASE = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8081';

// Helper to build query string
function buildQueryString(filters: ListingFilters, page = 1, pageSize = 20): string {
  const params = new URLSearchParams();
  params.set('page', String(page));
  params.set('pageSize', String(pageSize));

  if (filters.query) params.set('q', filters.query);
  if (filters.transactionType) params.set('transactionType', filters.transactionType);
  if (filters.propertyType?.length) params.set('propertyType', filters.propertyType.join(','));
  if (filters.priceMin !== undefined) params.set('priceMin', String(filters.priceMin));
  if (filters.priceMax !== undefined) params.set('priceMax', String(filters.priceMax));
  if (filters.areaMin !== undefined) params.set('areaMin', String(filters.areaMin));
  if (filters.areaMax !== undefined) params.set('areaMax', String(filters.areaMax));
  if (filters.roomsMin !== undefined) params.set('roomsMin', String(filters.roomsMin));
  if (filters.roomsMax !== undefined) params.set('roomsMax', String(filters.roomsMax));
  if (filters.bedroomsMin !== undefined) params.set('bedroomsMin', String(filters.bedroomsMin));
  if (filters.city) params.set('city', filters.city);
  if (filters.district) params.set('district', filters.district);
  if (filters.features?.length) params.set('features', filters.features.join(','));
  if (filters.sortBy) params.set('sortBy', filters.sortBy);
  if (filters.sortOrder) params.set('sortOrder', filters.sortOrder);

  return params.toString();
}

// Listings Search with Pagination
export function useListings(filters: ListingFilters = {}, page = 1, pageSize = 20) {
  return useQuery({
    queryKey: ['listings', filters, page],
    queryFn: async (): Promise<PaginatedListings> => {
      const queryString = buildQueryString(filters, page, pageSize);
      const response = await fetch(`${API_BASE}/api/v1/listings?${queryString}`, {
        credentials: 'include',
      });
      if (!response.ok) throw new Error('Failed to fetch listings');
      return response.json();
    },
  });
}

// Infinite scroll listings
export function useInfiniteListings(filters: ListingFilters = {}, pageSize = 20) {
  return useInfiniteQuery({
    queryKey: ['listings-infinite', filters],
    queryFn: async ({ pageParam = 1 }): Promise<PaginatedListings> => {
      const queryString = buildQueryString(filters, pageParam, pageSize);
      const response = await fetch(`${API_BASE}/api/v1/listings?${queryString}`, {
        credentials: 'include',
      });
      if (!response.ok) throw new Error('Failed to fetch listings');
      return response.json();
    },
    getNextPageParam: (lastPage) => {
      if (lastPage.page < lastPage.totalPages) {
        return lastPage.page + 1;
      }
      return undefined;
    },
    initialPageParam: 1,
  });
}

// Single Listing Detail
export function useListing(slug: string) {
  return useQuery({
    queryKey: ['listing', slug],
    queryFn: async (): Promise<ListingDetail> => {
      const response = await fetch(`${API_BASE}/api/v1/listings/${slug}`, {
        credentials: 'include',
      });
      if (!response.ok) throw new Error('Failed to fetch listing');
      return response.json();
    },
    enabled: !!slug,
  });
}

// Featured Listings (for homepage)
export function useFeaturedListings() {
  return useQuery({
    queryKey: ['featured-listings'],
    queryFn: async (): Promise<FeaturedListingsResponse> => {
      const response = await fetch(`${API_BASE}/api/v1/listings/featured`, {
        credentials: 'include',
      });
      if (!response.ok) throw new Error('Failed to fetch featured listings');
      return response.json();
    },
    staleTime: 5 * 60 * 1000, // 5 minutes
  });
}

// Category Counts (for homepage)
export function useCategoryCounts() {
  return useQuery({
    queryKey: ['category-counts'],
    queryFn: async (): Promise<CategoryCount[]> => {
      const response = await fetch(`${API_BASE}/api/v1/listings/categories`, {
        credentials: 'include',
      });
      if (!response.ok) throw new Error('Failed to fetch category counts');
      return response.json();
    },
    staleTime: 10 * 60 * 1000, // 10 minutes
  });
}

// Search Suggestions (autocomplete)
export function useSearchSuggestions(query: string) {
  return useQuery({
    queryKey: ['search-suggestions', query],
    queryFn: async (): Promise<SearchSuggestion[]> => {
      const response = await fetch(
        `${API_BASE}/api/v1/listings/suggestions?q=${encodeURIComponent(query)}`,
        { credentials: 'include' }
      );
      if (!response.ok) throw new Error('Failed to fetch suggestions');
      return response.json();
    },
    enabled: query.length >= 2,
    staleTime: 30 * 1000, // 30 seconds
  });
}

// Toggle Favorite
export function useToggleFavorite() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async ({
      listingId,
      isFavorite,
    }: {
      listingId: string;
      isFavorite: boolean;
    }): Promise<void> => {
      const response = await fetch(`${API_BASE}/api/v1/favorites/${listingId}`, {
        method: isFavorite ? 'DELETE' : 'POST',
        credentials: 'include',
      });
      if (!response.ok) throw new Error('Failed to toggle favorite');
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['listings'] });
      queryClient.invalidateQueries({ queryKey: ['listings-infinite'] });
      queryClient.invalidateQueries({ queryKey: ['favorites'] });
    },
  });
}

// Record Listing View (analytics)
export function useRecordView() {
  return useMutation({
    mutationFn: async (listingId: string): Promise<void> => {
      await fetch(`${API_BASE}/api/v1/listings/${listingId}/view`, {
        method: 'POST',
        credentials: 'include',
      });
    },
  });
}
