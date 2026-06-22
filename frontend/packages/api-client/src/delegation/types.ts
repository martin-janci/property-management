/**
 * Delegation types (Epic 3, Story 3.4).
 *
 * Mirrors the wire format of `backend/servers/api-server/src/routes/delegations.rs`.
 * The Rust response structs use plain `serde` (no rename), so every field is
 * serialised in `snake_case` — these types match that exactly to avoid runtime
 * key drift.
 */

/** A delegation scope. Mirrors the backend's `valid_scopes` allow-list. */
export type DelegationScope = 'all' | 'voting' | 'documents' | 'faults' | 'financial';

/** All valid delegation scopes, in display order. */
export const DELEGATION_SCOPES: readonly DelegationScope[] = [
  'all',
  'voting',
  'documents',
  'faults',
  'financial',
] as const;

/** Lifecycle status of a delegation (as stored/returned by the backend). */
export type DelegationStatus = 'pending' | 'active' | 'declined' | 'revoked' | 'expired' | string;

/**
 * Compact delegation record returned by the list endpoints.
 *
 * Corresponds to `DelegationSummaryResponse` in `routes/delegations.rs`.
 */
export interface DelegationSummary {
  id: string;
  owner_user_id: string;
  delegate_user_id: string;
  unit_id: string | null;
  scopes: string[];
  status: DelegationStatus;
}

/**
 * Full delegation record returned by create/get/accept/decline/revoke.
 *
 * Corresponds to `DelegationResponse` in `routes/delegations.rs`.
 */
export interface Delegation {
  id: string;
  owner_user_id: string;
  delegate_user_id: string;
  unit_id: string | null;
  scopes: string[];
  status: DelegationStatus;
  /** ISO date (YYYY-MM-DD). */
  start_date: string;
  /** ISO date (YYYY-MM-DD) or null. */
  end_date: string | null;
  /** RFC3339 timestamp or null. */
  accepted_at: string | null;
  /** RFC3339 timestamp or null. */
  revoked_at: string | null;
  revoked_reason: string | null;
  /** RFC3339 timestamp. */
  created_at: string;
}

/**
 * Body for `POST /api/v1/delegations`.
 *
 * Corresponds to `CreateDelegationRequest` in `routes/delegations.rs`.
 */
export interface CreateDelegationRequest {
  delegate_user_id: string;
  /** Optional — applies to all owned units when omitted. */
  unit_id?: string | null;
  /** Defaults to `["all"]` server-side when omitted. */
  scopes?: string[];
  /** ISO date (YYYY-MM-DD). Defaults to today server-side. */
  start_date?: string | null;
  /** ISO date (YYYY-MM-DD). */
  end_date?: string | null;
}

/** Filters for `GET /api/v1/delegations`. */
export interface ListDelegationsQuery {
  unit_id?: string;
  status?: string;
}

/** Result of `GET /api/v1/delegations/check/{unit_id}/{scope}`. */
export interface CheckDelegationResult {
  has_delegation: boolean;
}
