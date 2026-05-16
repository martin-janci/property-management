import { type NextRequest, NextResponse } from 'next/server';
import createMiddleware from 'next-intl/middleware';
import { routing } from './i18n/routing';
import { renderKillSwitchHtml } from './lib/kill-switch';

const intlMiddleware = createMiddleware(routing);

/**
 * Phase 3: tenant-aware middleware.
 *
 * The next-intl middleware still does the locale routing — we wrap it and
 * tag the response with two headers so RSC layers + the cache layer can
 * branch on the resolved tenant without re-fetching `/tenant-config`:
 *
 * - `x-tenant-host`: the inbound host the SSR layer should pin its
 *   `getTenantConfig` lookup to. (Reading the host inside RSC is allowed
 *   but an explicit header makes the data-flow visible.)
 * - `x-tenant-pathname`: the path post-locale-rewrite, useful for
 *   cache-key construction in revalidate-tag flows.
 *
 * # N8: building_disabled returns a real 503
 *
 * When the tenant has `building_disabled.enabled = true` we short-circuit
 * with HTTP 503 BEFORE any rendering happens. Doing this in the layout
 * (the previous behaviour) returned 200 — CDN, monitoring, and SEO all
 * mistreated the kill page as a legitimate response. The fetch is cheap
 * (a single GET against api-server with `next: { revalidate: 60 }`) and
 * happens on every request anyway via `getTenantConfig` in the SSR layer;
 * we just learn the answer earlier on tenants that flipped the switch.
 */
export default async function middleware(request: NextRequest) {
  const host = request.headers.get('host') || '';

  // Edge fetch the per-tenant feature flags. Failures fall through to the
  // normal locale-routed response — never block traffic on a transient
  // api-server hiccup.
  const killSwitch = await fetchKillSwitch(host);
  if (killSwitch.disabled) {
    return new NextResponse(renderKillSwitchHtml(killSwitch.locale), {
      status: 503,
      headers: {
        'Content-Type': 'text/html; charset=utf-8',
        'Retry-After': '3600',
        'Cache-Control': 'no-store',
        'x-tenant-host': host,
        'x-tenant-kill-switch': 'building_disabled',
      },
    });
  }

  const response = intlMiddleware(request);

  if (host) {
    response.headers.set('x-tenant-host', host);
  }
  response.headers.set('x-tenant-pathname', request.nextUrl.pathname);

  return response;
}

/**
 * Resolve the api-server URL the edge middleware should call.
 *
 * Mirrors `getApiInternalUrl` in `lib/tenant-config.ts` — edge code cannot
 * import that file (it pulls in `next/headers`), so the resolver is
 * duplicated here. Keep them in sync.
 */
function getApiInternalUrl(): string {
  const raw =
    process.env.API_INTERNAL_URL || process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080';
  return raw.replace(/\/+$/, '');
}

type KillSwitchInfo = { disabled: boolean; locale: string | null };

async function fetchKillSwitch(host: string): Promise<KillSwitchInfo> {
  if (!host) return { disabled: false, locale: null };
  try {
    const response = await fetch(`${getApiInternalUrl()}/tenant-config`, {
      method: 'GET',
      headers: { Host: host },
      // 60s ISR — same cadence as the SSR-side getTenantConfig. The SSR
      // layer's React cache() handles per-request dedup, but the data
      // cache here is keyed by URL + host so a flip propagates within a
      // minute.
      next: { revalidate: 60, tags: [`tenant:${host}`] },
    });
    if (!response.ok) return { disabled: false, locale: null };
    const json = (await response.json()) as {
      feature_flags?: Record<string, { enabled?: boolean } | undefined>;
      locales?: string[];
    };
    const flag = json.feature_flags?.building_disabled;
    const disabled = Boolean(flag?.enabled);
    const locale = json.locales?.[0] ?? null;
    return { disabled, locale };
  } catch {
    return { disabled: false, locale: null };
  }
}

export const config = {
  // Match all pathnames except for:
  // - api routes
  // - _next (Next.js internals)
  // - _vercel (Vercel internals)
  // - static files (images, favicon, etc.)
  matcher: ['/((?!api|_next|_vercel|.*\\..*).*)'],
};
