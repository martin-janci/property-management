/**
 * Regression coverage for the env-driven runtime resolution in `app.config.ts`.
 *
 * `app.config.ts` is a churn hotspot: it maps `APP_ENV` + the parsed
 * `.env.<env>` file onto the resolved Expo config (`extra`, `ios.infoPlist`,
 * `android`, `updates`). The icon (app.config.icon.test.ts), iOS push
 * (app.config.push.test.ts) and iOS build-config (app.config.iosBuildConfig.
 * test.ts) suites already pin their slices; this suite pins the remaining
 * runtime-value surface that the JS app actually reads at runtime:
 *
 *   - API_BASE_URL / WS_BASE_URL resolution + WS derivation from API
 *   - environment + debugMode (+ iOS ATS) resolution
 *   - the production placeholder-URL guard (issue #620 / #523)
 *   - the gap-85-3 App Link host (extra + iOS associatedDomains + Android
 *     intentFilter)
 *   - the `updates` OTA block (EXPO_PROJECT_ID present / absent)
 *   - the google-services.json toggle
 *   - the stable `extra` shape + base identifiers
 *
 * `app.config.ts` is a pure function of `{ config }` + `process.env` +
 * the parsed `.env.<env>` file, so we evaluate it directly with a fresh module
 * per case. `dotenv` and `node:fs` are mocked via mutable module-scoped
 * predicates (the same technique as app.config.icon.test.ts) so each test
 * controls the parsed env / on-disk files without committing fixtures.
 */

import type { ConfigContext, ExpoConfig } from 'expo/config';

/** Parsed `.env.<env>` contents returned to `app.config.ts` per test. */
let mockParsedEnv: Record<string, string> = {};

jest.mock('dotenv', () => ({
  config: () => ({ parsed: mockParsedEnv }),
}));

/**
 * `existsSync` predicate. Default: icon assets "exist" (so icon resolution
 * never warns / falls back here) but `google-services.json` does not (the
 * common local/CI state — it is gitignored). Individual tests override.
 */
let mockFsExists: (p: string) => boolean = (p) => !p.includes('google-services.json');

jest.mock('node:fs', () => {
  const actual = jest.requireActual('node:fs');
  return {
    ...actual,
    existsSync: (p: unknown) => mockFsExists(String(p)),
  };
});

type Env = 'development' | 'staging' | 'production';

/**
 * `appEnv` argument:
 *   - a valid `Env` literal -> assigned to `process.env.APP_ENV`
 *   - any other string (e.g. `'garbage'`) -> assigned verbatim so the
 *     invalid-`APP_ENV` fallback branch of `resolveAppEnv` can be exercised
 *   - `null` -> `APP_ENV` is left cleared (deleted in `beforeEach`), so the
 *     `NODE_ENV`-based fallback branch is exercised
 */
function loadConfig(
  appEnv: Env | 'garbage' | null,
  extraProcessEnv: Record<string, string> = {}
): ExpoConfig {
  jest.resetModules();
  if (appEnv === null) {
    delete process.env.APP_ENV;
  } else {
    // Cast: `'garbage'` is intentionally invalid so the resolver's
    // invalid-APP_ENV fallback branch can be exercised. `process.env.APP_ENV`
    // is typed to the valid `Env` union via a ProcessEnv augmentation.
    process.env.APP_ENV = appEnv as Env;
  }
  for (const [k, v] of Object.entries(extraProcessEnv)) {
    process.env[k] = v;
  }
  const mod = require('./app.config').default as (ctx: ConfigContext) => ExpoConfig;
  const ctx: ConfigContext = {
    projectRoot: '.',
    staticConfigPath: null,
    packageJsonPath: null,
    config: {} as ExpoConfig,
  };
  return mod(ctx);
}

// Keys app.config.ts consults from process.env — snapshot + wipe them so a
// host/CI environment cannot leak into (or flake) these assertions.
const MANAGED_ENV_KEYS = [
  'APP_ENV',
  'NODE_ENV',
  'API_BASE_URL',
  'WS_BASE_URL',
  'APP_LINK_HOST',
  'EXPO_PROJECT_ID',
] as const;

let savedEnv: Record<string, string | undefined>;

beforeEach(() => {
  savedEnv = {};
  for (const k of MANAGED_ENV_KEYS) {
    savedEnv[k] = process.env[k];
    delete process.env[k];
  }
  mockParsedEnv = {};
  mockFsExists = (p) => !p.includes('google-services.json');
});

afterEach(() => {
  for (const k of MANAGED_ENV_KEYS) {
    if (savedEnv[k] === undefined) {
      delete process.env[k];
    } else {
      process.env[k] = savedEnv[k];
    }
  }
});

function extra(cfg: ExpoConfig): Record<string, unknown> {
  return (cfg.extra ?? {}) as Record<string, unknown>;
}

