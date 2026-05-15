/**
 * Admin (Super-admin Control Plane) API types — Phase 5.
 *
 * Mirrors the JSON shapes returned by the api-server's `/api/v1/admin/*`
 * endpoints. Hand-written for now; will be replaced by generated types once
 * the OpenAPI spec for the admin surface is regenerated.
 */

export type AgencyStatus = 'active' | 'suspended' | 'archived' | string;

export interface Agency {
  id: string;
  name: string;
  slug: string;
  status: AgencyStatus;
  member_count: number;
}

export interface ListAgenciesParams {
  page?: number;
  page_size?: number;
  q?: string;
}

export interface AdminPaginatedResponse<T> {
  items: T[];
  total: number;
  page: number;
  page_size: number;
  total_pages: number;
}
