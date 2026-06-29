/**
 * Low-level fetch client for @ppt/accounting-api-client.
 *
 * A thin typed wrapper over fetch that:
 *   - resolves the accounting-server base URL + /api/v1 prefix (config.ts),
 *   - attaches the Bearer JWT (resource-server auth, EPIC-ACC-17 STORY-002),
 *   - throws a structured AccountingApiError on non-2xx.
 *
 * This is the hand-written fallback transport the tanstack-query hooks use
 * until the generated hey-api client lands; the public hook surface stays
 * identical after generation, so screens don't change.
 */

import { apiUrl, authHeaders } from './config';

/** Structured error thrown for any non-2xx accounting-server response. */
export class AccountingApiError extends Error {
  readonly status: number;
  readonly body: unknown;

  constructor(status: number, message: string, body: unknown) {
    super(message);
    this.name = 'AccountingApiError';
    this.status = status;
    this.body = body;
  }

  /** True when the server rejected the JWT (missing/expired/tampered → 401). */
  get isUnauthorized(): boolean {
    return this.status === 401;
  }

  /** True when authz denied the action (insufficient TenantRole → 403). */
  get isForbidden(): boolean {
    return this.status === 403;
  }
}

interface RequestOptions {
  method?: 'GET' | 'POST' | 'PATCH' | 'PUT' | 'DELETE';
  /** JSON-serializable request body. */
  body?: unknown;
  /** Extra query params appended to the URL. */
  query?: Record<string, string | number | boolean | undefined>;
  signal?: AbortSignal;
}

function buildQuery(query?: RequestOptions['query']): string {
  if (!query) return '';
  const params = new URLSearchParams();
  for (const [key, value] of Object.entries(query)) {
    if (value !== undefined) params.set(key, String(value));
  }
  const qs = params.toString();
  return qs ? `?${qs}` : '';
}

async function parseBody(res: Response): Promise<unknown> {
  if (res.status === 204) return undefined;
  const text = await res.text();
  if (!text) return undefined;
  try {
    return JSON.parse(text);
  } catch {
    return text;
  }
}

/**
 * Perform an authenticated request against `/api/v1<path>` and return the
 * parsed JSON body typed as T. Throws AccountingApiError on non-2xx.
 */
export async function request<T>(path: string, opts: RequestOptions = {}): Promise<T> {
  const { method = 'GET', body, query, signal } = opts;
  const url = `${apiUrl(path)}${buildQuery(query)}`;

  const res = await fetch(url, {
    method,
    headers: authHeaders(),
    body: body === undefined ? undefined : JSON.stringify(body),
    signal,
  });

  const parsed = await parseBody(res);
  if (!res.ok) {
    const message =
      (typeof parsed === 'object' && parsed && 'message' in parsed
        ? String((parsed as { message: unknown }).message)
        : undefined) ?? `accounting-server ${method} ${path} failed: ${res.status}`;
    throw new AccountingApiError(res.status, message, parsed);
  }
  return parsed as T;
}