describe('API_BASE_URL / WS_BASE_URL resolution', () => {
  it('propagates the parsed .env URLs into extra and ios.infoPlist', () => {
    mockParsedEnv = { API_BASE_URL: 'https://api.ppt.test', WS_BASE_URL: 'wss://api.ppt.test' };
    const cfg = loadConfig('staging');
    expect(extra(cfg).API_BASE_URL).toBe('https://api.ppt.test');
    expect(extra(cfg).WS_BASE_URL).toBe('wss://api.ppt.test');
    expect(cfg.ios?.infoPlist?.API_BASE_URL).toBe('https://api.ppt.test');
  });

  it('defaults to localhost when nothing is set (development)', () => {
    const cfg = loadConfig('development');
    expect(extra(cfg).API_BASE_URL).toBe('http://localhost:8080');
    // WS derived from API by swapping the http(s) scheme prefix for ws(s).
    expect(extra(cfg).WS_BASE_URL).toBe('ws://localhost:8080');
  });

  it('derives WS_BASE_URL from API_BASE_URL when WS is unset (https -> wss)', () => {
    mockParsedEnv = { API_BASE_URL: 'https://api.ppt.test' };
    const cfg = loadConfig('staging');
    expect(extra(cfg).WS_BASE_URL).toBe('wss://api.ppt.test');
  });

  // NOTE (test-fidelity, #2204): this pins the *file-wins* precedence
  // `envVars.X ?? process.env.X` (app.config.ts). It does NOT model the
  // documented CI production-override path: the `dotenv` mock here returns
  // `mockParsedEnv` directly and never mutates `process.env`, whereas the real
  // sentinel-override path (#620 / #523) rewrites `.env.production` or sets
  // `process.env` via `eas.json env` / `eas secret:create`. A `process.env`
  // override alone would be shadowed by any file value under this precedence —
  // so treat this case as documenting current precedence, not the CI path.
  it('prefers the parsed .env value over process.env', () => {
    mockParsedEnv = { API_BASE_URL: 'https://from-dotenv.test' };
    const cfg = loadConfig('staging', { API_BASE_URL: 'https://from-process.test' });
    expect(extra(cfg).API_BASE_URL).toBe('https://from-dotenv.test');
  });
});

describe('APP_ENV fallback resolution (shared resolveAppEnv)', () => {
  // These cases drive the fallback branch of the environment resolver through
  // the default export. app.config.ts no longer keeps a private `getAppEnv`
  // copy — it re-uses the exported `resolveAppEnv` from withIosBuildConfig.ts
  // (#2204), so a single tested function backs both the JS runtime surface and
  // the native xcconfig surface. With no `ENVIRONMENT` in the parsed .env,
  // `extra.ENVIRONMENT` reflects exactly the resolved APP_ENV.

  it('falls back to production when APP_ENV is unset and NODE_ENV=production', () => {
    const cfg = loadConfig(null, { NODE_ENV: 'production' });
    expect(extra(cfg).ENVIRONMENT).toBe('production');
    // Production posture propagates: debug off + iOS ATS enforces HTTPS.
    expect(extra(cfg).DEBUG_MODE).toBe(false);
    expect(cfg.ios?.infoPlist?.NSAppTransportSecurity).toEqual({ NSAllowsArbitraryLoads: false });
  });

  it('falls back to development when both APP_ENV and NODE_ENV are unset', () => {
    const cfg = loadConfig(null);
    expect(extra(cfg).ENVIRONMENT).toBe('development');
    expect(extra(cfg).DEBUG_MODE).toBe(true);
  });

  it('treats an unrecognised APP_ENV as development', () => {
    const cfg = loadConfig('garbage');
    expect(extra(cfg).ENVIRONMENT).toBe('development');
    expect(extra(cfg).DEBUG_MODE).toBe(true);
  });
});

describe('environment + debugMode resolution', () => {
  it('development => debug mode on, iOS ATS allows arbitrary loads', () => {
    const cfg = loadConfig('development');
    expect(extra(cfg).ENVIRONMENT).toBe('development');
    expect(extra(cfg).DEBUG_MODE).toBe(true);
    expect(cfg.ios?.infoPlist?.ENVIRONMENT).toBe('development');
    expect(cfg.ios?.infoPlist?.NSAppTransportSecurity).toEqual({ NSAllowsArbitraryLoads: true });
  });

  it('production => debug mode off, iOS ATS enforces HTTPS', () => {
    mockParsedEnv = { API_BASE_URL: 'https://api.ppt.test', WS_BASE_URL: 'wss://api.ppt.test' };
    const cfg = loadConfig('production');
    expect(extra(cfg).ENVIRONMENT).toBe('production');
    expect(extra(cfg).DEBUG_MODE).toBe(false);
    expect(cfg.ios?.infoPlist?.NSAppTransportSecurity).toEqual({ NSAllowsArbitraryLoads: false });
  });

  it('DEBUG_MODE=true in .env forces debug on for a non-dev env', () => {
    mockParsedEnv = {
      API_BASE_URL: 'https://api.ppt.test',
      WS_BASE_URL: 'wss://api.ppt.test',
      DEBUG_MODE: 'true',
    };
    const cfg = loadConfig('staging');
    expect(extra(cfg).DEBUG_MODE).toBe(true);
  });

  it('an explicit ENVIRONMENT in .env overrides the APP_ENV-derived value', () => {
    mockParsedEnv = { API_BASE_URL: 'https://api.ppt.test', ENVIRONMENT: 'staging' };
    const cfg = loadConfig('development');
    expect(extra(cfg).ENVIRONMENT).toBe('staging');
  });
});

