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
 * All API calls are mocked via page.route() — no backend required.
 */

import { expect, test } from '@playwright/test';

// ---------------------------------------------------------------------------
// Constants — must match AuthContext.tsx tokenStorage keys.
// ---------------------------------------------------------------------------

const ACCESS_TOKEN_KEY = 'ppt_access_token';
const REFRESH_TOKEN_KEY = 'ppt_refresh_token';
const USER_KEY = 'ppt_user';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Build a minimal JWT-shaped string using Node.js Buffer base64url encoding. */
function makeJwt(payload: Record<string, unknown>): string {
  const header = Buffer.from(JSON.stringify({ alg: 'HS256', typ: 'JWT' })).toString('base64url');
  const body = Buffer.from(JSON.stringify(payload)).toString('base64url');
  return `${header}.${body}.fakesig`;
}

/** Seconds from now (unix epoch). */
function nowPlus(seconds: number): number {
  return Math.floor(Date.now() / 1000) + seconds;
}

const MOCK_USER = {
  id: 'usr-e2e-refresh-01',
  email: 'manager@test.example',
  firstName: 'Test',
  lastName: 'Manager',
  role: 'manager',
};

/**
 * Seed localStorage so AuthProvider.initializeAuth() takes the refresh branch:
 * a stored user + refresh token but NO access token. initializeAuth only sets
 * the user via the refresh branch when a stored user is present, so we keep the
 * user but drop the access token to force a silent refresh on mount.
 */
async function seedRefreshableSession(
  page: import('@playwright/test').Page,
  refreshToken: string
): Promise<void> {
  await page.goto('/login');
  await page.evaluate(
    ({ rtKey, uKey, atKey, rt, user }) => {
      localStorage.clear();
      sessionStorage.clear();
      localStorage.setItem(rtKey, rt);
      localStorage.setItem(uKey, JSON.stringify(user));
      // Intentionally NO access token → initializeAuth() refresh branch.
      localStorage.removeItem(atKey);
    },
    {
      rtKey: REFRESH_TOKEN_KEY,
      uKey: USER_KEY,
      atKey: ACCESS_TOKEN_KEY,
      rt: refreshToken,
      user: MOCK_USER,
    }
  );
}

/** Stub every non-auth API call so a protected route can finish rendering. */
async function stubOtherApis(page: import('@playwright/test').Page): Promise<void> {
  await page.route('**/api/v1/**', async (route) => {
    const url = route.request().url();
    if (url.includes('/api/v1/auth/refresh')) {
      await route.continue();
    } else {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({}),
      });
    }
  });
}

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

