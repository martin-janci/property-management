/**
 * A {@link BasePage} whose path and auth are resolved from {@link '@ppt/sitemap'}
 * at runtime. This is the bridge that lets generic spec factories drive ANY
 * route without a bespoke page object.
 */

import type { Page } from '@playwright/test';
import { type AuthRequirement, buildUrl, type FrontendRoute, getRoute } from '@ppt/sitemap';
import type { SitemapApp } from '../env';
import { BasePage } from './BasePage';

/** Options for resolving a sitemap-backed page. */
export interface SitemapPageOptions {
  /** Path params to substitute (`:id` / `[slug]`). */
  readonly params?: Readonly<Record<string, string>>;
  /** Query params to append. */
  readonly query?: Readonly<Record<string, string | number | boolean>>;
  /**
   * Locale prefix for Next.js i18n apps (reality-web), e.g. `'sk'`.
   * Prepended as `/<locale>` when set. ppt-web/admin-web ignore this.
   */
  readonly locale?: string;
}

/** Page object backed by a sitemap route entry. */
export class SitemapPage extends BasePage {
  readonly route: FrontendRoute;
  private readonly resolvedPath: string;

  constructor(page: Page, app: SitemapApp, routeId: string, options: SitemapPageOptions = {}) {
    super(page);
    const route = getRoute(app, routeId);
    if (!route) {
      throw new Error(`SitemapPage: route '${routeId}' not found for app '${app}' in @ppt/sitemap`);
    }
    this.route = route;
    this.resolvedPath = SitemapPage.resolvePath(route, options);
  }

  get path(): string {
    return this.resolvedPath;
  }

  /** The route's auth requirement (drives login-needed decisions). */
  get auth(): AuthRequirement {
    return this.route.auth;
  }

  /** Whether this route requires authentication. */
  get requiresAuth(): boolean {
    return this.route.auth.required;
  }

  private static resolvePath(route: FrontendRoute, options: SitemapPageOptions): string {
    const built = buildUrl(route, { ...(options.params ?? {}) }, { ...(options.query ?? {}) });
    if (options.locale) {
      const locale = options.locale.replace(/^\/|\/$/g, '');
      const suffix = built === '/' ? '' : built;
      return `/${locale}${suffix}`;
    }
    return built;
  }
}

/** Factory: build a {@link SitemapPage} for `app`/`routeId`. */
export function sitemapPage(
  page: Page,
  app: SitemapApp,
  routeId: string,
  options?: SitemapPageOptions
): SitemapPage {
  return new SitemapPage(page, app, routeId, options);
}
