/**
 * Agency Hooks
 *
 * React Query hooks for agency management in Reality Portal (Epic 45).
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { getApiBase, portalFetch } from '../config';
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

/**
 * Thrown — without any network call — by hooks whose feature reality-server
 * does not offer yet. Status 501 so `isApiError(e, 501)` lets the UI render a
 * "not available yet" state instead of a retry screen.
 */
export class NotAvailableError extends ApiError {
  readonly feature: string;

  constructor(feature: string) {
    super(501);
    this.name = 'NotAvailableError';
    this.feature = feature;
    this.message = `${feature} is not available yet`;
  }
}

/** Type guard for {@link NotAvailableError}. */
export function isNotAvailable(error: unknown): error is NotAvailableError {
  return error instanceof NotAvailableError;
}

/** Rejects with {@link NotAvailableError}; used as a queryFn / mutationFn. */
export function notAvailable<T>(feature: string): Promise<T> {
  return Promise.reject(new NotAvailableError(feature));
}

/** Type guard for {@link ApiError} with an optional status match. */
export function isApiError(error: unknown, status?: number): error is ApiError {
  return error instanceof ApiError && (status === undefined || error.status === status);
}

async function fetchApi<T>(endpoint: string, options?: RequestInit): Promise<T> {
  const response = await portalFetch(`${getApiBase()}${endpoint}`, {
    credentials: 'include',
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
  });

  if (!response.ok) {
    throw new ApiError(response.status);
  }

  if (response.status === 204) return undefined as T;
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

/**
 * Agency dashboard totals (`GET /api/v1/agencies/{id}/stats`). The server
 * reports lifetime totals; `period` only partitions the cache key.
 */
export function useAgencyStats(agencyId: string, period?: string) {
  return useQuery({
    queryKey: ['agency-stats', agencyId, period],
    queryFn: () => fetchApi<AgencyStats>(`/api/v1/agencies/${encodeURIComponent(agencyId)}/stats`),
    enabled: !!agencyId,
  });
}

/** Agency activity bucketed over time (`GET /api/v1/agencies/{id}/performance`). */
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
      if (startDate) params.set('startDate', startDate.slice(0, 10));
      if (endDate) params.set('endDate', endDate.slice(0, 10));
      if (interval) params.set('interval', interval);
      const qs = params.toString();
      return fetchApi<AgencyPerformance[]>(
        `/api/v1/agencies/${encodeURIComponent(agencyId)}/performance${qs ? `?${qs}` : ''}`
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

/** Wire shape of `GET /api/v1/agencies/{id}/members` (db `RealityAgencyMember`, snake_case). */
interface WireAgencyMember {
  id: string;
  agency_id: string;
  user_id: string;
  role: string;
  is_active: boolean;
  joined_at: string;
  left_at?: string | null;
}

/**
 * Map an agency member onto the UI's `Realtor`. The members endpoint carries no
 * profile data (name, email, listing counts), so those fields are left empty
 * and the UI falls back to the user id.
 */
export function memberToRealtor(m: WireAgencyMember): Realtor {
  return {
    id: m.id,
    userId: m.user_id,
    agencyId: m.agency_id,
    name: '',
    email: '',
    title: m.role,
    specializations: [],
    activeListings: 0,
    totalSales: 0,
    reviewCount: 0,
    status: m.is_active ? 'active' : 'inactive',
    joinedAt: m.joined_at,
    createdAt: m.joined_at,
    updatedAt: m.left_at ?? m.joined_at,
  };
}

/** Agency realtors, read from the agency members endpoint. */
export function useRealtors(agencyId: string) {
  return useQuery({
    queryKey: ['realtors', agencyId],
    queryFn: async () => {
      const { members } = await fetchApi<{ members: WireAgencyMember[]; total: number }>(
        `/api/v1/agencies/${encodeURIComponent(agencyId)}/members`
      );
      return members.map(memberToRealtor);
    },
    enabled: !!agencyId,
  });
}

/** A single agency realtor, picked from the members list (there is no per-member endpoint). */
export function useRealtor(agencyId: string, realtorId: string) {
  return useQuery({
    queryKey: ['realtor', agencyId, realtorId],
    queryFn: async () => {
      const { members } = await fetchApi<{ members: WireAgencyMember[]; total: number }>(
        `/api/v1/agencies/${encodeURIComponent(agencyId)}/members`
      );
      const member = members.find((m) => m.id === realtorId || m.user_id === realtorId);
      if (!member) throw new ApiError(404);
      return memberToRealtor(member);
    },
    enabled: !!agencyId && !!realtorId,
  });
}

/** Stats of one agency member (`realtorId` is the membership id from {@link useRealtors}). */
export function useRealtorStats(agencyId: string, realtorId: string, period?: string) {
  return useQuery({
    queryKey: ['realtor-stats', agencyId, realtorId, period],
    queryFn: () =>
      fetchApi<RealtorStats>(
        `/api/v1/agencies/${encodeURIComponent(agencyId)}/members/${encodeURIComponent(realtorId)}/stats`
      ),
    enabled: !!agencyId && !!realtorId,
  });
}

// Listing Queries

/**
 * Listings of the agency's active members (`GET /api/v1/agencies/{id}/listings`).
 * `realtorId` filters by the realtor's user id.
 */
export function useAgencyListings(
  agencyId: string,
  options?: { status?: string; realtorId?: string; page?: number; limit?: number }
) {
  return useQuery({
    queryKey: ['agency-listings', agencyId, options],
    queryFn: async () => {
      const params = new URLSearchParams();
      if (options?.status) params.set('status', options.status);
      if (options?.realtorId) params.set('realtorId', options.realtorId);
      if (options?.page) params.set('page', String(options.page));
      if (options?.limit) params.set('limit', String(options.limit));
      const qs = params.toString();
      const wire = await fetchApi<{ listings: Omit<AgencyListing, 'slug'>[]; total: number }>(
        `/api/v1/agencies/${encodeURIComponent(agencyId)}/listings${qs ? `?${qs}` : ''}`
      );
      return {
        total: wire.total,
        // Listings are addressed by id on the portal; there is no slug column.
        listings: wire.listings.map((l) => ({ ...l, slug: l.id })),
      };
    },
    enabled: !!agencyId,
  });
}

/** Wire shape of one row of `GET /api/v1/my/listings` (`PortalListingResponse`, camelCase). */
interface WirePortalListing {
  id: string;
  title: string;
  propertyType: string;
  transactionType: string;
  price: number | string;
  currency: string;
  status: string;
  slug?: string | null;
  createdAt: string;
  updatedAt: string;
}

const AGENCY_LISTING_STATUSES = ['draft', 'active', 'pending', 'sold', 'rented', 'withdrawn'];

function toAgencyListing(l: WirePortalListing): AgencyListing {
  return {
    id: l.id,
    title: l.title,
    slug: l.slug ?? l.id,
    propertyType: l.propertyType,
    transactionType: l.transactionType === 'rent' ? 'rent' : 'sale',
    price: Number(l.price) || 0,
    currency: l.currency,
    status: (AGENCY_LISTING_STATUSES.includes(l.status)
      ? l.status
      : 'withdrawn') as AgencyListing['status'],
    realtorId: '',
    realtorName: '',
    views: 0,
    inquiries: 0,
    createdAt: l.createdAt,
    updatedAt: l.updatedAt,
  };
}

/** The signed-in user's own listings (`GET /api/v1/my/listings`, offset-paged). */
export function useMyListings(options?: { status?: string; page?: number; limit?: number }) {
  return useQuery({
    queryKey: ['my-listings', options],
    queryFn: async () => {
      const params = new URLSearchParams();
      const limit = options?.limit ?? 20;
      if (options?.status) params.set('status', options.status);
      params.set('limit', limit.toString());
      if (options?.page && options.page > 1) {
        params.set('offset', ((options.page - 1) * limit).toString());
      }
      const res = await fetchApi<{ listings: WirePortalListing[]; total: number }>(
        `/api/v1/my/listings?${params.toString()}`
      );
      return { listings: (res.listings ?? []).map(toAgencyListing), total: res.total };
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
        method: 'PUT',
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

      const response = await portalFetch(`${getApiBase()}/api/v1/agencies/${agencyId}/branding`, {
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
      // `POST /agencies/{id}/invitations` (db `CreateAgencyInvitation`): the
      // server takes email + optional role/message; it returns the invitation,
      // not a realtor, so the realtor list is refetched on success.
      fetchApi<unknown>(`/api/v1/agencies/${encodeURIComponent(agencyId)}/invitations`, {
        method: 'POST',
        body: JSON.stringify({ email: data.email, role: 'realtor', message: data.message }),
      }),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: ['realtors', variables.agencyId] });
    },
  });
}

/**
 * Update an agency member (`PUT /api/v1/agencies/{id}/members/{memberId}`).
 * The server stores membership data only: `title` maps to the member role
 * (`owner`/`manager`/`realtor`) and `status` to the active flag.
 */
export function useUpdateRealtor() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({
      agencyId,
      realtorId,
      data,
    }: {
      agencyId: string;
      realtorId: string;
      data: UpdateRealtorRequest;
    }): Promise<Realtor> => {
      const body: { role?: string; isActive?: boolean } = {};
      if (data.title === 'owner' || data.title === 'manager' || data.title === 'realtor') {
        body.role = data.title;
      }
      if (data.status) body.isActive = data.status === 'active';
      const member = await fetchApi<WireAgencyMember>(
        `/api/v1/agencies/${encodeURIComponent(agencyId)}/members/${encodeURIComponent(realtorId)}`,
        { method: 'PUT', body: JSON.stringify(body) }
      );
      return memberToRealtor(member);
    },
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: ['realtors', variables.agencyId] });
    },
  });
}

/** Remove an agency member (`DELETE /api/v1/agencies/{id}/members/{memberId}`). */
export function useRemoveRealtor() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ agencyId, realtorId }: { agencyId: string; realtorId: string }) =>
      fetchApi<void>(
        `/api/v1/agencies/${encodeURIComponent(agencyId)}/members/${encodeURIComponent(realtorId)}`,
        { method: 'DELETE' }
      ),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: ['realtors', variables.agencyId] });
    },
  });
}

/** reality-server cannot resend an invitation yet; rejects with {@link NotAvailableError}. */
export function useResendInvitation() {
  return useMutation({
    mutationFn: (_vars: { agencyId: string; realtorId: string }) =>
      notAvailable<void>('Resending invitations'),
  });
}
