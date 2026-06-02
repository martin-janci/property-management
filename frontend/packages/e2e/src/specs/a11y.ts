/**
 * Accessibility spec factory: runs an axe-core scan per public route and
 * asserts there are no `serious`/`critical` violations.
 */

import AxeBuilder from '@axe-core/playwright';
import { expect, test } from '../fixtures';
import { SitemapPage } from '../pages/SitemapPage';
import { isSitemapApp, navigablePublicRoutes } from './shared';

export interface RegisterA11yOptions {
  readonly app: string;
  /** axe impact levels that fail the test. Default: serious + critical. */
  readonly failOn?: readonly ('minor' | 'moderate' | 'serious' | 'critical')[];
  /** Rule ids to disable (e.g. known-accepted false positives). */
  readonly disableRules?: readonly string[];
}

/** Register a11y tests. Call inside a `test.describe`. */
export function registerA11ySpecs({
  app,
  failOn = ['serious', 'critical'],
  disableRules = [],
}: RegisterA11yOptions): void {
  if (!isSitemapApp(app)) {
    test.skip(`a11y specs unavailable for '${app}' (no sitemap routes)`, () => {});
    return;
  }

  for (const route of navigablePublicRoutes(app)) {
    test(`a11y: ${route.id} has no serious/critical violations`, async ({ page }) => {
      const po = new SitemapPage(page, app, route.id);
      await po.goto();
      await expect(po.main()).toBeAttached();

      let builder = new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa']);
      if (disableRules.length > 0) {
        builder = builder.disableRules([...disableRules]);
      }

      const results = await builder.analyze();
      const blocking = results.violations.filter((v) =>
        failOn.includes(v.impact as (typeof failOn)[number])
      );
      const summary = blocking
        .map((v) => `${v.id} (${v.impact}) — ${v.nodes.length} node(s)`)
        .join('\n');
      expect(blocking, `a11y violations on ${route.id}:\n${summary}`).toEqual([]);
    });
  }
}
