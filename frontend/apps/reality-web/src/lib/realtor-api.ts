/**
 * Realtor write APIs (UC-51).
 *
 * The reality-api-client doesn't yet expose listing CRUD mutations or the
 * realtor `me` profile endpoints, so this module wraps the corresponding
 * reality-server endpoints with thin fetch helpers. Replace with generated
 * hooks once the SDK is regenerated.
 */

import { getApiBase } from './env';

export class RealtorApiError extends Error {
  constructor(
    message: string,
    public readonly status: number
  ) {
    super(message);
    this.name = 'RealtorApiError';
  }
}

async function request<T>(path: string, init: RequestInit = {}): Promise<T> {
  // Spread `init` first so caller-provided headers don't drop the defaults
  // below. The `headers` assignment always wins, and the caller's headers
  // (if any) are merged into it after the default `Content-Type`.
  const { headers: callerHeaders, ...rest } = init;
  let response: Response;
  try {
    response = await fetch(`${getApiBase()}${path}`, {
      credentials: 'include',
      ...rest,
      headers: { 'Content-Type': 'application/json', ...(callerHeaders ?? {}) },
    });
  } catch {
    throw new RealtorApiError('Network error. Please try again.', 0);
  }
  if (!response.ok) {
    let message = `Request failed (${response.status})`;
    try {
      const data = await response.json();
      if (typeof data?.message === 'string') message = data.message;
    } catch {
      // ignore non-JSON
    }
    throw new RealtorApiError(message, response.status);
  }
  if (response.status === 204) return undefined as T;
  return (await response.json()) as T;
}

export interface RealtorProfile {
  id: string;
  name: string;
  email: string;
  phone?: string;
  title?: string;
  bio?: string;
  photoUrl?: string;
  specializations: string[];
  licenseNumber?: string;
}

export type UpdateRealtorProfileRequest = Partial<
  Pick<RealtorProfile, 'name' | 'phone' | 'title' | 'bio' | 'specializations' | 'licenseNumber'>
>;

export function getMyRealtorProfile(): Promise<RealtorProfile> {
  return request<RealtorProfile>('/api/v1/realtors/me');
}

export function updateMyRealtorProfile(data: UpdateRealtorProfileRequest): Promise<RealtorProfile> {
  return request<RealtorProfile>('/api/v1/realtors/me', {
    method: 'PATCH',
    body: JSON.stringify(data),
  });
}

export interface ListingDraft {
  title: string;
  description: string;
  propertyType: 'apartment' | 'house' | 'land' | 'commercial' | 'other';
  transactionType: 'sale' | 'rent';
  price: number;
  currency: string;
  city: string;
  street?: string;
  postalCode?: string;
  country?: string;
  area?: number;
  rooms?: number;
  floor?: number;
  /**
   * Owner-settable lifecycle status. Publishing (`active`) is moderation-gated
   * server-side: owners may only set the non-public lifecycle states
   * `draft` | `paused` | `sold` | `rented` | `archived` (the server returns 400
   * for anything else). Typed as `string` because the response can also surface
   * the moderated `active` state.
   */
  status?: string;
  isNegotiable?: boolean;
}

export interface ListingResponse extends ListingDraft {
  id: string;
  slug?: string | null;
  status: string;
  isNegotiable?: boolean;
  isPublished?: boolean;
  postalCode?: string;
  country?: string;
  floor?: number;
  totalFloors?: number;
  createdAt: string;
  updatedAt: string;
}

export function createListing(data: ListingDraft): Promise<ListingResponse> {
  return request<ListingResponse>('/api/v1/listings', {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

export function getListing(id: string): Promise<ListingResponse> {
  return request<ListingResponse>(`/api/v1/listings/${id}`);
}

/** Get an owned listing for editing (includes drafts). Requires auth. */
export function getMyListing(id: string): Promise<ListingResponse> {
  return request<ListingResponse>(`/api/v1/my/listings/${id}`);
}

export function updateListing(id: string, data: Partial<ListingDraft>): Promise<ListingResponse> {
  return request<ListingResponse>(`/api/v1/my/listings/${id}`, {
    method: 'PATCH',
    body: JSON.stringify(data),
  });
}

export interface RealtorAnalytics {
  totalListings: number;
  activeListings: number;
  totalViews: number;
  totalInquiries: number;
  conversionRate: number;
}

export function getMyRealtorAnalytics(period?: string): Promise<RealtorAnalytics> {
  const query = period ? `?period=${encodeURIComponent(period)}` : '';
  return request<RealtorAnalytics>(`/api/v1/realtors/me/stats${query}`);
}

/**
 * One of the authenticated user's own portal listings.
 *
 * camelCase wire mirror of reality-server's `PortalListingResponse`
 * (`backend/servers/reality-server/src/routes/portal_listings.rs`). Note there
 * is no image field on this contract, and `price` / `sizeSqm` are Decimals that
 * can arrive as a number or a string depending on the serializer, so callers
 * must coerce before formatting.
 */
export interface MyListing {
  id: string;
  title: string;
  description?: string | null;
  propertyType: string;
  transactionType: string;
  price: number | string;
  currency: string;
  street: string;
  city: string;
  postalCode: string;
  country: string;
  sizeSqm?: number | string | null;
  rooms?: number | null;
  floor?: number | null;
  totalFloors?: number | null;
  status: string;
  isNegotiable: boolean;
  isPublished: boolean;
  slug?: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface MyListingsResponse {
  listings: MyListing[];
  total: number;
}

/** Lists the authenticated caller's own listings (`GET /api/v1/my/listings`). */
export function listMyListings(status?: string): Promise<MyListingsResponse> {
  const query = status ? `?status=${encodeURIComponent(status)}` : '';
  return request<MyListingsResponse>(`/api/v1/my/listings${query}`);
}

/**
 * A single realtor review.
 *
 * snake_case wire mirror of reality-server's `RealtorReview`
 * (`backend/servers/reality-server/src/routes/agent_reviews.rs`) — this
 * endpoint is not camelCased, unlike the rest of realtor-api, so the fields are
 * kept as-is.
 */
export interface RealtorReview {
  id: string;
  realtor_id: string;
  reviewer_user_id: string;
  reviewer_name: string;
  rating: number;
  body?: string | null;
  verified_buyer: boolean;
  created_at: string;
  updated_at: string;
}

export interface RealtorReviewsResponse {
  reviews: RealtorReview[];
  total: number;
  avg_rating?: number | null;
}

/** Lists reviews for a realtor (`GET /api/v1/realtors/{id}/reviews`). */
export function getRealtorReviews(realtorId: string, limit = 20): Promise<RealtorReviewsResponse> {
  return request<RealtorReviewsResponse>(
    `/api/v1/realtors/${encodeURIComponent(realtorId)}/reviews?limit=${limit}`
  );
}
