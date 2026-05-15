/**
 * Phase 5 (B6) — interim platform-principal predicate.
 *
 * The proper source of truth is `GET /api/v1/admin/capabilities/users/:me`,
 * which returns both the principal kind and the resolved capability set.
 * That endpoint is not yet wired in the frontend, so this hook fills the gap
 * with a JWT-claim peek + role fallback.
 *
 * TODO(phase-5-followup): replace with a TanStack-Query hook backed by
 * `useMeCapabilities()` once the admin client surfaces it. Until then:
 *   - Prefer `principal_kind` claim if the backend includes it in the JWT.
 *   - Fall back to `role` matching `platform` / `platform_admin` /
 *     `superadmin` so seeded platform users can reach `/admin/*` in dev.
 *
 * Capabilities default to the empty array; the admin pages render but the
 * `<ResourceTable>` actions stay hidden until real capabilities arrive.
 */

import type { Capability } from '@ppt/admin-ui';
import { decodeJwt } from '@ppt/shared';
import { useMemo } from 'react';
import { useAuth } from '../../contexts';

/** Custom JWT claim shape we may receive once the backend signs it in. */
interface ExtendedJwtPayload {
  principal_kind?: string;
  capabilities?: ReadonlyArray<string>;
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
  const { user, getAccessToken } = useAuth();

  return useMemo<PrincipalCapabilitiesResult>(() => {
    const token = getAccessToken();
    const payload = (token ? decodeJwt(token) : null) as
      | (ReturnType<typeof decodeJwt> & ExtendedJwtPayload)
      | null;

    const claimKind = payload?.principal_kind?.toLowerCase();
    const role = user?.role?.toLowerCase();

    const isPlatformPrincipal =
      claimKind === 'platform' || (role !== undefined && PLATFORM_ROLE_FALLBACKS.has(role));

    // Capabilities will be empty until `me::capabilities` lands; that is
    // intentional — surfaces stay visible but action buttons stay hidden.
    const capabilities = (payload?.capabilities ?? []) as ReadonlyArray<Capability>;

    return { isPlatformPrincipal, capabilities };
  }, [user, getAccessToken]);
}
