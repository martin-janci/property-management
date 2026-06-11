/**
 * Regression for feat-mobile-push: iOS native push (APNs) configuration.
 *
 * The JS push pipeline (`usePushNotifications` + `backgroundNotifications`)
 * was already complete, but the iOS native config was missing two keys that
 * iOS *requires* for the pipeline to work end-to-end:
 *
 *   1. `ios.entitlements['aps-environment']` — without it iOS never issues an
 *      APNs device token, so `getDevicePushTokenAsync()` cannot register the
 *      device with the backend notification pipeline. Must be `development`
 *      for debug/staging (sandbox APNs) and `production` for release builds.
 *   2. `ios.infoPlist.UIBackgroundModes` must include `remote-notification` so
 *      iOS wakes the app for background/silent (data-only) FCM/APNs pushes —
 *      the path that drives `syncBadgeFromData` for data-only messages.
 *
 * These tests pin both keys per environment. `app.config.ts` is a pure
 * function of `{ config }` + `process.env.APP_ENV`, so we can evaluate it
 * directly with a fresh module per env (env is read at call time).
 */

import type { ConfigContext, ExpoConfig } from 'expo/config';

// `app.config.ts` resolves API URLs from the parsed `.env.<env>` file FIRST
// (`envVars.X ?? process.env.X`), and `.env.production` ships the
// `__SET_BY_CI__` sentinel that the config refuses to build with. Mock dotenv
// so the parsed env carries real URLs — this is the same effect CI achieves by
// overwriting `.env.production` (eas.json env / `eas secret:create`) before a
// production build. The iOS push keys under test do not otherwise depend on
// .env values.
jest.mock('dotenv', () => ({
  config: () => ({
    parsed: { API_BASE_URL: 'https://api.ppt.test', WS_BASE_URL: 'wss://api.ppt.test' },
  }),
}));

function loadConfig(appEnv: 'development' | 'production'): ExpoConfig {
  jest.resetModules();
  process.env.APP_ENV = appEnv;
  // Re-require so the default export picks up the current APP_ENV.
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
});

describe('iOS aps-environment entitlement', () => {
  it('is `development` for development builds (APNs sandbox)', () => {
    const cfg = loadConfig('development');
    expect(cfg.ios?.entitlements?.['aps-environment']).toBe('development');
  });

  it('is `production` for production builds (APNs production gateway)', () => {
    const cfg = loadConfig('production');
    expect(cfg.ios?.entitlements?.['aps-environment']).toBe('production');
  });
});

describe('iOS UIBackgroundModes', () => {
  it('includes `remote-notification` so background/silent push wakes the app', () => {
    const cfg = loadConfig('development');
    const modes = cfg.ios?.infoPlist?.UIBackgroundModes as string[] | undefined;
    expect(modes).toContain('remote-notification');
  });

  it('still includes `remote-notification` in production', () => {
    const cfg = loadConfig('production');
    const modes = cfg.ios?.infoPlist?.UIBackgroundModes as string[] | undefined;
    expect(modes).toContain('remote-notification');
  });
});
