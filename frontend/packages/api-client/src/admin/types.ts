/**
 * Admin (Super-admin Control Plane) API types — Phase 5 / Epic 10A-2.
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

// ============================================================
// OAuth Client Management (Epic 10A-2)
// ============================================================

export interface OAuthClientSummary {
  id: string;
  clientId: string;
  name: string;
  description: string | null;
  scopes: string[];
  isActive: boolean;
  createdAt: string;
}

export interface RegisterOAuthClientRequest {
  name: string;
  description?: string;
  redirectUris: string[];
  scopes: string[];
  isConfidential?: boolean;
  rotateRefreshTokens?: boolean;
}

export interface RegisterOAuthClientResponse {
  id: string;
  clientId: string;
  clientSecret: string;
  name: string;
  redirectUris: string[];
  scopes: string[];
  createdAt: string;
}

export interface UpdateOAuthClientRequest {
  name?: string;
  description?: string;
  redirectUris?: string[];
  scopes?: string[];
  isActive?: boolean;
  rotateRefreshTokens?: boolean;
}

export interface RegenerateSecretResponse {
  clientSecret: string;
}

/** Known OAuth scopes served by the PPT api-server. */
export const KNOWN_OAUTH_SCOPES = ['profile', 'email', 'org:read', 'full'] as const;
export type KnownOAuthScope = (typeof KNOWN_OAUTH_SCOPES)[number];
