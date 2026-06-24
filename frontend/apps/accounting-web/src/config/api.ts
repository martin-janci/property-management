/**
 * accounting-web API configuration.
 *
 * Single place that resolves the accounting-server base URL for client code.
 * accounting-server is a resource server on :8082 that validates
 * api-server-issued JWTs (EPIC-ACC-17). The base URL comes from
 * NEXT_PUBLIC_ACCOUNTING_API_URL; in local dev it defaults to
 * http://localhost:8082 (CONTRACT §8 / preamble).
 *
 * Resolution order (so Docker/worktree deploys can override the build-time
 * baked NEXT_PUBLIC_* value):
 *   1. window.__ENV__.NEXT_PUBLIC_ACCOUNTING_API_URL (deploy-time injection)
 *   2. process.env.NEXT_PUBLIC_ACCOUNTING_API_URL (build-time bake / server)
 *   3. http://localhost:8082 (local dev default)
 *
 * The SDK (@ppt/accounting-api-client) has its own equivalent resolver; this
 * module is the accounting-web-side mirror and is what we feed into
 * configureAccountingApi() so both agree on a single base URL.
 */

declare global {
  interface Window {
    __ENV__?: Record<string, string | undefined>;
  }
}

/** Default accounting-server origin for local development (CONTRACT §8). */
export const DEFAULT_ACCOUNTING_API_URL = 'http://localhost:8082';

function readRuntimeEnvUrl(): string | undefined {
  if (typeof window === 'undefined') return undefined;
  return window.__ENV__?.NEXT_PUBLIC_ACCOUNTING_API_URL;
}

/** Resolve the accounting-server base URL (no trailing slash). */
export function getAccountingApiBase(): string {
  const candidate =
    readRuntimeEnvUrl() ??
    process.env.NEXT_PUBLIC_ACCOUNTING_API_URL ??
    DEFAULT_ACCOUNTING_API_URL;
  return candidate.replace(/\/+$/, '');
}
