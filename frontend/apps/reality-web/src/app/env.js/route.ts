import { type NextRequest, NextResponse } from 'next/server';
import { serializeForScript } from '@/lib/serialize-script';

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
 * host-inference fallback, so this is defence-in-depth.
 *
 * Load with: <script src="/env.js" /> (synchronous, no async/defer)
 * Cache-Control is no-store so each page load gets the current values.
 */
function inferFromHost(host: string | null): {
  apiUrl?: string;
  siteUrl?: string;
} {
  if (!host) return {};
  const bareHost = host.split(':')[0]?.toLowerCase();
  if (!bareHost) return {};
  if (bareHost === 'rlt.sk' || bareHost.endsWith('.rlt.sk')) {
    const isStaging = bareHost === 'staging.rlt.sk' || bareHost.endsWith('.staging.rlt.sk');
    return {
      apiUrl: isStaging ? 'https://api.staging.rlt.sk' : 'https://api.rlt.sk',
      siteUrl: `https://${bareHost}`,
    };
  }
  return {};
}

export function GET(req: NextRequest) {
  const env: Record<string, string> = {};
  const inferred = inferFromHost(req.headers.get('host'));

  if (process.env.NEXT_PUBLIC_API_URL) {
    env.NEXT_PUBLIC_API_URL = process.env.NEXT_PUBLIC_API_URL;
  } else if (inferred.apiUrl) {
    env.NEXT_PUBLIC_API_URL = inferred.apiUrl;
  }
  if (process.env.NEXT_PUBLIC_SITE_URL) {
    env.NEXT_PUBLIC_SITE_URL = process.env.NEXT_PUBLIC_SITE_URL;
  } else if (inferred.siteUrl) {
    env.NEXT_PUBLIC_SITE_URL = inferred.siteUrl;
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
