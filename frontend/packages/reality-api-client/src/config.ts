/**
 * Runtime API configuration for @ppt/reality-api-client.
 *
 * Returns the URL at call time so it reads window.__ENV__ (injected by the
 * /env.js route handler in reality-web) rather than the build-time baked
 * value from process.env.NEXT_PUBLIC_API_URL.
 *
 * Worktree hosts (`wt-*.dev.rlt.sk` / `wt-*.staging.rlt.sk`) return an empty
 * string so callers produce relative URLs (`/api/v1/...`); the Next.js
 * rewrite in reality-web's next.config.js proxies those to the matching
 * api host, avoiding cross-origin CORS that the prod allowlist rejects.
 * Mirrors the implementation in apps/reality-web/src/lib/env.ts — kept in
 * sync deliberately rather than shared because this package targets
 * ppt-web/mobile too where the lib/env.ts module isn't reachable.
 */

const WORKTREE_HOST_RE = /^wt-.+\.(dev|staging)\.rlt\.sk$/;

function getRuntimeApiUrl(): string | undefined {
  if (typeof window === 'undefined') return undefined;
  // 1) Explicit /env.js injection wins (dedicated worktrees point at their own
  //    api.wt-<name>.* host, not api.rlt.sk).
  const env = (window as { __ENV__?: Record<string, string> }).__ENV__;
  if (env?.NEXT_PUBLIC_API_URL) {
    return env.NEXT_PUBLIC_API_URL;
  }
  // 2) Shared-mode worktrees with no env injection: relative URL hits the
  //    next.config.js rewrite to api.rlt.sk (avoids CORS).
  if (WORKTREE_HOST_RE.test(window.location.hostname)) {
    return '';
  }
  return undefined;
}

export function getApiBase(): string {
  const runtime = getRuntimeApiUrl();
  if (runtime !== undefined) return runtime;
  return process.env.NEXT_PUBLIC_API_URL ?? 'http://localhost:8081';
}
