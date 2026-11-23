/**
 * Minimal `fetch`-based query/mutation hooks for the mobile app.
 *
 * Why not `@ppt/api-client`?
 *
 * - The web-side `@ppt/api-client` modules call `fetch` with relative paths
 *   like `/api/v1/outages`. That works under Vite's dev proxy in the browser
 *   but fails on React Native, where every URL must be absolute.
 * - The mobile `AuthContext` already manages the bearer token, and
 *   `config/api.ts::getApiBaseUrl()` returns the correct platform-aware
 *   base URL (`http://10.0.2.2:8080` on Android, `http://localhost:8080`
 *   on iOS, the production URL otherwise).
 *
 * Until the api-client is refactored to accept a base URL, screens use
 * these hooks for direct, typed JSON access against the api-server.
 *
 * Both hooks integrate with TanStack Query so screens get caching,
 * refetch-on-mount, retry, and the standard `{ data, isLoading, error }`
 * surface for free.
 */

import { type UseQueryOptions, useMutation, useQuery } from '@tanstack/react-query';
import * as SecureStore from 'expo-secure-store';
import { getApiBaseUrl } from '../config/api';
import { readTokenPair, resolveTenantId } from '../utils/session';

export { getTenantId };

const ACCESS_TOKEN_KEY = 'ppt_access_token';
const REFRESH_TOKEN_KEY = 'ppt_refresh_token';

/** Read the current bearer token without forcing a re-render. */
async function getAccessToken(): Promise<string | null> {
  try {
    return await SecureStore.getItemAsync(ACCESS_TOKEN_KEY);
  } catch {
    return null;
  }
}

/**
 * In-flight refresh, shared across concurrent 401s.
 *
 * Several queries can 401 at once when a short-lived access token lapses;
 * without this, each would POST `/auth/refresh` independently, racing to rotate
 * the refresh token (and, since the server one-time-uses it, invalidating one
 * another). Collapsing them onto a single promise means one refresh serves
 * every waiter, then the slot is cleared so the next lapse refreshes again.
 */
let inFlightRefresh: Promise<string | null> | null = null;

/**
 * Exchange the stored refresh token for a fresh access token on a 401.
 *
 * Returns the new access token (also persisted, together with the rotated
 * refresh token) or `null` when refresh isn't possible — no stored refresh
 * token, a non-OK `/auth/refresh` response, an unparseable body, or a keystore
 * error. On `null` the caller surfaces the original 401; tearing the session
 * down (logout + cache clear) stays `AuthContext`'s responsibility, driven off
 * that surfaced error / the next auth-gate evaluation.
 */
async function refreshAccessToken(): Promise<string | null> {
  if (inFlightRefresh) return inFlightRefresh;

  const run = (async (): Promise<string | null> => {
    try {
      const storedRefreshToken = await SecureStore.getItemAsync(REFRESH_TOKEN_KEY);
      if (!storedRefreshToken) return null;

      const response = await fetch(`${getApiBaseUrl()}/api/v1/auth/refresh`, {
        method: 'POST',
        headers: { Accept: 'application/json', 'Content-Type': 'application/json' },
        body: JSON.stringify({ refreshToken: storedRefreshToken }),
      });
      if (!response.ok) return null;

      const tokens = readTokenPair(await response.json().catch(() => null));
      if (!tokens) return null;

      await SecureStore.setItemAsync(ACCESS_TOKEN_KEY, tokens.accessToken);
      await SecureStore.setItemAsync(REFRESH_TOKEN_KEY, tokens.refreshToken);
      return tokens.accessToken;
    } catch {
      return null;
    }
  })();

  inFlightRefresh = run.finally(() => {
    inFlightRefresh = null;
  });
  return inFlightRefresh;
}

/** Pull the tenant id from the current access token, or null if missing. */
async function getTenantId(): Promise<string | null> {
  const token = await getAccessToken();
  if (!token) return null;
  return resolveTenantId(token);
}

interface RequestOptions {
  signal?: AbortSignal;
  headers?: Record<string, string>;
}

/**
 * Send an authenticated JSON request to the api-server.
 *
 * On a 401 (an expired/invalid access token) the request transparently
 * refreshes the token once and retries, so a lapsed bearer no longer leaves the
 * user stuck on a failed screen. The retry is capped at one attempt
 * (`allowRefresh` guards against a refresh→retry→401 loop), and the refresh
 * itself is single-flighted across concurrent requests (see
 * `refreshAccessToken`). When refresh can't recover the session the original
 * 401 is surfaced unchanged.
 */
