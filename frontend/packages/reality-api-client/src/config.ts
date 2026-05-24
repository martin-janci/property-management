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

function inferApiBaseFromHost(host: string): string | null {
  if (host === 'rlt.sk' || host.endsWith('.rlt.sk')) {
    if (host.endsWith('.staging.rlt.sk') || host === 'staging.rlt.sk') {
      return 'https://api.staging.rlt.sk';
    }
    return 'https://api.rlt.sk';
  }
  return null;
}

function getRuntimeApiUrl(): string | undefined {
  if (typeof window === 'undefined') return undefined;
  const env = (window as { __ENV__?: Record<string, string> }).__ENV__;
  const envUrl = env?.NEXT_PUBLIC_API_URL;
  const host = window.location.hostname;
  const onWorktree = WORKTREE_HOST_RE.test(host);

  if (onWorktree) {
    // Dedicated worktree backend is the one case where env wins over
    // host inference — inference can't know about wt-* api hosts.
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
  // misconfigured env (e.g. a baked-in `http://localhost:8081`) would
  // otherwise leak to the browser and break every request.
  const inferred = inferApiBaseFromHost(host);
  if (inferred) return inferred;

  if (envUrl) return envUrl;
  return undefined;
}

export function getApiBase(): string {
  const runtime = getRuntimeApiUrl();
  if (runtime !== undefined) return runtime;
  return process.env.NEXT_PUBLIC_API_URL ?? 'http://localhost:8081';
}
