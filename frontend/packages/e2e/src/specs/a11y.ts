/**
 * Accessibility spec factory: runs an axe-core scan per public route.
 *
 * Report-only by default (the green-by-default CI ratchet): every scan still
 * runs on every public route and any `serious`/`critical` violations are
 * always attached to the test (as an annotation + a stderr line) so they are
 * visible in the report — but the test only *fails* when `E2E_A11Y_STRICT` is
 * set. Flip that env var on to ratchet a route (or the whole suite) up to a
 * hard gate once its violations are driven to zero.
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

/** Whether the a11y gate is ratcheted to a hard failure (opt-in via env). */
function isStrict(): boolean {
  const v = (process.env.E2E_A11Y_STRICT ?? '').toLowerCase();
  return v !== '' && v !== '0' && v !== 'false';
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

      // Always surface findings on the report (as a test annotation + a stderr
      // line), whether or not we gate on them — the report-only ratchet.
      if (blocking.length > 0) {
        test.info().annotations.push({
          type: 'a11y-violation',
          description: `${route.id}: ${summary}`,
        });
        process.stderr.write(`[a11y] report-only violations on ${route.id}:\n${summary}\n`);
      }

      // Report-only by default; only fail under the opt-in strict ratchet.
      if (isStrict()) {
        expect(blocking, `a11y violations on ${route.id}:\n${summary}`).toEqual([]);
      }
    });
  }
}
