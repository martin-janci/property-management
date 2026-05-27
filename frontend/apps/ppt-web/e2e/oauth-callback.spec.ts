/**
 * OAuth Callback Flow — E2E Integration Tests
 * Story gap-79-2: ppt-web OAuth callback (AuthCallbackPage)
 *
 * Covers five scenarios:
 *   1. Happy path — /auth/callback?code=<valid>&state=<valid> calls
 *      POST /api/v1/auth/sso/callback, stores tokens and redirects to /dashboard.
 *   2. State-nonce-mismatch — navigating with a tampered state (no prior
 *      setSsoState()) aborts without any API call and renders an error.
 *   3. Already-authenticated short-circuit — if isAuthenticated is true on
 *      mount the page redirects without making an API call.
 *   4. Missing code/state — renders an error banner, stays on /auth/callback.
 *   5. API failure — renders an error banner, stays on /auth/callback.
 *
 * All API calls are mocked via page.route() — no backend required.
 */

import { expect, test } from '@playwright/test';

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/** sessionStorage key used by @ppt/shared setSsoState / getAndClearSsoState */
const SSO_STATE_KEY = 'sso_state';

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

/** Seconds from now (unix epoch) */
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

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

test.describe('OAuth Callback Flow (AuthCallbackPage)', () => {
  test.beforeEach(async ({ page }) => {
    // Navigate somewhere first so we can touch storage, then clear it.
    await page.goto('/login');
    await page.evaluate(() => {
      localStorage.clear();
      sessionStorage.clear();
    });
  });

  // -------------------------------------------------------------------------
  // Scenario 1 — Happy path: valid state nonce + valid code → tokens stored,
  //              redirect to /dashboard
  // -------------------------------------------------------------------------

  test('stores access + refresh tokens and redirects to /dashboard on success', async ({
    page,
  }) => {
    const validAccessToken = makeJwt({ sub: MOCK_USER.id, exp: nowPlus(900) });
    const validRefreshToken = 'refresh-token-abc123';
    const validState = 'csrf-state-e2e-001';

    // Seed sessionStorage with the state nonce so AuthCallbackPage passes validation.
    await page.evaluate(
      ({ key, value }) => {
        sessionStorage.setItem(key, value);
      },
      { key: SSO_STATE_KEY, value: validState }
    );

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

    // Absorb any other API calls so the redirect settles without a real backend.
    await page.route('**/api/v1/**', async (route) => {
      if (route.request().url().includes('/api/v1/auth/sso/callback')) {
        await route.continue();
      } else {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({}),
        });
      }
    });

    await page.goto(`/auth/callback?code=auth-code-valid-001&state=${validState}`);

    // Should redirect to /dashboard (or a sub-route such as /dashboard/manager).
    await page.waitForURL('**/dashboard', { timeout: 8000 });

    // Verify tokens are persisted in localStorage by AuthContext.loginWithSsoCode().
    const storedAccess = await page.evaluate((key) => localStorage.getItem(key), ACCESS_TOKEN_KEY);
    const storedRefresh = await page.evaluate(
      (key) => localStorage.getItem(key),
      REFRESH_TOKEN_KEY
    );
    const storedUserRaw = await page.evaluate((key) => localStorage.getItem(key), USER_KEY);

    expect(storedAccess).toBe(validAccessToken);
    expect(storedRefresh).toBe(validRefreshToken);
    expect(storedUserRaw).not.toBeNull();

    const storedUser = JSON.parse(storedUserRaw as string) as typeof MOCK_USER;
    expect(storedUser.email).toBe(MOCK_USER.email);
    expect(storedUser.id).toBe(MOCK_USER.id);
  });

  // -------------------------------------------------------------------------
  // Scenario 2 — State-nonce-mismatch: tampered state aborts without API call
  // -------------------------------------------------------------------------

  test('shows error and makes no API call when state nonce is missing or mismatched', async ({
    page,
  }) => {
    let apiCallMade = false;

    await page.route('**/api/v1/auth/sso/callback', async (route) => {
      apiCallMade = true;
      await route.fulfill({ status: 400, body: '{}' });
    });

    // Navigate WITHOUT seeding sessionStorage — simulates direct navigation,
    // tab-napping, or a replay attempt (OIDC §3.1.2.7).
    await page.goto('/auth/callback?code=auth-code-x&state=tampered-state');

    // Page stays at /auth/callback (no redirect to /dashboard).
    await page.waitForLoadState('networkidle');
    expect(page.url()).toContain('/auth/callback');

    // Error banner must be visible.
    const errorBanner = page.locator('[role="alert"]');
    await expect(errorBanner).toBeVisible({ timeout: 5000 });

    // No API call should have been made.
    expect(apiCallMade).toBe(false);
  });

  // -------------------------------------------------------------------------
  // Scenario 3 — Already authenticated: short-circuit without API call
  // -------------------------------------------------------------------------

  test('redirects without API call when user is already authenticated', async ({ page }) => {
    const existingToken = makeJwt({ sub: MOCK_USER.id, exp: nowPlus(900) });

    // Pre-populate localStorage to simulate an already-authenticated session.
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
        await route.continue();
      } else {
        await route.fulfill({ status: 200, contentType: 'application/json', body: '{}' });
      }
    });

    await page.goto('/auth/callback?code=auth-code-duplicate&state=any-state');

    // Should redirect to /dashboard without exchanging the code.
    await page.waitForURL('**/dashboard', { timeout: 8000 });
    expect(apiCallMade).toBe(false);
  });

  // -------------------------------------------------------------------------
  // Scenario 4 — Missing code/state params: error banner shown
  // -------------------------------------------------------------------------

  test('renders error banner when authorization code or state is missing', async ({ page }) => {
    // Seed a valid state so the nonce check passes, but omit the code param.
    await page.evaluate(
      ({ key, value }) => {
        sessionStorage.setItem(key, value);
      },
      { key: SSO_STATE_KEY, value: 'state-no-code' }
    );

    await page.goto('/auth/callback?state=state-no-code');

    await page.waitForLoadState('networkidle');

    const errorBanner = page.locator('[role="alert"]');
    await expect(errorBanner).toBeVisible({ timeout: 5000 });
  });

  // -------------------------------------------------------------------------
  // Scenario 5 — API failure: error banner shown, no tokens stored
  // -------------------------------------------------------------------------

  test('renders error banner and stores no tokens when API returns an error', async ({ page }) => {
    const failState = 'csrf-state-fail-001';

    await page.evaluate(
      ({ key, value }) => {
        sessionStorage.setItem(key, value);
      },
      { key: SSO_STATE_KEY, value: failState }
    );

    await page.route('**/api/v1/auth/sso/callback', async (route) => {
      await route.fulfill({
        status: 400,
        contentType: 'application/json',
        body: JSON.stringify({ message: 'Invalid authorization code' }),
      });
    });

    await page.goto(`/auth/callback?code=auth-code-bad-001&state=${failState}`);

    await page.waitForLoadState('networkidle');

    const errorBanner = page.locator('[role="alert"]');
    await expect(errorBanner).toBeVisible({ timeout: 5000 });

    // Tokens must NOT have been written.
    const storedAt = await page.evaluate((key) => localStorage.getItem(key), ACCESS_TOKEN_KEY);
    expect(storedAt).toBeNull();
  });
});
