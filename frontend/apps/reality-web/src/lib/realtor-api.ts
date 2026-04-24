/**
 * Realtor write APIs (UC-51).
 *
 * The reality-api-client doesn't yet expose listing CRUD mutations or the
 * realtor `me` profile endpoints, so this module wraps the corresponding
 * reality-server endpoints with thin fetch helpers. Replace with generated
 * hooks once the SDK is regenerated.
 */

const API_BASE = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8081';

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
  let response: Response;
  try {
    response = await fetch(`${API_BASE}${path}`, {
      credentials: 'include',
      headers: { 'Content-Type': 'application/json', ...(init.headers ?? {}) },
      ...init,
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

export function updateMyRealtorProfile(
  data: UpdateRealtorProfileRequest
): Promise<RealtorProfile> {
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
  area?: number;
  rooms?: number;
}

export interface ListingResponse extends ListingDraft {
  id: string;
  slug: string;
  status: string;
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

export function updateListing(id: string, data: Partial<ListingDraft>): Promise<ListingResponse> {
  return request<ListingResponse>(`/api/v1/listings/${id}`, {
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
