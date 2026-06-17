/**
 * OAuth callback flow — E2E integration specs (AuthCallbackPage).
 * Story gap-79-2: ppt-web SSO callback.
 *
 * Migrated from the legacy procedural `oauth-callback.spec.ts`. API calls are
 * mocked via `page.route()` (no backend required); navigation + the error
 * banner go through the {@link AuthCallbackPage} page object. Storage seeding
 * and route mocking are test orchestration, not selectors, so they remain in
 * the spec — but no raw `[role="alert"]` / `/auth/callback` URL strings leak
 * into the assertions.
 *
 * Five scenarios:
 *   1. Happy path — valid state + code → tokens stored, redirect to /dashboard.
 *   2. State-nonce mismatch — tampered state aborts without an API call.
 *   3. Already authenticated — short-circuits without an API call.
 *   4. Missing code/state — error banner, stays on /auth/callback.
 *   5. API failure — error banner, no tokens stored.
 */

import { expect, test } from '@ppt/e2e';
import { AuthCallbackPage } from '../pages';

// ---------------------------------------------------------------------------
// Storage keys (contract with @ppt/shared + AuthContext)
// ---------------------------------------------------------------------------

const SSO_STATE_KEY = 'sso_state';
const ACCESS_TOKEN_KEY = 'ppt_access_token';
const REFRESH_TOKEN_KEY = 'ppt_refresh_token';
const USER_KEY = 'ppt_user';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Build a minimal JWT-shaped string using Node Buffer base64url encoding. */
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
  id: 'usr-e2e-01',
  email: 'manager@test.example',
  firstName: 'Test',
  lastName: 'Manager',
  role: 'manager',
};

