/**
 * Agency Hooks
 *
 * React Query hooks for agency management in Reality Portal (Epic 45).
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { getApiBase } from '../config';
import type {
  Agency,
  AgencyBranding,
  AgencyListing,
  AgencyMember,
  AgencyPerformance,
  AgencyStats,
  CreateAgencyRequest,
  InviteRealtorRequest,
  Realtor,
  RealtorStats,
  UpdateAgencyRequest,
  UpdateBrandingRequest,
  UpdateRealtorRequest,
} from './types';

/**
 * Error thrown by the agency fetchers on a non-2xx response.
 *
 * Carries the HTTP `status` so callers can distinguish a genuine `404`
 * ("you have no agency" — an expected onboarding state) from a transport
 * or server error (5xx/network) that warrants an error/retry screen.
 * Before this existed every failure was an opaque `Error`, so a 404 and a
 * 500 were indistinguishable and both surfaced the retry UI (Issue #2343).
 */
export class ApiError extends Error {
  readonly status: number;

  constructor(status: number) {
    super(`API error: ${status}`);
    this.name = 'ApiError';
    this.status = status;
  }
}

/** Type guard for {@link ApiError} with an optional status match. */
export function isApiError(error: unknown, status?: number): error is ApiError {
  return error instanceof ApiError && (status === undefined || error.status === status);
}

async function fetchApi<T>(endpoint: string, options?: RequestInit): Promise<T> {
  const response = await fetch(`${getApiBase()}${endpoint}`, {
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
    credentials: 'include',
    ...options,
  });

  if (!response.ok) {
    throw new ApiError(response.status);
  }

  return response.json();
}

// Agency Queries

export function useAgency(agencyId?: string) {
  return useQuery({
    queryKey: ['agency', agencyId],
    queryFn: () => fetchApi<Agency>(`/api/v1/agencies/${agencyId}`),
    enabled: !!agencyId,
  });
}

/**
 * Public agency lookup by slug (Issue #978).
 *
 * The reality-server `GET /api/v1/agencies/by-slug/{slug}` handler wraps the
 * payload in `AgencyResponse { agency }` (unlike the by-id callers above which
 * read the bare entity), so we unwrap `.agency` in the queryFn.
 */
export function useAgencyBySlug(slug?: string) {
  return useQuery({
    queryKey: ['agency-by-slug', slug],
    queryFn: async () => {
      const { agency } = await fetchApi<{ agency: Agency }>(`/api/v1/agencies/by-slug/${slug}`);
      return agency;
    },
    enabled: !!slug,
  });
}

/**
 * Public agency team members (Issue #978).
 *
 * `GET /api/v1/agencies/{id}/members` returns `MembersResponse { members, total }`.
 */
export function useAgencyMembers(agencyId?: string) {
  return useQuery({
    queryKey: ['agency-members', agencyId],
    queryFn: () =>
      fetchApi<{ members: AgencyMember[]; total: number }>(`/api/v1/agencies/${agencyId}/members`),
    enabled: !!agencyId,
  });
}

export function useMyAgency() {
  return useQuery({
    queryKey: ['my-agency'],
    queryFn: () => fetchApi<Agency>('/api/v1/agencies/me'),
    // A 404 means "this user has no agency" — an expected state, not a
    // transient failure — so don't burn retries spinning on it. Transport
    // and server errors (5xx/network) still get the default retry behaviour.
    retry: (failureCount, error) => !isApiError(error, 404) && failureCount < 3,
  });
}

export function useAgencyStats(agencyId: string, period?: string) {
  return useQuery({
    queryKey: ['agency-stats', agencyId, period],
    queryFn: () =>
      fetchApi<AgencyStats>(
        `/api/v1/agencies/${agencyId}/stats${period ? `?period=${period}` : ''}`
      ),
    enabled: !!agencyId,
  });
}

export function useAgencyPerformance(
  agencyId: string,
  startDate?: string,
  endDate?: string,
  interval?: 'day' | 'week' | 'month'
) {
  return useQuery({
    queryKey: ['agency-performance', agencyId, startDate, endDate, interval],
    queryFn: () => {
      const params = new URLSearchParams();
      if (startDate) params.set('startDate', startDate);
      if (endDate) params.set('endDate', endDate);
      if (interval) params.set('interval', interval);
      return fetchApi<AgencyPerformance[]>(
        `/api/v1/agencies/${agencyId}/performance?${params.toString()}`
      );
    },
    enabled: !!agencyId,
  });
}

export function useAgencyBranding(agencyId: string) {
  return useQuery({
    queryKey: ['agency-branding', agencyId],
    queryFn: () => fetchApi<AgencyBranding>(`/api/v1/agencies/${agencyId}/branding`),
    enabled: !!agencyId,
  });
}

// Realtor Queries

export function useRealtors(agencyId: string) {
  return useQuery({
    queryKey: ['realtors', agencyId],
    queryFn: () => fetchApi<Realtor[]>(`/api/v1/agencies/${agencyId}/realtors`),
    enabled: !!agencyId,
  });
}

