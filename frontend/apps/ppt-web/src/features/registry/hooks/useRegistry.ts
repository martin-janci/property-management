/**
 * Registry Hooks
 *
 * React hooks for building-registry data (pets, vehicles, rules) using
 * @ppt/api-client (Epic 57).
 *
 * Follows the canonical `useBuildings.ts` pattern: the API client is rebuilt
 * via `useMemo` keyed on the current access token so every request carries an
 * `Authorization: Bearer <token>` header and the client rotates when the token
 * refreshes — instead of a module-scope singleton that is created once at
 * import time with no auth context (which shipped every registry request
 * unauthenticated → 401).
 */

import { createRegistryApi, createRegistryHooks, getToken } from '@ppt/api-client';
import { useMemo } from 'react';

// API base URL from environment
const API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? '';

/**
 * Creates a registry API client with the current auth token.
 *
 * `createRegistryApi` bakes the `Authorization: Bearer <token>` header (and
 * `X-Tenant-ID` when present) into the returned client at construction time,
 * so the client must be re-created whenever the token rotates — see
 * {@link useRegistryApi}.
 */
function getRegistryApi(accessToken: string | undefined) {
  return createRegistryApi({
    baseUrl: API_BASE_URL,
    accessToken,
  });
}

/**
 * Hook to get registry API hooks with current auth context.
 *
 * Creates a memoized API client and returns TanStack Query hooks for registry
 * operations. The memo is keyed on the current access token so a token refresh
 * (rotation) rebuilds the client with the fresh Bearer header instead of
 * sending a stale (or missing) token for the rest of the component's lifetime.
 */
export function useRegistryApi() {
  const token = getToken() ?? undefined;
  const api = useMemo(() => getRegistryApi(token), [token]);
  return useMemo(() => createRegistryHooks(api), [api]);
}
