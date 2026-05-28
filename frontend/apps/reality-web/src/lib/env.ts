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
/** Worktree hosts: `wt-<name>.dev.rlt.sk` or `wt-<name>.staging.rlt.sk`. */
const WORKTREE_HOST_RE = /^wt-.+\.(dev|staging)\.rlt\.sk$/;

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
 *
 * Worktree hosts return an empty string so callers produce relative URLs
 * (e.g. `/api/v1/sso/session`); the Next.js rewrite in next.config.js
 * proxies those to api.rlt.sk, avoiding the cross-origin CORS preflight
 * that would otherwise fail (worktree subdomains aren't in the prod CORS
 * allowlist). Empty-string return is intentional and load-bearing —
 * downstream callers do `${getApiBase()}/api/v1/...` which becomes a
 * relative URL when the base is empty.
 */
export function getApiBase(): string {
  if (typeof window !== 'undefined') {
    const host = window.location.hostname;
    const onWorktree = WORKTREE_HOST_RE.test(host);
    const envUrl = window.__ENV__?.NEXT_PUBLIC_API_URL;

    if (onWorktree) {
      // Worktree dedicated backend (api.wt-*) is the one case where env
      // wins over inference — inference doesn't know about worktree apis.
      if (envUrl) {
        try {
          if (new URL(envUrl).hostname.startsWith('api.wt-')) return envUrl;
        } catch {
          // malformed env value — ignore
        }
      }
      return ''; // shared worktree → relative URL via next.config.js rewrite
    }

    // For known *.rlt.sk hosts, host inference is authoritative. A
    // misconfigured env (e.g. a baked-in `http://localhost:8081` from a
    // bad container build) would otherwise leak through to the browser
    // and every request would ERR_CONNECTION_REFUSED in the user's tab.
    const inferred = inferApiBaseFromHost(host);
    if (inferred) return inferred;

    if (envUrl) return envUrl;
  }
  return process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8081';
}

/**
 * Returns the public site URL (used for canonical links and OG metadata).
 */
export function getSiteUrl(): string {
  if (typeof window !== 'undefined') {
    // Same rationale as getApiBase: for known *.rlt.sk hosts the page's
    // own origin is the authoritative answer, even if env says otherwise.
    const inferred = inferSiteUrlFromHost(window.location.hostname, window.location.protocol);
    if (inferred) return inferred;
    if (window.__ENV__?.NEXT_PUBLIC_SITE_URL) {
      return window.__ENV__.NEXT_PUBLIC_SITE_URL;
    }
  }
  return process.env.NEXT_PUBLIC_SITE_URL || 'http://localhost:3001';
}
