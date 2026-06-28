/**
 * Runtime API configuration for @ppt/accounting-api-client.
 *
 * The accounting SDK talks to accounting-server (a resource server on :8082)
 * which VALIDATES api-server-issued JWTs. This module resolves:
 *   1. the API base URL (NEXT_PUBLIC_ACCOUNTING_API_URL), and
 *   2. the Bearer JWT to attach to every request.
 *
 * Base URL resolution prefers a request-time value over the build-time baked
 * NEXT_PUBLIC_* (Next.js inlines those at build time, so Docker runtime env
 * vars otherwise have no effect on client components). We read
 * `window.__ENV__` first (deploy-injected), then the build-time env, then a
 * localhost:8082 default — the dev convention from CONTRACT §8.
 *
 * Mirrors the shape of reality-api-client/src/config.ts but kept independent
 * because this package targets accounting-web only.
 */

declare global {
  interface Window {
    __ENV__?: Record<string, string | undefined>;
  }
}

/** Default accounting-server origin for local development (CONTRACT §8). */
export const DEFAULT_ACCOUNTING_API_URL = 'http://localhost:8082';

/**
 * A function that returns the current Bearer token, or null when the user is
 * unauthenticated. accounting-web injects this once (via configureAccountingApi)
 * so it can read whatever token store the host app uses (localStorage key
 * `ppt_access_token`, an auth context, etc.) without this package depending on
 * a specific storage implementation.
 */
export type TokenProvider = () => string | null | undefined;

interface AccountingApiConfig {
  baseUrl?: string;
  getToken?: TokenProvider;
}

// Module-level singleton config, set once by the host app at startup.
const runtimeConfig: AccountingApiConfig = {};

/**
 * Configure the SDK once at app startup (e.g. in the (app) layout provider).
 * Idempotent — later calls override earlier values.
 */
export function configureAccountingApi(cfg: AccountingApiConfig): void {
  if (cfg.baseUrl !== undefined) runtimeConfig.baseUrl = cfg.baseUrl;
  if (cfg.getToken !== undefined) runtimeConfig.getToken = cfg.getToken;
}

/**
 * Clear all runtime config back to defaults. Primarily for tests; production
 * code calls configureAccountingApi once at startup and never resets.
 */
export function resetAccountingApi(): void {
  delete runtimeConfig.baseUrl;
  delete runtimeConfig.getToken;
}

function readRuntimeEnvUrl(): string | undefined {
  if (typeof window === 'undefined') return undefined;
  const env = window.__ENV__;
  return env?.NEXT_PUBLIC_ACCOUNTING_API_URL;
}

/**
 * Resolve the accounting-server base URL.
 *
 * Order of precedence:
 *   1. an explicit baseUrl passed to configureAccountingApi (host app override)
 *   2. window.__ENV__.NEXT_PUBLIC_ACCOUNTING_API_URL (deploy-time injection)
 *   3. process.env.NEXT_PUBLIC_ACCOUNTING_API_URL (build-time bake)
 *   4. http://localhost:8082 (local dev default — CONTRACT §8)
 *
 * The returned value never has a trailing slash.
 */
export function getApiBase(): string {
  const candidate =
    runtimeConfig.baseUrl ??
    readRuntimeEnvUrl() ??
    process.env.NEXT_PUBLIC_ACCOUNTING_API_URL ??
    DEFAULT_ACCOUNTING_API_URL;
  return candidate.replace(/\/+$/, '');
}

/** API prefix for all accounting-server REST routes (CONTRACT §5e). */
export const API_PREFIX = '/api/v1';

/** Build a fully-qualified URL for an accounting-server REST path. */
export function apiUrl(path: string): string {
  const suffix = path.startsWith('/') ? path : `/${path}`;
  return `${getApiBase()}${API_PREFIX}${suffix}`;
}

/** Read the current Bearer token via the host-app-provided token provider. */
export function getAuthToken(): string | null {
  const token = runtimeConfig.getToken?.();
  return token ?? null;
}

/**
 * Build the request headers for an authenticated accounting-server call.
 * Always JSON; attaches `Authorization: Bearer <jwt>` when a token is present.
 * accounting-server is a resource server: it rejects a missing/expired token
 * with 401 (EPIC-ACC-17 STORY-002).
 */
export function authHeaders(extra?: HeadersInit): Headers {
  const headers = new Headers(extra);
  if (!headers.has('Content-Type')) headers.set('Content-Type', 'application/json');
  const token = getAuthToken();
  if (token) headers.set('Authorization', `Bearer ${token}`);
  return headers;
}
