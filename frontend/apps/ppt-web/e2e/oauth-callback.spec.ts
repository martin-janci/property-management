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
 * Shared JWT builders, storage keys, and route/storage helpers live in
 * `auth-helpers.ts` (also consumed by `auth-refresh.spec.ts`).
 *
 * All API calls are mocked via page.route() — no backend required.
 */

import { expect, type Page, test } from '@playwright/test';
import {
  ACCESS_TOKEN_KEY,
  MOCK_USER,
  makeJwt,
  nowPlus,
  REFRESH_TOKEN_KEY,
  readLocalStorage,
  resetStorage,
  SSO_CALLBACK_ENDPOINT,
  SSO_STATE_KEY,
  stubOtherApis,
  USER_KEY,
} from './auth-helpers';

/** Seed the SSO state nonce so AuthCallbackPage passes nonce validation. */
function seedSsoState(page: Page, state: string): Promise<unknown> {
  return page.evaluate(
    ({ key, value }) => {
      sessionStorage.setItem(key, value);
    },
    { key: SSO_STATE_KEY, value: state }
  );
}

test.describe('OAuth Callback Flow (AuthCallbackPage)', () => {
  test.beforeEach(async ({ page }) => {
    await resetStorage(page);
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

    await seedSsoState(page, validState);

    await page.route(SSO_CALLBACK_ENDPOINT, async (route) => {
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
    await stubOtherApis(page);

    await page.goto(`/auth/callback?code=auth-code-valid-001&state=${validState}`);

    // Should redirect to /dashboard (or a sub-route such as /dashboard/manager).
    await page.waitForURL('**/dashboard', { timeout: 8000 });

    // Verify tokens are persisted in localStorage by AuthContext.loginWithSsoCode().
    expect(await readLocalStorage(page, ACCESS_TOKEN_KEY)).toBe(validAccessToken);
    expect(await readLocalStorage(page, REFRESH_TOKEN_KEY)).toBe(validRefreshToken);

    const storedUserRaw = await readLocalStorage(page, USER_KEY);
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

    await page.route(SSO_CALLBACK_ENDPOINT, async (route) => {
      apiCallMade = true;
      await route.fulfill({ status: 400, body: '{}' });
    });

    // Navigate WITHOUT seeding sessionStorage — simulates direct navigation,
    // tab-napping, or a replay attempt (OIDC §3.1.2.7).
    await page.goto('/auth/callback?code=auth-code-x&state=tampered-state');

    // Error banner must be visible — a web-first assertion that also waits.
    await expect(page.locator('[role="alert"]')).toBeVisible({ timeout: 5000 });

    // Page stays at /auth/callback (no redirect to /dashboard) and no API call.
    expect(page.url()).toContain('/auth/callback');
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
    await page.route(SSO_CALLBACK_ENDPOINT, async (route) => {
      apiCallMade = true;
      await route.fulfill({ status: 400, body: '{}' });
    });
    await stubOtherApis(page);

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
    await seedSsoState(page, 'state-no-code');

    await page.goto('/auth/callback?state=state-no-code');

    await expect(page.locator('[role="alert"]')).toBeVisible({ timeout: 5000 });
  });

  // -------------------------------------------------------------------------
  // Scenario 5 — API failure: error banner shown, no tokens stored
  // -------------------------------------------------------------------------

  test('renders error banner and stores no tokens when API returns an error', async ({ page }) => {
    const failState = 'csrf-state-fail-001';

    await seedSsoState(page, failState);

    await page.route(SSO_CALLBACK_ENDPOINT, async (route) => {
      await route.fulfill({
        status: 400,
        contentType: 'application/json',
        body: JSON.stringify({ message: 'Invalid authorization code' }),
      });
    });

    await page.goto(`/auth/callback?code=auth-code-bad-001&state=${failState}`);

    await expect(page.locator('[role="alert"]')).toBeVisible({ timeout: 5000 });

    // Tokens must NOT have been written.
    expect(await readLocalStorage(page, ACCESS_TOKEN_KEY)).toBeNull();
  });
});
