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
import { useQuery } from '@tanstack/react-query';

import { useAdminAuth } from './AdminAuthContext';

/** Shape returned by `/admin/capabilities/me`. */
interface MeCapabilitiesResponse {
  principal_kind: 'platform' | 'org' | 'service';
  capabilities: ReadonlyArray<Capability>;
}

export interface PrincipalCapabilitiesResult {
  isPlatformPrincipal: boolean;
  capabilities: ReadonlyArray<Capability>;
  /** True while the /me request is in flight; UI should treat as loading. */
  isLoading: boolean;
}

async function fetchMeCapabilities(token: string): Promise<MeCapabilitiesResponse> {
  const resp = await fetch('/api/v1/admin/capabilities/me', {
    headers: { Authorization: `Bearer ${token}` },
    credentials: 'include',
  });
  if (!resp.ok) {
    // 401 / 403 → return defaults (gated UI stays hidden). Any other
    // status throws and bubbles to the React Query error surface.
    if (resp.status === 401 || resp.status === 403) {
      return { principal_kind: 'org', capabilities: [] };
    }
    throw new Error(`/admin/capabilities/me failed: ${resp.status}`);
  }
  return (await resp.json()) as MeCapabilitiesResponse;
}

export function usePrincipalCapabilities(): PrincipalCapabilitiesResult {
  const { token } = useAdminAuth();

  const { data, isLoading } = useQuery({
    queryKey: ['admin', 'capabilities', 'me', token],
    queryFn: () => fetchMeCapabilities(token as string),
    enabled: token !== null,
    staleTime: 60_000,
    retry: 1,
  });

  return {
    isPlatformPrincipal: data?.principal_kind === 'platform',
    capabilities: data?.capabilities ?? [],
    isLoading: isLoading && token !== null,
  };
}
