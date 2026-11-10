/**
 * Shared server-side API-base / site-URL resolution for reality-web SSR paths.
 *
 * The host-inference + resolve logic below was previously copied verbatim into
 * three separate server call sites:
 *   - src/app/[locale]/listings/[slug]/page.tsx  (listing fetch)
 *   - src/lib/layout.ts                          (resolved-layout fetch)
 *   - src/app/env.js/route.ts                    (window.__ENV__ bootstrap)
 * This module is now the single source of truth for all three.
 *
 * The client-side counterpart in src/lib/env.ts (which reads window.location /
 * window.__ENV__) is intentionally kept separate: it runs in the browser, never
 * sees a Host header carrying a port or arbitrary case, and has its own
 * worktree-rewrite rules that don't apply to SSR fetches.
 */

/**
 * Reduce an inbound Host header to a bare, lowercased hostname.
 *
 * A Host header may carry a port (`rlt.sk:443`) and arbitrary case, unlike the
 * browser's `window.location.hostname`, so strip the port and lowercase before
 * matching. Returns null when there is nothing to match on.
 */
function bareHostOf(host: string | null): string | null {
  const bareHost = host?.split(':')[0]?.toLowerCase();
  return bareHost ? bareHost : null;
}

/**
 * Infer the reality-server API base URL from the inbound Host header.
 *
 * Returns null for any host outside the known *.rlt.sk topology, so a stray
 * preview/localhost host never silently resolves to a prod API.
 */
export function inferApiBaseFromHost(host: string | null): string | null {
  const bareHost = bareHostOf(host);
  if (!bareHost) return null;
  if (bareHost === 'rlt.sk' || bareHost.endsWith('.rlt.sk')) {
    if (bareHost === 'staging.rlt.sk' || bareHost.endsWith('.staging.rlt.sk')) {
      return 'https://api.staging.rlt.sk';
    }
    return 'https://api.rlt.sk';
  }
  return null;
}

/**
 * Infer the public site URL from the inbound Host header, for the same known
 * *.rlt.sk topology. Returns null otherwise. Always https — these hosts are
 * TLS-terminated at the edge.
 */
export function inferSiteUrlFromHost(host: string | null): string | null {
  const bareHost = bareHostOf(host);
  if (!bareHost) return null;
  if (bareHost === 'rlt.sk' || bareHost.endsWith('.rlt.sk')) {
    return `https://${bareHost}`;
  }
  return null;
}

/**
 * Resolve the reality-server API base URL for an SSR fetch.
 *
 * Order (first match wins):
 *   1. API_INTERNAL_URL      — Docker-network internal URL when set
 *   2. host inference        — authoritative for known *.rlt.sk hosts, so a
 *                              baked-in/misconfigured NEXT_PUBLIC_API_URL
 *                              (e.g. `http://localhost:8081`) can't take down
 *                              SSR on staging/prod
 *   3. NEXT_PUBLIC_API_URL   — public env
 *   4. http://localhost:8081 — dev-only fallback
 */
export function resolveApiBase(host: string | null): string {
  if (process.env.API_INTERNAL_URL) return process.env.API_INTERNAL_URL;
  const inferred = inferApiBaseFromHost(host);
  if (inferred) return inferred;
  return process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8081';
}
