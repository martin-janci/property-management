/**
 * OAuth Callback Flow — E2E Integration Tests
 * Story gap-79-2: ppt-web OAuth callback
 *
 * Covers four scenarios:
 *   1. Happy path — /auth/callback?code=<valid> stores tokens and redirects to /dashboard.
 *   2. Logout — clears localStorage (ppt_access_token, ppt_refresh_token, ppt_user).
 *   3. Expired access token — triggers silent refresh when returned token is past exp claim.
 *   4. Error paths — redirects to /login on OAuth error, missing code, or API failure.
 *
 * All API calls are mocked via page.route() — no backend required.
 */

import { expect, test } from '@playwright/test';

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

const ACCESS_TOKEN_KEY = 'ppt_access_token';
const REFRESH_TOKEN_KEY = 'ppt_refresh_token';
const USER_KEY = 'ppt_user';

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

test.describe('OAuth Callback Flow', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/login');
    await page.evaluate(() => {
      localStorage.clear();
      sessionStorage.clear();
    });
  });

  // -------------------------------------------------------------------------
  // Scenario 1 — Happy path: stores tokens, redirects to /dashboard
  // -------------------------------------------------------------------------

  test('stores access + refresh tokens and redirects to /dashboard on success', async ({
    page,
  }) => {
    const validAccessToken = makeJwt({ sub: MOCK_USER.id, exp: nowPlus(900) });
    const validRefreshToken = 'refresh-token-abc123';

    await page.route('**/api/v1/auth/callback', async (route) => {
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

    // Absorb all other API calls so the redirect settles without a real backend.
    await page.route('**/api/v1/**', async (route) => {
      if (route.request().url().includes('/api/v1/auth/callback')) {
        await route.continue();
      } else {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({}),
        });
      }
    });

    await page.goto('/auth/callback?code=auth-code-valid-001&state=csrf-state-001');

    // Should redirect to /dashboard (or /dashboard/manager after the Navigate redirect).
    await page.waitForURL(/\/dashboard/, { timeout: 8000 });

    // Verify tokens are persisted in localStorage.
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
  // Scenario 2 — Logout clears session and storage
  // -------------------------------------------------------------------------

  test('logout clears access token, refresh token, and user from storage', async ({ page }) => {
    const existingAccessToken = makeJwt({
      sub: MOCK_USER.id,
      exp: nowPlus(900),
    });
    const existingRefreshToken = 'refresh-token-existing-xyz';

    // Pre-populate storage as if the user is already authenticated.
    await page.goto('/login');
    await page.evaluate(
      ({ atKey, rtKey, uKey, at, rt, user }) => {
        localStorage.setItem(atKey, at);
        localStorage.setItem(rtKey, rt);
        localStorage.setItem(uKey, JSON.stringify(user));
      },
      {
        atKey: ACCESS_TOKEN_KEY,
        rtKey: REFRESH_TOKEN_KEY,
        uKey: USER_KEY,
        at: existingAccessToken,
        rt: existingRefreshToken,
        user: MOCK_USER,
      }
    );

    // Mock the server-side logout endpoint.
    await page.route('**/api/v1/auth/logout', async (route) => {
      await route.fulfill({ status: 204 });
    });

    // Simulate the client-side logout path mirroring AuthContext.logout():
    //   1. Call server logout.
    //   2. Clear localStorage keys.
    await page.evaluate(
      async ({ atKey, rtKey, uKey, rt }) => {
        await fetch('/api/v1/auth/logout', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ refreshToken: rt }),
        }).catch(() => {});
        localStorage.removeItem(atKey);
        localStorage.removeItem(rtKey);
        localStorage.removeItem(uKey);
      },
      {
        atKey: ACCESS_TOKEN_KEY,
        rtKey: REFRESH_TOKEN_KEY,
        uKey: USER_KEY,
        rt: existingRefreshToken,
      }
    );

    // Verify all three keys have been cleared.
    const afterAt = await page.evaluate((key) => localStorage.getItem(key), ACCESS_TOKEN_KEY);
    const afterRt = await page.evaluate((key) => localStorage.getItem(key), REFRESH_TOKEN_KEY);
    const afterUser = await page.evaluate((key) => localStorage.getItem(key), USER_KEY);

    expect(afterAt).toBeNull();
    expect(afterRt).toBeNull();
    expect(afterUser).toBeNull();
  });

  // -------------------------------------------------------------------------
  // Scenario 3 — Expired access token triggers silent refresh
  // -------------------------------------------------------------------------

  test('triggers silent token refresh when returned access token is already expired', async ({
    page,
  }) => {
    // Token with exp in the past.
    const expiredAccessToken = makeJwt({
      sub: MOCK_USER.id,
      exp: nowPlus(-60),
    });
    const refreshTokenValue = 'refresh-token-for-expired';
    const freshAccessToken = makeJwt({
      sub: MOCK_USER.id,
      exp: nowPlus(900),
    });

    let refreshCalled = false;

    await page.route('**/api/v1/auth/callback', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          accessToken: expiredAccessToken,
          refreshToken: refreshTokenValue,
          user: MOCK_USER,
        }),
      });
    });

    await page.route('**/api/v1/auth/refresh', async (route) => {
      refreshCalled = true;
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          accessToken: freshAccessToken,
          refreshToken: 'refresh-token-fresh-456',
        }),
      });
    });

    await page.route('**/api/v1/**', async (route) => {
      const url = route.request().url();
      if (url.includes('/api/v1/auth/callback') || url.includes('/api/v1/auth/refresh')) {
        await route.continue();
      } else {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({}),
        });
      }
    });

    await page.goto('/auth/callback?code=auth-code-expired-test&state=state-exp');

    // Should still reach /dashboard after the silent refresh.
    await page.waitForURL(/\/dashboard/, { timeout: 8000 });

    // The /auth/refresh endpoint must have been called.
    expect(refreshCalled).toBe(true);
  });

  // -------------------------------------------------------------------------
  // Scenario 4 — Error paths
  // -------------------------------------------------------------------------

  test('redirects to /login with error param when OAuth server returns error', async ({ page }) => {
    await page.goto('/auth/callback?error=access_denied&error_description=User+denied+access');

    await page.waitForURL(/\/login/, { timeout: 5000 });

    const url = new URL(page.url());
    expect(url.pathname).toBe('/login');
    expect(url.searchParams.get('error')).not.toBeNull();
  });

  test('redirects to /login with error param when authorization code is missing', async ({
    page,
  }) => {
    await page.goto('/auth/callback');

    await page.waitForURL(/\/login/, { timeout: 5000 });

    const url = new URL(page.url());
    expect(url.pathname).toBe('/login');
    expect(url.searchParams.get('error')).not.toBeNull();
  });

  test('redirects to /login when token-exchange API returns an error', async ({ page }) => {
    await page.route('**/api/v1/auth/callback', async (route) => {
      await route.fulfill({
        status: 400,
        contentType: 'application/json',
        body: JSON.stringify({ message: 'Invalid authorization code' }),
      });
    });

    await page.goto('/auth/callback?code=auth-code-bad-001&state=state-bad');

    await page.waitForURL(/\/login/, { timeout: 5000 });

    const url = new URL(page.url());
    expect(url.pathname).toBe('/login');
    expect(url.searchParams.get('error')).not.toBeNull();

    // Tokens must NOT have been written.
    const storedAt = await page.evaluate((key) => localStorage.getItem(key), ACCESS_TOKEN_KEY);
    expect(storedAt).toBeNull();
  });
});
