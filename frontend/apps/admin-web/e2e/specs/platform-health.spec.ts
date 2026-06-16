/**
 * admin-web Platform Health — app-specific authed screen.
 *
 * Platform Health is a representative authed control-plane screen: every route
 * below `/login` requires a token AND the matching capability (`audit_read`
 * here), so we sign in as the `admin` role via the framework `login` fixture.
 * When no backend/seed is available, that fixture calls `test.skip()` — so
 * these are honest real-screen assertions that turn on against a seeded
 * environment, never green-by-stub.
 *
 * All selectors are encapsulated in the page object — no raw locator strings.
 */

import { backendEnabled, expect, test } from '@ppt/e2e';
import { PlatformHealthPage } from '../pages';

test.describe('admin-web platform health @requires-backend', () => {
  // Platform Health is an authed, capability-gated control-plane screen backed
  // by the health API. Without a backend there is no session to establish and
  // no data to render, so the whole block skips cleanly unless a backend is
  // wired (E2E_WITH_BACKEND=1). The `login('admin')` fixture would also skip on
  // its own, but gating up front keeps the report honest (skipped, not failed).
  test.skip(!backendEnabled(), 'authed health screen requires a backend');

  // admin-web's login route is not in the sitemap; the LoginPage used by the
  // `login` fixture falls back to '/login', which is correct for admin-web.
  test.beforeEach(async ({ login }) => {
    await login('admin');
  });

  test('renders the health title and the three core sections', async ({ page }) => {
    const health = new PlatformHealthPage(page);
    await health.goto();

    await expect(health.title()).toBeVisible();
    await expect(health.currentMetricsSection()).toBeVisible();
    await expect(health.alertsSection()).toBeVisible();
    await expect(health.thresholdsSection()).toBeVisible();
  });

  test('exposes a refresh control and the alerts active/all toggle', async ({ page }) => {
    const health = new PlatformHealthPage(page);
    await health.goto();

    await expect(health.refreshButton()).toBeEnabled();
    await expect(health.alertsToggle('active')).toBeVisible();
    await expect(health.alertsToggle('all')).toBeVisible();
  });

  test('toggling to "all alerts" keeps the alerts section mounted', async ({ page }) => {
    const health = new PlatformHealthPage(page);
    await health.goto();

    await health.alertsToggle('all').click();
    // The section heading is stable across the filter switch — no full reload.
    await expect(health.alertsSection()).toBeVisible();
  });

  test('platform health is reachable from the sidebar', async ({ page }) => {
    const health = new PlatformHealthPage(page);
    await health.goto();

    // The shell sidebar links back to the page we are on (capability-gated).
    await expect(health.navLink(/health/i)).toBeVisible();
    await expect(health.signOutButton()).toBeVisible();
  });
});
