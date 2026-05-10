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
 * Resolve the reality-server base URL when no explicit env injection happened.
 *
 * The deploy-server worktree path doesn't set NEXT_PUBLIC_API_URL on the dev
 * frontend container (see backend/servers/deploy-server/src/api/worktree.rs);
 * the production blue-green deployer does. Without this, every wt-*.dev.rlt.sk
 * page falls back to `http://localhost:8081` and every API call ERR_CONNECTION_REFUSEDs
 * in the user's browser. Until the deploy-server gains env injection, we
 * derive the API host from the public host the page is served on.
 *
 * Mapping is deliberately narrow — only known *.rlt.sk topology — so a stray
 * vercel.app preview or localhost dev never silently hits prod.
 */
function inferApiBaseFromHost(host: string): string | null {
  if (host === 'rlt.sk' || host.endsWith('.rlt.sk')) {
    if (host.endsWith('.staging.rlt.sk') || host === 'staging.rlt.sk') {
      return 'https://api.staging.rlt.sk';
    }
    return 'https://api.rlt.sk';
  }
  return null;
}

function inferSiteUrlFromHost(host: string, protocol: string): string | null {
  if (host === 'rlt.sk' || host.endsWith('.rlt.sk')) {
    return `${protocol}//${host}`;
  }
  return null;
}

/**
 * Returns the reality-server API base URL.
 * Evaluated at call time so it picks up window.__ENV__ after script injection.
 */
export function getApiBase(): string {
  if (typeof window !== 'undefined') {
    if (window.__ENV__?.NEXT_PUBLIC_API_URL) {
      return window.__ENV__.NEXT_PUBLIC_API_URL;
    }
    const inferred = inferApiBaseFromHost(window.location.hostname);
    if (inferred) return inferred;
  }
  return process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8081';
}

/**
 * Returns the public site URL (used for canonical links and OG metadata).
 */
export function getSiteUrl(): string {
  if (typeof window !== 'undefined') {
    if (window.__ENV__?.NEXT_PUBLIC_SITE_URL) {
      return window.__ENV__.NEXT_PUBLIC_SITE_URL;
    }
    const inferred = inferSiteUrlFromHost(window.location.hostname, window.location.protocol);
    if (inferred) return inferred;
  }
  return process.env.NEXT_PUBLIC_SITE_URL || 'http://localhost:3001';
}
