/**
 * Shared Playwright config factory.
 *
 * Each app's `playwright.config.ts` is a one-liner:
 *   export default defineE2EConfig({ app: 'ppt-web' });
 *
 * Everything (base URL, reporters, retries, webServer, browser projects) is
 * resolved here from {@link '@ppt/sitemap'}-independent env so behaviour is
 * identical across apps and there are no scattered magic constants.
 */

import {
  defineConfig,
  devices,
  type PlaywrightTestConfig,
  type ReporterDescription,
} from '@playwright/test';
import { type AppName, getBaseURL, getLocalPort, shouldStartWebServer } from './env';

const IS_CI = Boolean(process.env.CI);

export interface DefineE2EConfigOptions {
  /** The app this suite targets. */
  readonly app: AppName;
  /** Local dev-server port (defaults to the app's known dev port). */
  readonly port?: number;
  /** Explicit base URL (wins over env-derived). */
  readonly baseURL?: string;
  /**
   * Test dir relative to the app's e2e config. Each app points this at its own
   * specs folder. Default: `./specs`.
   */
  readonly testDir?: string;
  /** Extra config to shallow-merge over the computed base. Escape hatch. */
  readonly overrides?: PlaywrightTestConfig;
}

/** Build the shared Playwright config for an app. */
export function defineE2EConfig(options: DefineE2EConfigOptions): PlaywrightTestConfig {
  const { app, port = getLocalPort(app), testDir = './specs' } = options;
  const baseURL = options.baseURL ?? getBaseURL(app);

  const config: PlaywrightTestConfig = {
    testDir,
    fullyParallel: true,
    forbidOnly: IS_CI,
    retries: IS_CI ? 2 : 0,
    workers: IS_CI ? 1 : undefined,
    reporter: buildReporters(),
    use: {
      baseURL,
      trace: 'on-first-retry',
      screenshot: 'only-on-failure',
      video: 'retain-on-failure',
    },
    projects: buildProjects(),
    webServer: shouldStartWebServer()
      ? {
          command: 'pnpm dev',
          url: `http://localhost:${port}`,
          reuseExistingServer: !IS_CI,
          timeout: 120_000,
        }
      : undefined,
  };

  return defineConfig(mergeConfig(config, options.overrides));
}

/** html + list + junit; allure-playwright added when `E2E_ALLURE=1`. */
function buildReporters(): ReporterDescription[] {
  const reporters: ReporterDescription[] = [
    ['html', { outputFolder: 'playwright-report', open: 'never' }],
    ['list'],
    ['junit', { outputFile: 'test-results/junit.xml' }],
  ];
  if (process.env.E2E_ALLURE === '1') {
    // Soft dependency: only added when explicitly requested. The app must have
    // `allure-playwright` installed for this to resolve.
    reporters.push(['allure-playwright']);
  }
  return reporters;
}

/**
 * chromium by default; firefox/webkit gated behind `E2E_BROWSERS`
 * (comma-separated, e.g. `chromium,firefox,webkit`).
 */
function buildProjects(): PlaywrightTestConfig['projects'] {
  const requested = (process.env.E2E_BROWSERS ?? 'chromium')
    .split(',')
    .map((b) => b.trim().toLowerCase())
    .filter(Boolean);

  const projects: NonNullable<PlaywrightTestConfig['projects']> = [];
  if (requested.includes('chromium')) {
    projects.push({ name: 'chromium', use: { ...devices['Desktop Chrome'] } });
  }
  if (requested.includes('firefox')) {
    projects.push({ name: 'firefox', use: { ...devices['Desktop Firefox'] } });
  }
  if (requested.includes('webkit')) {
    projects.push({ name: 'webkit', use: { ...devices['Desktop Safari'] } });
  }
  // Always guarantee at least chromium.
  if (projects.length === 0) {
    projects.push({ name: 'chromium', use: { ...devices['Desktop Chrome'] } });
  }
  return projects;
}

function mergeConfig(
  base: PlaywrightTestConfig,
  overrides?: PlaywrightTestConfig
): PlaywrightTestConfig {
  if (!overrides) {
    return base;
  }
  return {
    ...base,
    ...overrides,
    use: { ...base.use, ...overrides.use },
  };
}
