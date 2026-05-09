/**
 * Runtime environment configuration for reality-web.
 *
 * Next.js bakes NEXT_PUBLIC_* variables into the client bundle at build time,
 * which means Docker runtime env vars have no effect on client components.
 * This module reads from window.__ENV__ (injected by the root server layout at
 * request time) so the API URL can be configured per-deployment without
 * rebuilding the image.
 *
 * Usage in client components: import { getApiBase } from '@/lib/env'
 * Server components can read process.env directly (they run at request time).
 */

declare global {
  interface Window {
    __ENV__?: {
      NEXT_PUBLIC_API_URL?: string;
      NEXT_PUBLIC_SITE_URL?: string;
    };
  }
}

/**
 * Returns the reality-server API base URL.
 * Evaluated at call time so it picks up window.__ENV__ after script injection.
 */
export function getApiBase(): string {
  if (typeof window !== 'undefined' && window.__ENV__?.NEXT_PUBLIC_API_URL) {
    return window.__ENV__.NEXT_PUBLIC_API_URL;
  }
  return process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8081';
}

/**
 * Returns the public site URL (used for canonical links and OG metadata).
 */
export function getSiteUrl(): string {
  if (typeof window !== 'undefined' && window.__ENV__?.NEXT_PUBLIC_SITE_URL) {
    return window.__ENV__.NEXT_PUBLIC_SITE_URL;
  }
  return process.env.NEXT_PUBLIC_SITE_URL || 'http://localhost:3001';
}