export async function apiRequest<T>(
  path: string,
  init: Omit<RequestInit, 'body'> & { body?: unknown } = {},
  options: RequestOptions = {}
): Promise<T> {
  return requestWithRefresh<T>(path, init, options, true);
}

async function requestWithRefresh<T>(
  path: string,
  init: Omit<RequestInit, 'body'> & { body?: unknown },
  options: RequestOptions,
  allowRefresh: boolean
): Promise<T> {
  const accessToken = await getAccessToken();
  const tenantId = await getTenantId();
  const url = path.startsWith('http') ? path : `${getApiBaseUrl()}${path}`;

  const headers: Record<string, string> = {
    Accept: 'application/json',
    ...(init.body !== undefined ? { 'Content-Type': 'application/json' } : {}),
    ...(accessToken ? { Authorization: `Bearer ${accessToken}` } : {}),
    // Tenant-scoped api-server routes (RlsConnection extractor) require
    // the X-Tenant-ID header. The value comes from the JWT's `tenant_id`
    // claim so requests stay scoped to whichever org the user logged into.
    ...(tenantId ? { 'X-Tenant-ID': tenantId } : {}),
    ...(init.headers as Record<string, string> | undefined),
    ...options.headers,
  };

  const response = await fetch(url, {
    ...init,
    headers,
    signal: options.signal,
    body: init.body !== undefined ? JSON.stringify(init.body) : undefined,
  });

  // Expired/invalid bearer: refresh once and retry with the fresh token. We
  // skip this when the caller sent no token at all (an anonymous request that
  // legitimately 401s has nothing to refresh) and on the already-retried pass.
  if (response.status === 401 && allowRefresh && accessToken) {
    const refreshed = await refreshAccessToken();
    if (refreshed) {
      return requestWithRefresh<T>(path, init, options, false);
    }
  }

  if (!response.ok) {
    let message = `HTTP ${response.status}`;
    try {
      const body = await response.json();
      if (body?.message) message = body.message;
    } catch {
      // body wasn't JSON — fall back to the status code message
    }
    throw new Error(message);
  }

  if (response.status === 204) {
    return undefined as T;
  }

  return (await response.json()) as T;
}

/**
 * GET a JSON endpoint via TanStack Query.
 *
 * Pass a `queryKey` that uniquely identifies the resource (path + params)
 * so the cache can dedupe and invalidate correctly.
 */
export function useApiQuery<T>(
  queryKey: readonly unknown[],
  path: string,
  options?: Omit<UseQueryOptions<T, Error, T, readonly unknown[]>, 'queryKey' | 'queryFn'>
) {
  return useQuery<T, Error>({
    queryKey,
    queryFn: ({ signal }) => apiRequest<T>(path, { method: 'GET' }, { signal }),
    ...options,
  });
}

/**
 * Resolve the current org/tenant id (the JWT `tenant_id` claim) reactively.
 *
 * Some api-server routes need the tenant id as an explicit query param in
 * addition to the `X-Tenant-ID` header — most notably `GET /api/v1/buildings`,
 * whose `ListBuildingsQuery.organization_id` is a required, non-defaulted
 * `Uuid` (the Axum `Query` extractor 400s without it, and `list_buildings`
 * 403s unless it equals the RLS tenant). Screens use this hook to build that
 * `?organization_id=<id>` param and to key the cache on it.
 *
 * Returns `{ tenantId, isLoading }`. `tenantId` is `null` until the token is
 * read (or if the user is unauthenticated / the token lacks the claim).
 */
export function useTenantId(): { tenantId: string | null; isLoading: boolean } {
  const { data, isLoading } = useQuery<string | null, Error>({
    queryKey: ['auth', 'tenant-id'],
    queryFn: () => getTenantId(),
    staleTime: Number.POSITIVE_INFINITY,
  });
  return { tenantId: data ?? null, isLoading };
}

/** POST/PUT/PATCH/DELETE a JSON body via TanStack Mutation. */
export function useApiMutation<TData, TVariables = void>(
  path: string | ((vars: TVariables) => string),
  method: 'POST' | 'PUT' | 'PATCH' | 'DELETE' = 'POST'
) {
  return useMutation<TData, Error, TVariables>({
    mutationFn: (variables) => {
      const resolvedPath = typeof path === 'function' ? path(variables) : path;
      return apiRequest<TData>(resolvedPath, {
        method,
        body: variables as unknown,
      });
    },
  });
}
