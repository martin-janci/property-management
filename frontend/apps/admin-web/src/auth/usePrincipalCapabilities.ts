/**
 * Phase 5 (B6) — platform-principal predicate for admin-web.
 *
 * Source of truth: `GET /api/v1/admin/capabilities/me`. The endpoint
 * returns the principal's resolved capability set + a flag indicating
 * whether the principal is platform-scoped (super-admin) or org-scoped
 * (tenant member). Backend access tokens don't include these fields
 * directly, so we fetch them once after login and cache via TanStack
 * Query.
 *
 * Pages call this hook anywhere; `CapabilityProvider` (in App.tsx)
 * consumes it to feed `useCapability` for the per-control gating used
 * by `<ResourceTable>` and `<SettingsForm>`.
 */

import type { Capability } from '@ppt/admin-ui';
import { ApiError, authenticatedFetchJson } from '@ppt/api-client';
import { useQuery } from '@tanstack/react-query';

import { useAdminAuth } from './AdminAuthContext';

/**
 * Shape returned by `/admin/capabilities/me`.
 *
 * `capabilities` is an array of `CapabilityGrant` rows (not bare strings) —
 * each row carries the grant metadata (id, granted_by, granted_at, expires_at,
 * revoked_at, mfa_required, note). The frontend currently only needs the
 * `capability` field for gating, so we flatten to `ReadonlyArray<Capability>`
 * in the hook's return value.
 *
 * If a follow-up surface wants to display the grant lineage (e.g. "granted
 * by X on date Y, expires Z"), expose `data` directly instead of flattening.
 */
interface CapabilityGrantRow {
  capability: Capability;
  expires_at: string | null;
  revoked_at: string | null;
}

interface MeCapabilitiesResponse {
  principal_kind: 'platform' | 'org' | 'service' | 'staff' | 'public';
  capabilities: ReadonlyArray<CapabilityGrantRow>;
  /**
   * ISO-8601 timestamp of the caller's most recent MFA verification within
   * the current window, or `null` / absent if never verified or expired.
   * Emitted by api-server since Sprint 1 PR B (scan finding #12).
   */
  mfa_verified_at?: string | null;
  /**
   * Length of the MFA validity window in seconds (typically 900 = 15 min).
   */
  mfa_window_seconds?: number | null;
}

export interface PrincipalCapabilitiesResult {
  isPlatformPrincipal: boolean;
  capabilities: ReadonlyArray<Capability>;
  /** True while the /me request is in flight; UI should treat as loading. */
  isLoading: boolean;
  /**
   * Unix-millisecond timestamp of the most recent MFA verification, derived
   * from the `mfa_verified_at` field returned by `/me`. `null` if never
   * verified within the current window.
   */
  mfaVerifiedAtMs: number | null;
  /**
   * Length of the MFA validity window in milliseconds (typically 900_000).
   * Defaults to 900_000 if the backend didn't send the field.
   */
  mfaWindowMs: number;
}

async function fetchMeCapabilities(): Promise<MeCapabilitiesResponse> {
  // Route through the shared admin-web client helper rather than a hand-rolled
  // `fetch`: `authenticatedFetchJson` injects the Authorization header from the
  // registered token provider, applies the `401 { error: "mfa_required" }`
  // challenge-and-retry flow, and normalizes non-2xx responses into `ApiError`
  // (carrying `status`). This is the same primitive the `@ppt/api-client` admin
  // module uses — see api-client `lib/fetch.ts`.
  try {
    return await authenticatedFetchJson<MeCapabilitiesResponse>('/api/v1/admin/capabilities/me');
  } catch (err) {
    // 401 / 403 → return defaults (gated UI stays hidden). Any other status
    // re-throws and bubbles to the React Query error surface, preserving the
    // previous behaviour.
    if (err instanceof ApiError && (err.status === 401 || err.status === 403)) {
      return { principal_kind: 'public', capabilities: [] };
    }
    throw err;
  }
}

/**
 * Filter to live grants only — drop revoked or expired entries. The backend
 * does not currently filter at the SQL layer; doing it here keeps the UI
 * truthful even if a grant is revoked mid-session.
 */
function liveCapabilities(rows: ReadonlyArray<CapabilityGrantRow>): ReadonlyArray<Capability> {
  const now = Date.now();
  return rows
    .filter(
      (g) => g.revoked_at === null && (g.expires_at === null || Date.parse(g.expires_at) > now)
    )
    .map((g) => g.capability);
}

export function usePrincipalCapabilities(): PrincipalCapabilitiesResult {
  const { token } = useAdminAuth();

  const { data, isLoading } = useQuery({
    queryKey: ['admin', 'capabilities', 'me', token],
    // The token is no longer threaded into the request — `authenticatedFetchJson`
    // reads it from the registered provider — but it stays in the query key so a
    // token change still refetches, and gates `enabled` below.
    queryFn: () => fetchMeCapabilities(),
    enabled: token !== null,
    staleTime: 60_000,
    retry: 1,
  });

  const mfaVerifiedAtMs = (() => {
    if (data?.mfa_verified_at == null) return null;
    const parsed = Date.parse(data.mfa_verified_at);
    return Number.isFinite(parsed) ? parsed : null;
  })();

  // Default to 900 s if the backend omits the field (older deployments).
  const mfaWindowMs = (data?.mfa_window_seconds ?? 900) * 1000;

  return {
    isPlatformPrincipal: data?.principal_kind === 'platform',
    capabilities: data ? liveCapabilities(data.capabilities) : [],
    isLoading: isLoading && token !== null,
    mfaVerifiedAtMs,
    mfaWindowMs,
  };
}
