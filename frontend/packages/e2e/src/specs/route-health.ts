/**
 * Route-health spec factory: every public route loads, shows a `main` landmark
 * and a heading, and logs no console errors. Sourced from the sitemap.
 */

import type { ConsoleMessage } from '@playwright/test';
import { expect, test } from '../fixtures';
import { SitemapPage } from '../pages/SitemapPage';
import { contentPublicRoutes, isSitemapApp } from './shared';

export interface RegisterRouteHealthOptions {
  readonly app: string;
  /**
   * Console-error substrings to ignore (noisy third-party warnings). Matching
   * is case-insensitive substring.
   */
  readonly ignoreConsole?: readonly string[];
}

/** Register route-health tests. Call inside a `test.describe`. */
export function registerRouteHealthSpecs({
  app,
  ignoreConsole = [],
}: RegisterRouteHealthOptions): void {
  if (!isSitemapApp(app)) {
    test.skip(`route-health specs unavailable for '${app}' (no sitemap routes)`, () => {});
    return;
  }

  // SSO callback routes are excluded: they render only a transient spinner /
  // error banner and need a backend to resolve, so they have no plain content
  // surface to health-check (see `isCallbackRoute`).
  for (const route of contentPublicRoutes(app)) {
    test(`route loads cleanly: ${route.id}`, async ({ page }) => {
      const errors: string[] = [];
      const onError = (msg: ConsoleMessage) => {
        if (msg.type() !== 'error') {
          return;
        }
        const text = msg.text();
        if (ignoreConsole.some((needle) => text.toLowerCase().includes(needle.toLowerCase()))) {
          return;
        }
        errors.push(text);
      };
      page.on('console', onError);

      const po = new SitemapPage(page, app, route.id);
      await po.goto();

      await expect(po.main()).toBeAttached();
      await expect(po.heading().first()).toBeVisible();

      page.off('console', onError);
      expect(errors, `console errors on ${route.id}:\n${errors.join('\n')}`).toEqual([]);
    });
  }
}
