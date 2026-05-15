import createMiddleware from 'next-intl/middleware';
import type { NextRequest } from 'next/server';
import { routing } from './i18n/routing';

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
 * We intentionally do NOT call `/tenant-config` from middleware: edge
 * middleware runs at the very front of every request, and the SSR layer
 * memoizes the call via React `cache()` anyway. Doing the fetch here would
 * double the load for no correctness gain.
 */
export default function middleware(request: NextRequest) {
  const response = intlMiddleware(request);

  const host = request.headers.get('host') || '';
  if (host) {
    response.headers.set('x-tenant-host', host);
  }
  response.headers.set('x-tenant-pathname', request.nextUrl.pathname);

  return response;
}

export const config = {
  // Match all pathnames except for:
  // - api routes
  // - _next (Next.js internals)
  // - _vercel (Vercel internals)
  // - static files (images, favicon, etc.)
  matcher: ['/((?!api|_next|_vercel|.*\\..*).*)'],
};
