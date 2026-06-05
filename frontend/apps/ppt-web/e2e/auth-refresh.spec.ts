/**
 * Token Refresh Flow — E2E Integration Tests
 * Story 79-2: ppt-web SSO authentication (token-storage + refresh flow).
 *
 * The OAuth callback path (token storage) is already covered by
 * `oauth-callback.spec.ts`. This suite closes the complementary gap: the
 * silent token-refresh flow driven by AuthProvider.initializeAuth() and
 * AuthContext.refreshTokenInternal() (PRs #568 / #609 / #642).
 *
 * Covers four scenarios:
 *   1. Refresh-on-mount rotation — a stored user + refresh token (but a
 *      missing/blank access token) triggers POST /api/v1/auth/refresh on app
 *      mount; the rotated access + refresh token pair is persisted and the
 *      session stays authenticated (a protected route renders, no bounce to
 *      /login).
 *   2. Refresh failure on mount — when /api/v1/auth/refresh returns 401, the
 *      stored tokens are cleared and a protected route bounces to /login.
 *   3. Token rotation persistence — the refresh endpoint's NEW refresh token
 *      replaces the old one in localStorage (rotation, not reuse).
 *   4. No-refresh-token short-circuit — empty storage makes no refresh call
 *      and a protected route lands on /login.
 *
 * Shared JWT builders, storage keys, and route/storage helpers live in
 * `auth-helpers.ts` (also consumed by `oauth-callback.spec.ts`).
 *
 * All API calls are mocked via page.route() — no backend required.
 */

import { expect, type Route, test } from '@playwright/test';
import {
  ACCESS_TOKEN_KEY,
  expectLocalStorage,
  managerAccessToken,
  REFRESH_ENDPOINT,
  REFRESH_TOKEN_KEY,
  resetStorage,
  seedRefreshableSession,
  seedRefreshTokenOnly,
  stubOtherApis,
} from './auth-helpers';

const PROTECTED_ROUTE = '/documents';

/** Read the `refreshToken` field off a refresh request's JSON body. */
function sentRefreshToken(route: Route): string | undefined {
  const body = JSON.parse(route.request().postData() ?? '{}') as { refreshToken?: string };
  return body.refreshToken;
}

test.describe('Token Refresh Flow (AuthProvider)', () => {
  test.beforeEach(async ({ page }) => {
    await resetStorage(page);
    await stubOtherApis(page);
  });

  // -------------------------------------------------------------------------
  // Scenario 1 — Refresh-on-mount rotation: refresh exchanged, session kept,
  //              rotated tokens persisted, protected route renders.
  // -------------------------------------------------------------------------

  test('refreshes the access token on mount and keeps the session authenticated', async ({
    page,
  }) => {
    const oldRefresh = 'refresh-token-old-001';
    const newAccess = managerAccessToken();
    const newRefresh = 'refresh-token-new-001';

    let refreshCallCount = 0;
    await page.route(REFRESH_ENDPOINT, async (route) => {
      refreshCallCount += 1;
      // The request must carry the OLD refresh token.
      expect(sentRefreshToken(route)).toBe(oldRefresh);
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ accessToken: newAccess, refreshToken: newRefresh }),
      });
    });

    await seedRefreshableSession(page, oldRefresh);

    // Land directly on a protected route. ProtectedRoute redirects to /login
    // BEFORE rendering children when unauthenticated, so staying on the
    // protected route is a real positive signal (not vacuous): a failed
    // refresh-on-mount would land on /login instead.
    await page.goto(PROTECTED_ROUTE);
    await expect(page).toHaveURL(new RegExp(`${PROTECTED_ROUTE}$`), { timeout: 8000 });
    expect(page.url()).not.toContain('/login');

    // The refresh endpoint was hit exactly once with the rotated pair persisted.
    expect(refreshCallCount).toBe(1);
    await expectLocalStorage(page, ACCESS_TOKEN_KEY, newAccess);
    await expectLocalStorage(page, REFRESH_TOKEN_KEY, newRefresh);
  });

  // -------------------------------------------------------------------------
  // Scenario 2 — Refresh failure on mount: tokens cleared, bounce to /login.
  // -------------------------------------------------------------------------

  test('clears tokens and redirects to /login when refresh fails on mount', async ({ page }) => {
    const staleRefresh = 'refresh-token-stale-001';

    await page.route(REFRESH_ENDPOINT, async (route) => {
      await route.fulfill({
        status: 401,
        contentType: 'application/json',
        body: JSON.stringify({ code: 'INVALID_CREDENTIALS', message: 'refresh token expired' }),
      });
    });

    // Seed ONLY a refresh token (no stored user) so a failed refresh leaves the
    // session unauthenticated and a protected route must redirect to /login.
    await seedRefreshTokenOnly(page, staleRefresh);

    await page.goto(PROTECTED_ROUTE);
    await page.waitForURL('**/login', { timeout: 8000 });

    // Stale tokens must have been cleared by initializeAuth()'s catch branch.
    await expectLocalStorage(page, REFRESH_TOKEN_KEY, null);
    await expectLocalStorage(page, ACCESS_TOKEN_KEY, null);
  });

  // -------------------------------------------------------------------------
  // Scenario 3 — Token rotation: the NEW refresh token replaces the old one.
  // -------------------------------------------------------------------------

  test('rotates the refresh token (new value replaces old) after a successful refresh', async ({
    page,
  }) => {
    const oldRefresh = 'refresh-token-rotate-old';
    const newAccess = managerAccessToken();
    const newRefresh = 'refresh-token-rotate-new';

    await page.route(REFRESH_ENDPOINT, async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ accessToken: newAccess, refreshToken: newRefresh }),
      });
    });

    await seedRefreshableSession(page, oldRefresh);

    await page.goto(PROTECTED_ROUTE);

    // The old refresh token must be replaced by the rotated one (not reused).
    await expectLocalStorage(page, REFRESH_TOKEN_KEY, newRefresh);
  });

  // -------------------------------------------------------------------------
  // Scenario 4 — No refresh token: no refresh call, protected route → /login.
  // -------------------------------------------------------------------------

  test('makes no refresh call and redirects to /login when storage is empty', async ({ page }) => {
    let refreshCallMade = false;
    await page.route(REFRESH_ENDPOINT, async (route) => {
      refreshCallMade = true;
      await route.fulfill({ status: 200, contentType: 'application/json', body: '{}' });
    });

    // beforeEach already cleared storage — go straight to a protected route.
    await page.goto(PROTECTED_ROUTE);
    await page.waitForURL('**/login', { timeout: 8000 });

    expect(refreshCallMade).toBe(false);
  });
});
