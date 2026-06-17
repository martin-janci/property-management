/**
 * Typed environment resolution for the PPT E2E framework.
 *
 * Single source of truth for environment selection and per-app base URLs.
 * No magic constants scattered across specs/configs — everything funnels
 * through {@link getBaseURL} / {@link resolveEnv}.
 */

/** Web apps that have an E2E suite. */
export type AppName = 'ppt-web' | 'reality-web' | 'admin-web';

/**
 * Apps for which the {@link '@ppt/sitemap'} package ships route data.
 * `admin-web` is intentionally excluded — its routes are not (yet) in the
 * sitemap, so sitemap-driven spec factories cannot run against it.
 */
export type SitemapApp = 'ppt-web' | 'reality-web';

/** Deployment target an E2E run points at. */
export type E2EEnv = 'local' | 'dev' | 'staging';

/** Per-environment, per-app base URL table. */
type BaseURLTable = Readonly<Record<E2EEnv, Readonly<Record<AppName, string>>>>;

/**
 * Local dev ports are derived from each app's dev server config:
 *  - ppt-web   — Vite, `server.port: 3000`
 *  - admin-web — Vite, `server.port: 3100`
 *  - reality-web — Next `next dev` (default 3000); we pin 3001 locally so all
 *    three can run side-by-side. Override with E2E_BASE_URL / E2E_REALITY_WEB_BASE_URL.
 */
const LOCAL_PORTS: Readonly<Record<AppName, number>> = {
  'ppt-web': 3000,
  'reality-web': 3001,
  'admin-web': 3100,
} as const;

/** Immutable base-URL matrix. dev/staging follow the *.rlt.sk convention. */
const BASE_URLS: BaseURLTable = {
  local: {
    'ppt-web': `http://localhost:${LOCAL_PORTS['ppt-web']}`,
    'reality-web': `http://localhost:${LOCAL_PORTS['reality-web']}`,
    'admin-web': `http://localhost:${LOCAL_PORTS['admin-web']}`,
  },
  dev: {
    'ppt-web': 'https://staging.ppt.rlt.sk',
    'reality-web': 'https://staging.rlt.sk',
    'admin-web': 'https://admin.staging.rlt.sk',
  },
  staging: {
    'ppt-web': 'https://staging.ppt.rlt.sk',
    'reality-web': 'https://staging.rlt.sk',
    'admin-web': 'https://admin.staging.rlt.sk',
  },
} as const;

const ENV_VALUES: readonly E2EEnv[] = ['local', 'dev', 'staging'] as const;

function isE2EEnv(value: string | undefined): value is E2EEnv {
  return value !== undefined && (ENV_VALUES as readonly string[]).includes(value);
}

/** Resolve the active environment from `E2E_ENV` (defaults to `local`). */
export function resolveEnv(): E2EEnv {
  const raw = process.env.E2E_ENV;
  return isE2EEnv(raw) ? raw : 'local';
}

/** Per-app env var that hard-overrides the computed base URL. */
const APP_URL_ENV: Readonly<Record<AppName, string>> = {
  'ppt-web': 'E2E_PPT_WEB_BASE_URL',
  'reality-web': 'E2E_REALITY_WEB_BASE_URL',
  'admin-web': 'E2E_ADMIN_WEB_BASE_URL',
} as const;

/**
 * Resolve the base URL for an app, in priority order:
 *  1. `E2E_BASE_URL` (global override — wins for whichever app is under test)
 *  2. `E2E_<APP>_BASE_URL` (per-app override)
 *  3. the {@link BASE_URLS} table for the resolved {@link E2EEnv}
 */
export function getBaseURL(app: AppName, env: E2EEnv = resolveEnv()): string {
  const globalOverride = process.env.E2E_BASE_URL;
  if (globalOverride) {
    return stripTrailingSlash(globalOverride);
  }
  const perApp = process.env[APP_URL_ENV[app]];
  if (perApp) {
    return stripTrailingSlash(perApp);
  }
  return stripTrailingSlash(BASE_URLS[env][app]);
}

/** Default local dev-server port for an app (used by the webServer block). */
export function getLocalPort(app: AppName): number {
  return LOCAL_PORTS[app];
}

/**
 * Resolve the backend API base URL for seeding/cleanup helpers.
 * `E2E_API_URL` wins; otherwise undefined (callers degrade gracefully).
 */
export function getApiURL(): string | undefined {
  const raw = process.env.E2E_API_URL;
  return raw ? stripTrailingSlash(raw) : undefined;
}

/** Whether Playwright should manage the dev server itself. */
export function shouldStartWebServer(): boolean {
  if (process.env.E2E_NO_SERVER === '1') {
    return false;
  }
  // An explicit base URL means a server is already running somewhere.
  return !process.env.E2E_BASE_URL;
}

/** Typed view of the resolved E2E environment for a single app. */
export interface ResolvedEnv {
  readonly env: E2EEnv;
  readonly app: AppName;
  readonly baseURL: string;
  readonly apiURL: string | undefined;
  readonly localPort: number;
}

/** Snapshot the whole resolved environment for one app. */
export function resolveEnvFor(app: AppName): ResolvedEnv {
  const env = resolveEnv();
  return {
    env,
    app,
    baseURL: getBaseURL(app, env),
    apiURL: getApiURL(),
    localPort: getLocalPort(app),
  };
}

function stripTrailingSlash(url: string): string {
  return url.endsWith('/') ? url.slice(0, -1) : url;
}
