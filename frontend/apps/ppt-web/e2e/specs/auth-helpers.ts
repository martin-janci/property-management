/**
 * Shared E2E helpers for the ppt-web auth suites.
 *
 * Both `auth-refresh.spec.ts` and `oauth-callback.spec.ts` exercise the same
 * AuthContext token-storage contract, so the JWT builders, storage keys, mock
 * user, and localStorage/route helpers live here to avoid drift between the two
 * specs. Keep the *_KEY constants in sync with `AuthContext.tsx` tokenStorage.
 */

import { expect, type Page } from '@playwright/test';

// ---------------------------------------------------------------------------
// Storage keys — must match AuthContext.tsx tokenStorage keys.
// ---------------------------------------------------------------------------

export const ACCESS_TOKEN_KEY = 'ppt_access_token';
export const REFRESH_TOKEN_KEY = 'ppt_refresh_token';
export const USER_KEY = 'ppt_user';

/** sessionStorage key used by @ppt/shared setSsoState / getAndClearSsoState. */
export const SSO_STATE_KEY = 'sso_state';

export const REFRESH_ENDPOINT = '**/api/v1/auth/refresh';
export const SSO_CALLBACK_ENDPOINT = '**/api/v1/auth/sso/callback';

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

export const MOCK_USER = {
  id: 'usr-e2e-refresh-01',
  email: 'manager@test.example',
  firstName: 'Test',
  lastName: 'Manager',
  role: 'manager',
} as const;

// ---------------------------------------------------------------------------
// JWT / time builders
// ---------------------------------------------------------------------------

/** Build a minimal JWT-shaped string using Node.js Buffer base64url encoding. */
export function makeJwt(payload: Record<string, unknown>): string {
  const header = Buffer.from(JSON.stringify({ alg: 'HS256', typ: 'JWT' })).toString('base64url');
  const body = Buffer.from(JSON.stringify(payload)).toString('base64url');
  return `${header}.${body}.fakesig`;
}

/** Seconds from now (unix epoch). */
export function nowPlus(seconds: number): number {
  return Math.floor(Date.now() / 1000) + seconds;
}

/** A signed-shaped manager access token that expires `seconds` from now. */
export function managerAccessToken(seconds = 900): string {
  return makeJwt({ sub: MOCK_USER.id, role: 'manager', exp: nowPlus(seconds) });
}

// ---------------------------------------------------------------------------
// Storage helpers — all run a single page.evaluate so callers stay terse.
// ---------------------------------------------------------------------------

/** Navigate to /login and wipe local + session storage for a clean slate. */
export async function resetStorage(page: Page): Promise<void> {
  await page.goto('/login');
  await page.evaluate(() => {
    localStorage.clear();
    sessionStorage.clear();
  });
}

/** Read a single localStorage value from the page. */
export function readLocalStorage(page: Page, key: string): Promise<string | null> {
  return page.evaluate((k) => localStorage.getItem(k), key);
}

/**
 * Seed localStorage so AuthProvider.initializeAuth() takes the refresh branch:
 * a stored user + refresh token but NO access token. initializeAuth only sets
 * the user via the refresh branch when a stored user is present, so we keep the
 * user but drop the access token to force a silent refresh on mount.
 */
export async function seedRefreshableSession(page: Page, refreshToken: string): Promise<void> {
  await resetStorage(page);
  await page.evaluate(
    ({ rtKey, uKey, atKey, rt, user }) => {
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

/** Seed ONLY a refresh token (no stored user) — used for failure-path tests. */
export async function seedRefreshTokenOnly(page: Page, refreshToken: string): Promise<void> {
  await resetStorage(page);
  await page.evaluate(
    ({ rtKey, rt }) => {
      localStorage.setItem(rtKey, rt);
    },
    { rtKey: REFRESH_TOKEN_KEY, rt: refreshToken }
  );
}

// ---------------------------------------------------------------------------
// Assertions — poll storage so we never race the AuthContext write.
// ---------------------------------------------------------------------------

/**
 * Assert a localStorage key settles to `expected`. Polling (rather than a bare
 * read after networkidle) removes the race against AuthContext persisting the
 * rotated tokens.
 */
export async function expectLocalStorage(
  page: Page,
  key: string,
  expected: string | null
): Promise<void> {
  await expect.poll(() => readLocalStorage(page, key), { timeout: 8000 }).toBe(expected);
}

// ---------------------------------------------------------------------------
// Route helpers
// ---------------------------------------------------------------------------

/**
 * Stub every non-auth API call with an empty 200 so a protected route can
 * finish rendering. The auth endpoints are left to their dedicated mocks.
 */
export async function stubOtherApis(page: Page): Promise<void> {
  await page.route('**/api/v1/**', async (route) => {
    const url = route.request().url();
    if (url.includes('/api/v1/auth/refresh') || url.includes('/api/v1/auth/sso/callback')) {
      await route.continue();
      return;
    }
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({}),
    });
  });
}
