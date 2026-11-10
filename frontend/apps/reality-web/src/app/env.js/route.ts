import { type NextRequest, NextResponse } from 'next/server';
import { serializeForScript } from '@/lib/serialize-script';
import { inferApiBaseFromHost, inferSiteUrlFromHost } from '@/lib/server-api-base';

// Always dynamic — this route's whole purpose is to expose runtime env vars.
export const dynamic = 'force-dynamic';

/**
 * Serves runtime environment configuration as a JavaScript snippet.
 *
 * Resolution order (first match wins):
 *  1. process.env (build-time baked or container env)
 *  2. host-based inference for known *.rlt.sk topology — covers worktree dev
 *     deployments where the deploy-server doesn't currently inject env vars
 *     when starting the reality-web container (shared backend mode).
 *
 * Client code reads window.__ENV__ via src/lib/env.ts with the same
 * host-inference fallback, so this is defence-in-depth. The host-inference
 * primitives themselves live in src/lib/server-api-base.ts, shared with the
 * listing / resolved-layout SSR fetches.
 *
 * Load with: <script src="/env.js" /> (synchronous, no async/defer)
 * Cache-Control is no-store so each page load gets the current values.
 */
export function GET(req: NextRequest) {
  const env: Record<string, string> = {};
  const host = req.headers.get('host');
  const inferredApiUrl = inferApiBaseFromHost(host);
  const inferredSiteUrl = inferSiteUrlFromHost(host);

  if (process.env.NEXT_PUBLIC_API_URL) {
    env.NEXT_PUBLIC_API_URL = process.env.NEXT_PUBLIC_API_URL;
  } else if (inferredApiUrl) {
    env.NEXT_PUBLIC_API_URL = inferredApiUrl;
  }
  if (process.env.NEXT_PUBLIC_SITE_URL) {
    env.NEXT_PUBLIC_SITE_URL = process.env.NEXT_PUBLIC_SITE_URL;
  } else if (inferredSiteUrl) {
    env.NEXT_PUBLIC_SITE_URL = inferredSiteUrl;
  }

  // Escape '<'/'>'/'&'/U+2028/U+2029 so no value can break out of the script
  // context (defence-in-depth; content is env vars). Shares the same hardened
  // primitive as the layout tenant bootstrap — a bare `.replace(/</g, ...)`
  // here previously missed the two Unicode line/paragraph separators.
  const safeJson = serializeForScript(env);

  return new NextResponse(`window.__ENV__=${safeJson};`, {
    headers: {
      'Content-Type': 'application/javascript; charset=utf-8',
      'Cache-Control': 'no-store',
    },
  });
}