test.describe('OAuth callback flow (AuthCallbackPage)', () => {
  test.beforeEach(async ({ page }) => {
    // Land on /login so we can touch storage, then clear it.
    await page.goto('/login');
    await page.evaluate(() => {
      localStorage.clear();
      sessionStorage.clear();
    });
  });

  // -------------------------------------------------------------------------
  // Scenario 1 — Happy path
  // -------------------------------------------------------------------------
  test('stores access + refresh tokens and redirects to /dashboard on success', async ({
    page,
  }) => {
    const validAccessToken = makeJwt({ sub: MOCK_USER.id, exp: nowPlus(900) });
    const validRefreshToken = 'refresh-token-abc123';
    const validState = 'csrf-state-e2e-001';

    await page.evaluate(({ key, value }) => sessionStorage.setItem(key, value), {
      key: SSO_STATE_KEY,
      value: validState,
    });

    await page.route('**/api/v1/auth/sso/callback', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          accessToken: validAccessToken,
          refreshToken: validRefreshToken,
          user: MOCK_USER,
        }),
      });
    });
    // Catch-all stub so no `/api/v1/*` call escapes to the (absent) backend via
    // the Vite dev proxy. The later-registered handler runs first, so we
    // `fallback()` (not `continue()`) the sso URL to the specific stub above —
    // `continue()` would send it to the real network and ECONNREFUSED.
    await page.route('**/api/v1/**', async (route) => {
      if (route.request().url().includes('/api/v1/auth/sso/callback')) {
        await route.fallback();
      } else {
        await route.fulfill({ status: 200, contentType: 'application/json', body: '{}' });
      }
    });

    const callback = new AuthCallbackPage(page, { code: 'auth-code-valid-001', state: validState });
    await callback.open();

    await page.waitForURL('**/dashboard', { timeout: 8000 });

    const storedAccess = await page.evaluate((k) => localStorage.getItem(k), ACCESS_TOKEN_KEY);
    const storedRefresh = await page.evaluate((k) => localStorage.getItem(k), REFRESH_TOKEN_KEY);
    const storedUserRaw = await page.evaluate((k) => localStorage.getItem(k), USER_KEY);

    expect(storedAccess).toBe(validAccessToken);
    expect(storedRefresh).toBe(validRefreshToken);
    expect(storedUserRaw).not.toBeNull();

    const storedUser = JSON.parse(storedUserRaw as string) as typeof MOCK_USER;
    expect(storedUser.email).toBe(MOCK_USER.email);
    expect(storedUser.id).toBe(MOCK_USER.id);
  });

  // -------------------------------------------------------------------------
  // Scenario 2 — State-nonce mismatch: no API call, error banner
  // -------------------------------------------------------------------------
  test('shows an error and makes no API call when the state nonce is missing/mismatched', async ({
    page,
  }) => {
    let apiCallMade = false;
    await page.route('**/api/v1/auth/sso/callback', async (route) => {
      apiCallMade = true;
      await route.fulfill({ status: 400, body: '{}' });
    });

    // No sessionStorage seed — simulates direct nav / replay (OIDC §3.1.2.7).
    const callback = new AuthCallbackPage(page, { code: 'auth-code-x', state: 'tampered-state' });
    await callback.open();

    await page.waitForLoadState('networkidle');
    expect(page.url()).toContain('/auth/callback');
    await expect(callback.errorBanner()).toBeVisible({ timeout: 5000 });
    expect(apiCallMade).toBe(false);
  });

  // -------------------------------------------------------------------------
  // Scenario 3 — Already authenticated: short-circuit, no API call
  // -------------------------------------------------------------------------
  test('redirects without an API call when the user is already authenticated', async ({ page }) => {
    const existingToken = makeJwt({ sub: MOCK_USER.id, exp: nowPlus(900) });
    await page.evaluate(
      ({ atKey, rtKey, uKey, at, user }) => {
        localStorage.setItem(atKey, at);
        localStorage.setItem(rtKey, 'refresh-existing');
        localStorage.setItem(uKey, JSON.stringify(user));
      },
      {
        atKey: ACCESS_TOKEN_KEY,
        rtKey: REFRESH_TOKEN_KEY,
        uKey: USER_KEY,
        at: existingToken,
        user: MOCK_USER,
      }
    );

    let apiCallMade = false;
    await page.route('**/api/v1/auth/sso/callback', async (route) => {
      apiCallMade = true;
      await route.fulfill({ status: 400, body: '{}' });
    });
    await page.route('**/api/v1/**', async (route) => {
      if (route.request().url().includes('/api/v1/auth/sso/callback')) {
        await route.fallback();
      } else {
        await route.fulfill({ status: 200, contentType: 'application/json', body: '{}' });
      }
    });

    const callback = new AuthCallbackPage(page, {
      code: 'auth-code-duplicate',
      state: 'any-state',
    });
    await callback.open();

    await page.waitForURL('**/dashboard', { timeout: 8000 });
    expect(apiCallMade).toBe(false);
  });

  // -------------------------------------------------------------------------
  // Scenario 4 — Missing code/state: error banner
  // -------------------------------------------------------------------------
  test('renders an error banner when the authorization code is missing', async ({ page }) => {
    await page.evaluate(({ key, value }) => sessionStorage.setItem(key, value), {
      key: SSO_STATE_KEY,
      value: 'state-no-code',
    });

    const callback = new AuthCallbackPage(page, { state: 'state-no-code' });
    await callback.open();

    await page.waitForLoadState('networkidle');
    await expect(callback.errorBanner()).toBeVisible({ timeout: 5000 });
  });

  // -------------------------------------------------------------------------
  // Scenario 5 — API failure: error banner, no tokens stored
  // -------------------------------------------------------------------------
  test('renders an error banner and stores no tokens when the API returns an error', async ({
    page,
  }) => {
    const failState = 'csrf-state-fail-001';
    await page.evaluate(({ key, value }) => sessionStorage.setItem(key, value), {
      key: SSO_STATE_KEY,
      value: failState,
    });

    await page.route('**/api/v1/auth/sso/callback', async (route) => {
      await route.fulfill({
        status: 400,
        contentType: 'application/json',
        body: JSON.stringify({ message: 'Invalid authorization code' }),
      });
    });

    const callback = new AuthCallbackPage(page, { code: 'auth-code-bad-001', state: failState });
    await callback.open();

    await page.waitForLoadState('networkidle');
    await expect(callback.errorBanner()).toBeVisible({ timeout: 5000 });

    const storedAt = await page.evaluate((k) => localStorage.getItem(k), ACCESS_TOKEN_KEY);
    expect(storedAt).toBeNull();
  });
});