describe('production placeholder-URL guard (issue #620 / #523)', () => {
  it('throws when a production build ships the __SET_BY_CI__ sentinel', () => {
    mockParsedEnv = { API_BASE_URL: '__SET_BY_CI__', WS_BASE_URL: '__SET_BY_CI__' };
    expect(() => loadConfig('production')).toThrow(/placeholder API URL/);
  });

  it('throws when a production build ships an example.com domain', () => {
    mockParsedEnv = {
      API_BASE_URL: 'https://api.example.com',
      WS_BASE_URL: 'wss://api.example.com',
    };
    expect(() => loadConfig('production')).toThrow(/placeholder API URL/);
  });

  it('does NOT throw for non-production even with a placeholder URL', () => {
    mockParsedEnv = { API_BASE_URL: '__SET_BY_CI__' };
    expect(() => loadConfig('staging')).not.toThrow();
  });

  it('does NOT throw for production with a real URL', () => {
    mockParsedEnv = { API_BASE_URL: 'https://api.ppt.test', WS_BASE_URL: 'wss://api.ppt.test' };
    expect(() => loadConfig('production')).not.toThrow();
  });
});

describe('gap-85-3 App Link host', () => {
  it('defaults to the convention host and wires iOS + Android deep-link config', () => {
    const cfg = loadConfig('development');
    expect(extra(cfg).appLinkHost).toBe('app.ppt.example.com');
    expect(cfg.ios?.associatedDomains).toContain('applinks:app.ppt.example.com');
    // `data` may be a single object or an array — normalise, then find the
    // https entry and read its host.
    const httpsData = (cfg.android?.intentFilters ?? [])
      .flatMap((f) => (Array.isArray(f.data) ? f.data : f.data ? [f.data] : []))
      .find((d) => (d as { scheme?: string }).scheme === 'https') as { host?: string } | undefined;
    expect(httpsData?.host).toBe('app.ppt.example.com');
  });

  it('honours APP_LINK_HOST from the parsed .env', () => {
    mockParsedEnv = { API_BASE_URL: 'https://api.ppt.test', APP_LINK_HOST: 'app.ppt.sk' };
    const cfg = loadConfig('staging');
    expect(extra(cfg).appLinkHost).toBe('app.ppt.sk');
    expect(cfg.ios?.associatedDomains).toContain('applinks:app.ppt.sk');
  });
});

describe('updates (OTA) block', () => {
  it('omits updates.url and disables OTA in development when EXPO_PROJECT_ID is unset', () => {
    const cfg = loadConfig('development');
    expect(cfg.updates?.url).toBeUndefined();
    expect(cfg.updates?.enabled).toBe(false);
    expect((extra(cfg).eas as { projectId?: string }).projectId).toBe('');
  });

  it('enables OTA outside development', () => {
    mockParsedEnv = { API_BASE_URL: 'https://api.ppt.test' };
    const cfg = loadConfig('staging');
    expect(cfg.updates?.enabled).toBe(true);
  });

  it('sets updates.url + eas.projectId when EXPO_PROJECT_ID is present', () => {
    mockParsedEnv = { API_BASE_URL: 'https://api.ppt.test' };
    const cfg = loadConfig('staging', { EXPO_PROJECT_ID: 'proj-123' });
    expect(cfg.updates?.url).toBe('https://u.expo.dev/proj-123');
    expect((extra(cfg).eas as { projectId?: string }).projectId).toBe('proj-123');
  });
});

describe('google-services.json toggle', () => {
  it('omits googleServicesFile when the file is absent', () => {
    mockFsExists = (p) => !p.includes('google-services.json'); // absent
    const cfg = loadConfig('development');
    expect(cfg.android?.googleServicesFile).toBeUndefined();
  });

  it('sets googleServicesFile when the file is present', () => {
    mockFsExists = () => true; // google-services.json present
    const cfg = loadConfig('development');
    expect(cfg.android?.googleServicesFile).toBe('./google-services.json');
  });
});

describe('stable config shape + base identifiers', () => {
  it('carries the expected extra keys', () => {
    mockParsedEnv = { API_BASE_URL: 'https://api.ppt.test' };
    const cfg = loadConfig('staging');
    expect(Object.keys(extra(cfg)).sort()).toEqual(
      ['API_BASE_URL', 'DEBUG_MODE', 'ENVIRONMENT', 'WS_BASE_URL', 'appLinkHost', 'eas'].sort()
    );
  });

  it('pins the management bundle id / package / scheme', () => {
    const cfg = loadConfig('development');
    expect(cfg.ios?.bundleIdentifier).toBe('three.two.bit.ppt.management');
    expect(cfg.android?.package).toBe('three.two.bit.ppt.management');
    expect(cfg.scheme).toBe('ppt-management');
  });
});
