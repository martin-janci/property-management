/**
 * Regression for gap-85-2 (app icon variants): per-environment launcher icons
 * with DEV / STG badges.
 *
 * `app.config.ts` resolves the launcher `icon` and the Android adaptive-icon
 * foreground per `APP_ENV`:
 *
 *   development  =>  icon-dev.png      / adaptive-icon-dev.png      (DEV badge)
 *   staging      =>  icon-staging.png  / adaptive-icon-staging.png  (STG badge)
 *   production   =>  icon.png          / adaptive-icon.png          (no badge)
 *
 * When a badged variant PNG is missing on disk, the config falls back to the
 * un-badged production asset (so prebuild / EAS build never fails on a missing
 * file). These tests pin both the happy path (badged asset present) and the
 * fallback path (badged asset absent).
 *
 * `app.config.ts` is a pure function of `{ config }` + `process.env.APP_ENV`,
 * so we evaluate it directly with a fresh module per env. `fs.existsSync` is
 * mocked so the test controls which badged assets "exist" without committing
 * binary icon files.
 */

import type { ConfigContext, ExpoConfig } from 'expo/config';

// Same rationale as app.config.push.test.ts: feed real URLs so the production
// placeholder guard does not throw.
jest.mock('dotenv', () => ({
  config: () => ({
    parsed: { API_BASE_URL: 'https://api.ppt.test', WS_BASE_URL: 'wss://api.ppt.test' },
  }),
}));

// `loadConfig` calls `jest.resetModules()` so `app.config.ts` re-requires
// `node:fs` fresh each time. The mock factory therefore delegates to a
// mutable module-scoped predicate (set per test) rather than a captured
// `jest.fn()` instance — otherwise a reset module would receive a brand-new,
// unconfigured mock.
let mockIconExistsPredicate: (path: string) => boolean = () => true;

jest.mock('node:fs', () => {
  const actual = jest.requireActual('node:fs');
  return {
    ...actual,
    existsSync: (p: unknown) => mockIconExistsPredicate(String(p)),
  };
});

type Env = 'development' | 'staging' | 'production';

function loadConfig(appEnv: Env): ExpoConfig {
  jest.resetModules();
  process.env.APP_ENV = appEnv;
  const mod = require('./app.config').default as (ctx: ConfigContext) => ExpoConfig;
  const ctx: ConfigContext = {
    projectRoot: '.',
    staticConfigPath: null,
    packageJsonPath: null,
    config: {} as ExpoConfig,
  };
  return mod(ctx);
}

const ORIGINAL_APP_ENV = process.env.APP_ENV;

afterEach(() => {
  process.env.APP_ENV = ORIGINAL_APP_ENV;
  mockIconExistsPredicate = () => true;
});

/** Make every referenced asset "present" on disk. */
function allAssetsPresent() {
  mockIconExistsPredicate = () => true;
}

/** Only the un-badged production assets exist (fresh-checkout scenario). */
function onlyBaseAssetsPresent() {
  mockIconExistsPredicate = (s: string) => !(s.includes('-dev') || s.includes('-staging'));
}

describe('per-environment launcher icon (badged variant present)', () => {
  beforeEach(allAssetsPresent);

  it('uses the DEV-badged icon for development', () => {
    const cfg = loadConfig('development');
    expect(cfg.icon).toContain('icon-dev.png');
    expect(cfg.android?.adaptiveIcon?.foregroundImage).toContain('adaptive-icon-dev.png');
  });

  it('uses the STG-badged icon for staging', () => {
    const cfg = loadConfig('staging');
    expect(cfg.icon).toContain('icon-staging.png');
    expect(cfg.android?.adaptiveIcon?.foregroundImage).toContain('adaptive-icon-staging.png');
  });

  it('uses the un-badged icon for production', () => {
    const cfg = loadConfig('production');
    expect(cfg.icon).toContain('icon.png');
    expect(cfg.icon).not.toContain('-dev');
    expect(cfg.icon).not.toContain('-staging');
    expect(cfg.android?.adaptiveIcon?.foregroundImage).toContain('adaptive-icon.png');
    expect(cfg.android?.adaptiveIcon?.foregroundImage).not.toContain('-dev');
    expect(cfg.android?.adaptiveIcon?.foregroundImage).not.toContain('-staging');
  });
});

describe('fallback when badged variant is missing', () => {
  beforeEach(onlyBaseAssetsPresent);

  it('falls back to the un-badged production icon for development', () => {
    const cfg = loadConfig('development');
    expect(cfg.icon).toContain('icon.png');
    expect(cfg.icon).not.toContain('-dev');
    expect(cfg.android?.adaptiveIcon?.foregroundImage).toContain('adaptive-icon.png');
    expect(cfg.android?.adaptiveIcon?.foregroundImage).not.toContain('-dev');
  });

  it('falls back to the un-badged production icon for staging', () => {
    const cfg = loadConfig('staging');
    expect(cfg.icon).toContain('icon.png');
    expect(cfg.icon).not.toContain('-staging');
    expect(cfg.android?.adaptiveIcon?.foregroundImage).toContain('adaptive-icon.png');
    expect(cfg.android?.adaptiveIcon?.foregroundImage).not.toContain('-staging');
  });
});
