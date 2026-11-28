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

/**
 * The user as persisted to SecureStore (`ppt_user`) and restored on cold start
 * / biometric unlock. Superset of {@link SessionUser}: it also carries the
 * profile fields `updateUser()` merges in after login. Structurally matches the
 * auth context's `User`.
 */
export interface StoredUser extends SessionUser {
  buildingId?: string;
  unitId?: string;
  avatarUrl?: string;
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
    // Narrow each field off the response body instead of casting the whole
    // object into a SessionUser: a non-string / missing field (or junk key)
    // must not be trusted verbatim, and the role must still pass through
    // toAppRole() rather than leak an arbitrary server string.
    const user: SessionUser = {
      id: str(embedded.id) ?? '',
      email: str(embedded.email) ?? str(claims.email) ?? '',
      firstName: str(embedded.firstName) ?? '',
      lastName: str(embedded.lastName) ?? '',
      role: toAppRole(str(embedded.role) ?? str(claims.role) ?? active?.role),
      organizationId: str(embedded.organizationId) ?? tenantId ?? undefined,
      organizationName: str(embedded.organizationName) ?? active?.tenantName,
    };
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
 * Normalise an arbitrary user-shaped value into the canonical {@link StoredUser}
 * shape: narrow every field off the object (dropping non-strings and junk keys)
 * and run `role` through {@link toAppRole} so no arbitrary / privileged role can
 * leak through. Returns `null` when the value is not an object or has no id.
 *
 * This is the single source of truth for user-shape normalisation on the
 * device. Both the cold-start / biometric restore path ({@link parseStoredUser})
 * and the auth context's `updateUser()` merge path run their result through it,
 * so the in-memory session, the persisted `ppt_user` blob, and a later restored
 * session can never drift apart — a field merged in via `updateUser()` is shaped
 * exactly as the restore path would re-shape it, instead of being stored
 * verbatim.
 */
export function normalizeUser(raw: unknown): StoredUser | null {
  if (!raw || typeof raw !== 'object') return null;
  const u = raw as Record<string, unknown>;
  const id = str(u.id);
  if (!id) return null;
  return {
    id,
    email: str(u.email) ?? '',
    firstName: str(u.firstName) ?? '',
    lastName: str(u.lastName) ?? '',
    role: toAppRole(str(u.role)),
    buildingId: str(u.buildingId),
    unitId: str(u.unitId),
    avatarUrl: str(u.avatarUrl),
    organizationId: str(u.organizationId),
    organizationName: str(u.organizationName),
  };
}

/**
 * Parse and validate the user persisted to SecureStore (`ppt_user`).
 *
 * The stored blob is on-device, mutable state restored on cold start and
 * biometric unlock, so it must be validated like any other untrusted input
 * rather than cast straight into a `User`. In particular the `role` is run
 * through {@link toAppRole} — exactly as the login path does — so a tampered or
 * drifted blob can never grant an arbitrary / privileged role, and every other
 * field is narrowed off the object (non-string and junk keys dropped). Returns
 * `null` when the blob is missing, not JSON, not an object, or has no id (an
 * id-less user would restore a half-formed session).
 */
export function parseStoredUser(json: string | null | undefined): StoredUser | null {
  if (!json) return null;
  let raw: unknown;
  try {
    raw = JSON.parse(json);
  } catch {
    return null;
  }
  return normalizeUser(raw);
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
