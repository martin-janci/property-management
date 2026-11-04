/**
 * Session helpers shared by the auth context and request helpers.
 *
 * The api-server login / refresh responses are camelCase
 * (`accessToken`, `refreshToken`, `tenants[]`) and the access token carries
 * neither a role nor a `tenant_id` claim. The active organisation therefore has
 * to be remembered on the device after login, so tenant-scoped requests can
 * send `X-Tenant-ID`.
 */
import * as SecureStore from 'expo-secure-store';
import { decodeJwtPayload, extractTenantId } from './jwt';

/** SecureStore key holding the organisation chosen at login. */
export const TENANT_ID_KEY = 'ppt_tenant_id';

export type AppRole = 'owner' | 'tenant' | 'resident' | 'manager' | 'admin';

export interface SessionTenant {
  tenantId: string;
  tenantName?: string;
  role?: string;
}

export interface SessionUser {
  id: string;
  email: string;
  firstName: string;
  lastName: string;
  role: AppRole;
  organizationId?: string;
  organizationName?: string;
}

function str(value: unknown): string | undefined {
  return typeof value === 'string' && value.length > 0 ? value : undefined;
}

/**
 * Read the token pair from a login / refresh response. Accepts the
 * api-server's camelCase shape and the legacy snake_case one.
 */
export function readTokenPair(data: unknown): { accessToken: string; refreshToken: string } | null {
  if (!data || typeof data !== 'object') return null;
  const d = data as Record<string, unknown>;
  const accessToken = str(d.accessToken) ?? str(d.access_token);
  const refreshToken = str(d.refreshToken) ?? str(d.refresh_token);
  if (!accessToken || !refreshToken) return null;
  return { accessToken, refreshToken };
}

/** The tenants listed on a login response (`tenants[]`), normalised. */
export function readTenants(data: unknown): SessionTenant[] {
  if (!data || typeof data !== 'object') return [];
  const raw = (data as Record<string, unknown>).tenants;
  if (!Array.isArray(raw)) return [];
  return raw.flatMap((entry) => {
    if (!entry || typeof entry !== 'object') return [];
    const e = entry as Record<string, unknown>;
    const tenantId = str(e.tenantId) ?? str(e.tenant_id);
    if (!tenantId) return [];
    return [
      {
        tenantId,
        tenantName: str(e.tenantName) ?? str(e.tenant_name),
        role: str(e.role),
      },
    ];
  });
}

/** Collapse the api-server's fine-grained roles onto the app's role set. */
export function toAppRole(role: string | undefined): AppRole {
  switch ((role ?? '').toLowerCase()) {
    case 'super_admin':
    case 'platform_admin':
    case 'org_admin':
    case 'admin':
      return 'admin';
    case 'manager':
    case 'property_manager':
    case 'technical_manager':
      return 'manager';
    case 'owner':
    case 'owner_delegate':
      return 'owner';
    case 'tenant':
      return 'tenant';
    default:
      return 'resident';
  }
}

/**
 * Build the signed-in user from a login response. Uses `data.user` when the
 * server embeds one, otherwise the access-token claims plus the active tenant
 * (the token's `tenant_id` claim if present, else the first listed tenant).
 */
export function buildSessionUser(
  data: unknown,
  accessToken: string
): { user: SessionUser; tenantId: string | null } {
  const claims = decodeJwtPayload(accessToken) ?? {};
  const tenants = readTenants(data);
  const claimTenant = str(claims.tenant_id);
  const active = tenants.find((t) => t.tenantId === claimTenant) ?? tenants[0];
  const tenantId = claimTenant ?? active?.tenantId ?? null;

  const embedded =
    data && typeof data === 'object'
      ? ((data as Record<string, unknown>).user as Record<string, unknown> | undefined)
      : undefined;

  if (embedded && typeof embedded === 'object' && str(embedded.id)) {
    const user = { ...embedded } as unknown as SessionUser;
    if (!user.organizationId && tenantId) user.organizationId = tenantId;
    if (!user.organizationName && active?.tenantName) user.organizationName = active.tenantName;
    return { user, tenantId };
  }

  const name = str(claims.name) ?? '';
  const [first = '', ...rest] = name.split(/\s+/).filter(Boolean);

  const user: SessionUser = {
    id: str(claims.sub) ?? '',
    email: str(claims.email) ?? '',
    firstName: first,
    lastName: rest.join(' '),
    role: toAppRole(str(claims.role) ?? active?.role),
    organizationId: tenantId ?? undefined,
    organizationName: active?.tenantName,
  };
  return { user, tenantId };
}

/**
 * The organisation to scope requests to: the token's `tenant_id` claim when
 * present, otherwise the one remembered at login.
 */
export async function resolveTenantId(accessToken: string | null): Promise<string | null> {
  const fromClaim = accessToken ? extractTenantId(accessToken) : null;
  if (fromClaim) return fromClaim;
  try {
    return await SecureStore.getItemAsync(TENANT_ID_KEY);
  } catch {
    return null;
  }
}
