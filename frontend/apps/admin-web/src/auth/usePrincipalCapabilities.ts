/**
 * Phase 5 (B6) — interim platform-principal predicate (admin-web port).
 *
 * Original lives in `ppt-web/src/features/admin/usePrincipalCapabilities.ts`
 * and reads from ppt-web's `AuthContext`. This port reads from `useAdminAuth`
 * (sessionStorage-backed access token) instead.
 *
 * TODO(phase-5-followup): replace with a TanStack-Query hook backed by
 * `GET /api/v1/admin/capabilities/me` once the admin client surfaces it.
 *
 * Capabilities default to the empty array; the admin pages render but the
 * `<ResourceTable>` actions stay hidden until real capabilities arrive.
 */

import type { Capability } from '@ppt/admin-ui';
import { decodeJwt } from '@ppt/shared';
import { useMemo } from 'react';

import { useAdminAuth } from './AdminAuthContext';

interface ExtendedJwtPayload {
  principal_kind?: string;
  capabilities?: ReadonlyArray<string>;
  role?: string;
}

const PLATFORM_ROLE_FALLBACKS: ReadonlySet<string> = new Set([
  'platform',
  'platform_admin',
  'superadmin',
]);

export interface PrincipalCapabilitiesResult {
  isPlatformPrincipal: boolean;
  capabilities: ReadonlyArray<Capability>;
}

export function usePrincipalCapabilities(): PrincipalCapabilitiesResult {
  const { token } = useAdminAuth();

  return useMemo<PrincipalCapabilitiesResult>(() => {
    const payload = (token ? decodeJwt(token) : null) as
      | (ReturnType<typeof decodeJwt> & ExtendedJwtPayload)
      | null;

    const claimKind = payload?.principal_kind?.toLowerCase();
    const role = payload?.role?.toLowerCase();

    const isPlatformPrincipal =
      claimKind === 'platform' ||
      (role !== undefined && PLATFORM_ROLE_FALLBACKS.has(role));

    const capabilities = (payload?.capabilities ?? []) as ReadonlyArray<Capability>;

    return { isPlatformPrincipal, capabilities };
  }, [token]);
}
