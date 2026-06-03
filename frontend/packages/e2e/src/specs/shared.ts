/**
 * Shared helpers for the spec factories. Centralizes how we enumerate routes
 * from the sitemap and how we substitute example params so a route with
 * `:id` / `[slug]` segments is still navigable.
 */

import { buildUrl, type FrontendRoute, getPublicRoutes } from '@ppt/sitemap';
import type { SitemapApp } from '../env';

/** Apps the spec factories can drive (sitemap-backed only). */
export function isSitemapApp(app: string): app is SitemapApp {
  return app === 'ppt-web' || app === 'reality-web';
}

/**
 * Public, parameter-free routes for an app — the safe set to smoke-test
 * without seeding data. Routes with required params are skipped here unless
 * the route ships example values for them.
 */
export function navigablePublicRoutes(app: SitemapApp): FrontendRoute[] {
  return getPublicRoutes(app).filter((r) => hasResolvableParams(r));
}

/**
 * Whether a route is an SSO / OAuth callback landing page rather than a plain
 * content page. These exchange a `code`/`state` for tokens against the backend
 * and only ever render a transient spinner or an error banner — they have no
 * stable `main` + heading and cannot "load cleanly" without a backend, so the
 * route-health factory excludes them.
 */
export function isCallbackRoute(route: FrontendRoute): boolean {
  return (route.tags ?? []).includes('sso') || route.id.endsWith('-auth-callback');
}

/**
 * Public content routes safe for route-health: navigable public routes minus
 * SSO callback landing pages (which have no plain content surface).
 */
export function contentPublicRoutes(app: SitemapApp): FrontendRoute[] {
  return navigablePublicRoutes(app).filter((r) => !isCallbackRoute(r));
}

/** Build a concrete URL for a route, applying example params when present. */
export function routeUrl(route: FrontendRoute): string {
  return buildUrl(route, exampleParams(route));
}

function hasResolvableParams(route: FrontendRoute): boolean {
  const required = (route.params ?? []).filter((p) => p.required);
  return required.every((p) => Boolean(p.example));
}

function exampleParams(route: FrontendRoute): Record<string, string> {
  const params: Record<string, string> = {};
  for (const p of route.params ?? []) {
    if (p.example) {
      params[p.name] = p.example;
    }
  }
  return params;
}
