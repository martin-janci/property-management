import type { ApiEndpoint, FrontendRoute, MobileScreen, UserFlow } from '../types';
import { apiServerEndpoints } from './api-server';
import { allFlows } from './flows';
import { mobileScreens } from './mobile';
import { pptWebRoutes } from './ppt-web';
import { realityServerEndpoints } from './reality-server';
import { realityWebRoutes } from './reality-web';

/**
 * Complete sitemap structure
 */
export const sitemap = {
  routes: {
    'ppt-web': pptWebRoutes,
    'reality-web': realityWebRoutes,
  },
  screens: {
    mobile: mobileScreens,
  },
  endpoints: {
    'api-server': apiServerEndpoints,
    'reality-server': realityServerEndpoints,
  },
  flows: allFlows,
} as const;

/**
 * Get a frontend route by ID
 */
export function getRoute(app: 'ppt-web' | 'reality-web', id: string): FrontendRoute | undefined {
  return sitemap.routes[app].find((r) => r.id === id);
}

/**
 * Get a mobile screen by ID
 */
export function getScreen(id: string): MobileScreen | undefined {
  return sitemap.screens.mobile.find((s) => s.id === id);
}

/**
 * Get an API endpoint by operation ID
 */
export function getEndpoint(
  server: 'api-server' | 'reality-server',
  operationId: string
): ApiEndpoint | undefined {
  return sitemap.endpoints[server].find((e) => e.operationId === operationId);
}

/**
 * Get a user flow by ID
 */
export function getFlow(id: string): UserFlow | undefined {
  return sitemap.flows.find((f) => f.id === id);
}

/**
 * Get all routes by tag
 */
export function getRoutesByTag(tag: string): FrontendRoute[] {
  const pptRoutes = sitemap.routes['ppt-web'].filter((r) => r.tags?.includes(tag));
  const realityRoutes = sitemap.routes['reality-web'].filter((r) => r.tags?.includes(tag));
  return [...pptRoutes, ...realityRoutes];
}

/**
 * Get all endpoints by tag
 */
export function getEndpointsByTag(tag: string): ApiEndpoint[] {
  const apiEndpoints = sitemap.endpoints['api-server'].filter((e) => e.tags?.includes(tag));
  const realityEndpoints = sitemap.endpoints['reality-server'].filter((e) => e.tags?.includes(tag));
  return [...apiEndpoints, ...realityEndpoints];
}

/**
 * Get all protected routes for an app
 */
export function getProtectedRoutes(app: 'ppt-web' | 'reality-web'): FrontendRoute[] {
  return sitemap.routes[app].filter((r) => r.auth.required);
}

/**
 * Get all public routes for an app
 */
export function getPublicRoutes(app: 'ppt-web' | 'reality-web'): FrontendRoute[] {
  return sitemap.routes[app].filter((r) => !r.auth.required);
}

export { apiServerEndpoints } from './api-server';
export { allFlows } from './flows';
export { mobileScreens } from './mobile';
// Re-export individual data arrays
export { pptWebRoutes } from './ppt-web';
export { realityServerEndpoints } from './reality-server';
export { realityWebRoutes } from './reality-web';