export function useRealtor(agencyId: string, realtorId: string) {
  return useQuery({
    queryKey: ['realtor', agencyId, realtorId],
    queryFn: () => fetchApi<Realtor>(`/api/v1/agencies/${agencyId}/realtors/${realtorId}`),
    enabled: !!agencyId && !!realtorId,
  });
}

export function useRealtorStats(agencyId: string, realtorId: string, period?: string) {
  return useQuery({
    queryKey: ['realtor-stats', agencyId, realtorId, period],
    queryFn: () =>
      fetchApi<RealtorStats>(
        `/api/v1/agencies/${agencyId}/realtors/${realtorId}/stats${period ? `?period=${period}` : ''}`
      ),
    enabled: !!agencyId && !!realtorId,
  });
}

// Listing Queries

export function useAgencyListings(
  agencyId: string,
  options?: { status?: string; realtorId?: string; page?: number; limit?: number }
) {
  return useQuery({
    queryKey: ['agency-listings', agencyId, options],
    queryFn: () => {
      const params = new URLSearchParams();
      if (options?.status) params.set('status', options.status);
      if (options?.realtorId) params.set('realtorId', options.realtorId);
      if (options?.page) params.set('page', options.page.toString());
      if (options?.limit) params.set('limit', options.limit.toString());
      return fetchApi<{ listings: AgencyListing[]; total: number }>(
        `/api/v1/agencies/${agencyId}/listings?${params.toString()}`
      );
    },
    enabled: !!agencyId,
  });
}

export function useMyListings(options?: { status?: string; page?: number; limit?: number }) {
  return useQuery({
    queryKey: ['my-listings', options],
    queryFn: () => {
      const params = new URLSearchParams();
      if (options?.status) params.set('status', options.status);
      if (options?.page) params.set('page', options.page.toString());
      if (options?.limit) params.set('limit', options.limit.toString());
      return fetchApi<{ listings: AgencyListing[]; total: number }>(
        `/api/v1/realtors/me/listings?${params.toString()}`
      );
    },
  });
}

// Agency Mutations

export function useCreateAgency() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (data: CreateAgencyRequest) =>
      fetchApi<Agency>('/api/v1/agencies', {
        method: 'POST',
        body: JSON.stringify(data),
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['my-agency'] });
    },
  });
}

export function useUpdateAgency() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ agencyId, data }: { agencyId: string; data: UpdateAgencyRequest }) =>
      fetchApi<Agency>(`/api/v1/agencies/${agencyId}`, {
        method: 'PATCH',
        body: JSON.stringify(data),
      }),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: ['agency', variables.agencyId] });
      queryClient.invalidateQueries({ queryKey: ['my-agency'] });
    },
  });
}

export function useUpdateBranding() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async ({ agencyId, data }: { agencyId: string; data: UpdateBrandingRequest }) => {
      const formData = new FormData();
      if (data.logo) formData.append('logo', data.logo);
      if (data.coverImage) formData.append('coverImage', data.coverImage);
      if (data.primaryColor) formData.append('primaryColor', data.primaryColor);
      if (data.secondaryColor) formData.append('secondaryColor', data.secondaryColor);
      if (data.accentColor) formData.append('accentColor', data.accentColor);
      if (data.fontFamily) formData.append('fontFamily', data.fontFamily);

      const response = await fetch(`${getApiBase()}/api/v1/agencies/${agencyId}/branding`, {
        method: 'PUT',
        body: formData,
        credentials: 'include',
      });

      if (!response.ok) {
        throw new ApiError(response.status);
      }

      return response.json();
    },
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: ['agency-branding', variables.agencyId] });
      queryClient.invalidateQueries({ queryKey: ['agency', variables.agencyId] });
    },
  });
}

// Realtor Mutations

export function useInviteRealtor() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ agencyId, data }: { agencyId: string; data: InviteRealtorRequest }) =>
      fetchApi<Realtor>(`/api/v1/agencies/${agencyId}/realtors/invite`, {
        method: 'POST',
        body: JSON.stringify(data),
      }),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: ['realtors', variables.agencyId] });
    },
  });
}

export function useUpdateRealtor() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({
      agencyId,
      realtorId,
      data,
    }: {
      agencyId: string;
      realtorId: string;
      data: UpdateRealtorRequest;
    }) =>
      fetchApi<Realtor>(`/api/v1/agencies/${agencyId}/realtors/${realtorId}`, {
        method: 'PATCH',
        body: JSON.stringify(data),
      }),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: ['realtors', variables.agencyId] });
      queryClient.invalidateQueries({
        queryKey: ['realtor', variables.agencyId, variables.realtorId],
      });
    },
  });
}

export function useRemoveRealtor() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ agencyId, realtorId }: { agencyId: string; realtorId: string }) =>
      fetchApi<void>(`/api/v1/agencies/${agencyId}/realtors/${realtorId}`, {
        method: 'DELETE',
      }),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: ['realtors', variables.agencyId] });
    },
  });
}

export function useResendInvitation() {
  return useMutation({
    mutationFn: ({ agencyId, realtorId }: { agencyId: string; realtorId: string }) =>
      fetchApi<void>(`/api/v1/agencies/${agencyId}/realtors/${realtorId}/resend-invitation`, {
        method: 'POST',
      }),
  });
}
