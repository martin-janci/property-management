/**
 * Authenticated JSON fetch against accounting-server.
 *
 * accounting-server is a resource server whose `AuthUser` extractor accepts
 * only `Authorization: Bearer <jwt>`; it has no cookie session. The invoice
 * and contact hooks each had a private copy of this helper that sent
 * `credentials: 'include'` and no Authorization header, so every list, detail
 * and mutation call was answered 401.
 */

import { getAccountingApiBase } from '@/lib/accountingClient';
import { getAccessToken } from '@/lib/auth';

export async function authedFetch<T>(path: string, init?: RequestInit): Promise<T> {
  const token = getAccessToken();
  const res = await fetch(`${getAccountingApiBase()}/api/v1${path}`, {
    ...init,
    credentials: 'include',
    // Spread `init` first: its `headers` must not replace the merged set below.
    headers: {
      'Content-Type': 'application/json',
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
      ...(init?.headers ?? {}),
    },
  });
  if (!res.ok) {
    throw new Error(`Request failed (${res.status})`);
  }
  if (res.status === 204) return undefined as T;
  return res.json() as Promise<T>;
}