test.describe('Token Refresh Flow (AuthProvider)', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/login');
    await page.evaluate(() => {
      localStorage.clear();
      sessionStorage.clear();
    });
  });

  // -------------------------------------------------------------------------
  // Scenario 1 — Refresh-on-mount rotation: refresh exchanged, session kept,
  //              rotated tokens persisted, protected route renders.
  // -------------------------------------------------------------------------

  test('refreshes the access token on mount and keeps the session authenticated', async ({
    page,
  }) => {
    const oldRefresh = 'refresh-token-old-001';
    const newAccess = makeJwt({ sub: MOCK_USER.id, role: 'manager', exp: nowPlus(900) });
    const newRefresh = 'refresh-token-new-001';

    let refreshCallCount = 0;
    await page.route('**/api/v1/auth/refresh', async (route) => {
      refreshCallCount += 1;
      // The request must carry the OLD refresh token.
      const sent = JSON.parse(route.request().postData() ?? '{}') as { refreshToken?: string };
      expect(sent.refreshToken).toBe(oldRefresh);
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ accessToken: newAccess, refreshToken: newRefresh }),
      });
    });
    await stubOtherApis(page);

    await seedRefreshableSession(page, oldRefresh);

    // Land directly on a protected route. If the silent refresh works the user
    // stays authenticated and the route renders rather than bouncing to /login.
    await page.goto('/dashboard/manager');
    await page.waitForLoadState('networkidle');

    // Must NOT have been redirected to /login.
    expect(page.url()).not.toContain('/login');

    // The refresh endpoint was hit exactly once.
    expect(refreshCallCount).toBe(1);

    // Rotated token pair is persisted.
    const storedAccess = await page.evaluate((k) => localStorage.getItem(k), ACCESS_TOKEN_KEY);
    const storedRefresh = await page.evaluate((k) => localStorage.getItem(k), REFRESH_TOKEN_KEY);
    expect(storedAccess).toBe(newAccess);
    expect(storedRefresh).toBe(newRefresh);
  });

  // -------------------------------------------------------------------------
  // Scenario 2 — Refresh failure on mount: tokens cleared, bounce to /login.
  // -------------------------------------------------------------------------

  test('clears tokens and redirects to /login when refresh fails on mount', async ({ page }) => {
    const staleRefresh = 'refresh-token-stale-001';

    await page.route('**/api/v1/auth/refresh', async (route) => {
      await route.fulfill({
        status: 401,
        contentType: 'application/json',
        body: JSON.stringify({ code: 'INVALID_CREDENTIALS', message: 'refresh token expired' }),
      });
    });
    await stubOtherApis(page);

    // Seed ONLY a refresh token (no stored user) so a failed refresh leaves the
    // session unauthenticated and a protected route must redirect to /login.
    await page.goto('/login');
    await page.evaluate(
      ({ rtKey, rt }) => {
        localStorage.clear();
        sessionStorage.clear();
        localStorage.setItem(rtKey, rt);
      },
      { rtKey: REFRESH_TOKEN_KEY, rt: staleRefresh }
    );

    await page.goto('/dashboard/manager');
    await page.waitForURL('**/login', { timeout: 8000 });

    // Stale tokens must have been cleared by initializeAuth()'s catch branch.
    const storedRefresh = await page.evaluate((k) => localStorage.getItem(k), REFRESH_TOKEN_KEY);
    const storedAccess = await page.evaluate((k) => localStorage.getItem(k), ACCESS_TOKEN_KEY);
    expect(storedRefresh).toBeNull();
    expect(storedAccess).toBeNull();
  });

  // -------------------------------------------------------------------------
  // Scenario 3 — Token rotation: the NEW refresh token replaces the old one.
  // -------------------------------------------------------------------------

  test('rotates the refresh token (new value replaces old) after a successful refresh', async ({
    page,
  }) => {
    const oldRefresh = 'refresh-token-rotate-old';
    const newAccess = makeJwt({ sub: MOCK_USER.id, role: 'manager', exp: nowPlus(900) });
    const newRefresh = 'refresh-token-rotate-new';

    await page.route('**/api/v1/auth/refresh', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ accessToken: newAccess, refreshToken: newRefresh }),
      });
    });
    await stubOtherApis(page);

    await seedRefreshableSession(page, oldRefresh);

    await page.goto('/dashboard/manager');
    await page.waitForLoadState('networkidle');

    const storedRefresh = await page.evaluate((k) => localStorage.getItem(k), REFRESH_TOKEN_KEY);
    // The old refresh token must be gone (rotation, not reuse).
    expect(storedRefresh).toBe(newRefresh);
    expect(storedRefresh).not.toBe(oldRefresh);
  });

  // -------------------------------------------------------------------------
  // Scenario 4 — No refresh token: no refresh call, protected route → /login.
  // -------------------------------------------------------------------------

  test('makes no refresh call and redirects to /login when storage is empty', async ({ page }) => {
    let refreshCallMade = false;
    await page.route('**/api/v1/auth/refresh', async (route) => {
      refreshCallMade = true;
      await route.fulfill({ status: 200, body: '{}' });
    });
    await stubOtherApis(page);

    // beforeEach already cleared storage — go straight to a protected route.
    await page.goto('/dashboard/manager');
    await page.waitForURL('**/login', { timeout: 8000 });

    expect(refreshCallMade).toBe(false);
  });
});
