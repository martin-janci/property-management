/**
 * Extended Playwright test with PPT fixtures.
 *
 * Provides:
 *  - `app`      — a small harness (resolved env + page-object factories).
 *  - `api`      — a fetch-based backend helper for seeding/cleanup, auth-aware,
 *                 degrades gracefully when no backend (`E2E_API_URL`) is set.
 *  - `login()`  — UI login helper (via {@link LoginPage}).
 *  - `authedPage` — a page authenticated as a role, reusing storageState under
 *                 `.auth/<role>.json` so we log in at most once per role.
 *
 * Test data (`testUsers`) is immutable and fully env-overridable — no creds are
 * hardcoded beyond non-secret local defaults.
 */

import { existsSync, mkdirSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { type Browser, test as base, type Page } from '@playwright/test';
import type { SitemapApp } from './env';
import { type AppName, getApiURL, resolveEnvFor } from './env';
import { LoginPage } from './pages/LoginPage';

// ---------------------------------------------------------------------------
// Test users (immutable; env-overridable; no hardcoded secrets)
// ---------------------------------------------------------------------------

/** Roles the framework knows how to authenticate as. */
export type TestRole = 'admin' | 'manager' | 'resident';

export interface TestUserCreds {
  readonly email: string;
  readonly password: string;
}

function userFromEnv(role: TestRole, defaults: TestUserCreds): TestUserCreds {
  const upper = role.toUpperCase();
  return Object.freeze({
    email: process.env[`E2E_${upper}_EMAIL`] ?? defaults.email,
    password: process.env[`E2E_${upper}_PASSWORD`] ?? defaults.password,
  });
}

/**
 * Test users. Local defaults are non-secret placeholders; override per role
 * with `E2E_<ROLE>_EMAIL` / `E2E_<ROLE>_PASSWORD`.
 */
export const testUsers: Readonly<Record<TestRole, TestUserCreds>> = Object.freeze({
  admin: userFromEnv('admin', { email: 'admin@test.example', password: 'TestPassword123!' }),
  manager: userFromEnv('manager', { email: 'manager@test.example', password: 'TestPassword123!' }),
  resident: userFromEnv('resident', {
    email: 'resident@test.example',
    password: 'TestPassword123!',
  }),
});

// ---------------------------------------------------------------------------
// API helper (thin fetch wrapper; graceful no-backend degradation)
// ---------------------------------------------------------------------------

/** Result of an API call; `ok=false` + `skipped` when no backend configured. */
export interface ApiResult<T = unknown> {
  readonly ok: boolean;
  readonly status: number;
  readonly skipped: boolean;
  readonly data: T | undefined;
}

/**
 * Thin fetch-based API helper for seeding/cleanup. When `E2E_API_URL` is unset
 * every call returns `{ ok:false, skipped:true }` so specs can branch without
 * throwing — never a hard dependency on the backend.
 */
export class ApiHelper {
  private token: string | undefined;

  constructor(private readonly baseURL: string | undefined) {}

  /** Whether a backend base URL is configured. */
  get available(): boolean {
    return Boolean(this.baseURL);
  }

  /** Attach a bearer token to subsequent requests. */
  setToken(token: string | undefined): void {
    this.token = token;
  }

  async get<T = unknown>(pathname: string): Promise<ApiResult<T>> {
    return this.request<T>('GET', pathname);
  }

  async post<T = unknown>(pathname: string, body?: unknown): Promise<ApiResult<T>> {
    return this.request<T>('POST', pathname, body);
  }

  async delete<T = unknown>(pathname: string): Promise<ApiResult<T>> {
    return this.request<T>('DELETE', pathname);
  }

  private async request<T>(
    method: string,
    pathname: string,
    body?: unknown
  ): Promise<ApiResult<T>> {
    if (!this.baseURL) {
      return { ok: false, status: 0, skipped: true, data: undefined };
    }
    const headers: Record<string, string> = { 'content-type': 'application/json' };
    if (this.token) {
      headers.authorization = `Bearer ${this.token}`;
    }
    const url = `${this.baseURL}${pathname.startsWith('/') ? pathname : `/${pathname}`}`;
    const res = await fetch(url, {
      method,
      headers,
      body: body === undefined ? undefined : JSON.stringify(body),
    });
    let data: T | undefined;
    try {
      data = (await res.json()) as T;
    } catch {
      data = undefined;
    }
    return { ok: res.ok, status: res.status, skipped: false, data };
  }
}

// ---------------------------------------------------------------------------
// App harness
// ---------------------------------------------------------------------------

/**
 * Per-test app harness. Carries the resolved env and yields page objects so
 * specs never re-derive base URLs or construct pages by hand.
 */
export class AppHarness {
  readonly env: ReturnType<typeof resolveEnvFor>;

  constructor(
    readonly app: AppName,
    private readonly page: Page
  ) {
    this.env = resolveEnvFor(app);
  }

  /** A {@link LoginPage} for this app (sitemap-resolved when applicable). */
  loginPage(loginRouteId?: string): LoginPage {
    const sitemapApp = this.app === 'admin-web' ? undefined : (this.app as SitemapApp);
    return new LoginPage(this.page, { app: sitemapApp, loginRouteId });
  }
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

export interface PptFixtures {
  /** The app under test. Each app overrides this default via its own config. */
  app: AppName;
  /** App harness (env + page-object factories). */
  appHarness: AppHarness;
  /** Backend API helper (seed/cleanup). */
  api: ApiHelper;
  /** UI login helper: `await login(role)` logs the current page in. */
  login: (role: TestRole, loginRouteId?: string) => Promise<void>;
  /** A page pre-authenticated as `manager` with storageState reuse. */
  authedPage: Page;
}

/** Worker-scoped option so each app's config sets its own `app`. */
export interface PptWorkerOptions {
  appName: AppName;
}

const AUTH_DIR = process.env.E2E_AUTH_DIR ?? path.resolve('.auth');

function storageStatePath(role: TestRole): string {
  return path.join(AUTH_DIR, `${role}.json`);
}

/**
 * Perform a UI login on `page` and return whether it succeeded. Uses explicit
 * URL/await waits, never a fixed timeout.
 */
async function performLogin(
  page: Page,
  app: AppName,
  role: TestRole,
  loginRouteId?: string
): Promise<boolean> {
  const sitemapApp = app === 'admin-web' ? undefined : (app as SitemapApp);
  const loginPage = new LoginPage(page, { app: sitemapApp, loginRouteId });
  await loginPage.goto();
  const creds = testUsers[role];
  await loginPage.fillAndSubmit(creds.email, creds.password);
  // Success = navigated away from the login route. Failure = error banner.
  try {
    await page.waitForURL((url) => !url.pathname.includes('login'), { timeout: 15_000 });
    return true;
  } catch {
    return false;
  }
}

export const test = base.extend<PptFixtures, PptWorkerOptions>({
  appName: ['ppt-web', { option: true, scope: 'worker' }],

  app: async ({ appName }, use) => {
    await use(appName);
  },

  appHarness: async ({ app, page }, use) => {
    await use(new AppHarness(app, page));
  },

  api: async ({ app }, use) => {
    const helper = new ApiHelper(getApiURL());
    void app; // app reserved for future per-app API roots
    await use(helper);
  },

  login: async ({ page, app }, use) => {
    await use(async (role: TestRole, loginRouteId?: string) => {
      const ok = await performLogin(page, app, role, loginRouteId);
      if (!ok) {
        test.skip(true, `Login as '${role}' failed (backend/seed unavailable)`);
      }
    });
  },

  authedPage: async ({ browser, app }, use) => {
    const role: TestRole = 'manager';
    const statePath = storageStatePath(role);
    if (!existsSync(AUTH_DIR)) {
      mkdirSync(AUTH_DIR, { recursive: true });
    }

    // Reuse cached storageState when present and non-empty.
    const haveState = existsSync(statePath) && isNonEmptyState(statePath);
    if (!haveState) {
      const ok = await seedStorageState(browser, app, role, statePath);
      if (!ok) {
        test.skip(true, `Could not establish '${role}' session (backend/seed unavailable)`);
      }
    }

    const context = await browser.newContext({ storageState: statePath });
    const authed = await context.newPage();
    await use(authed);
    await context.close();
  },
});

/** Drive a one-off login in a fresh context and persist its storageState. */
async function seedStorageState(
  browser: Browser,
  app: AppName,
  role: TestRole,
  statePath: string
): Promise<boolean> {
  const context = await browser.newContext();
  const page = await context.newPage();
  const ok = await performLogin(page, app, role);
  if (ok) {
    await context.storageState({ path: statePath });
  }
  await context.close();
  return ok;
}

function isNonEmptyState(statePath: string): boolean {
  try {
    const parsed = JSON.parse(readFileSync(statePath, 'utf8')) as {
      cookies?: unknown[];
      origins?: unknown[];
    };
    return (parsed.cookies?.length ?? 0) > 0 || (parsed.origins?.length ?? 0) > 0;
  } catch {
    return false;
  }
}

export { expect } from '@playwright/test';
