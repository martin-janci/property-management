/**
 * Regression for gap-85-2 (iOS build config by environment).
 *
 * `plugins/withIosBuildConfig.ts` selects the per-environment iOS .xcconfig at
 * prebuild and wires it into the generated Xcode project. The pure pieces —
 * environment resolution (`resolveAppEnv`) and template resolution + CI URL
 * override (`buildXcconfigContents`) — are unit-tested here without needing a
 * generated `ios/` project or a macOS toolchain.
 *
 * These mirror the contract that:
 *   - APP_ENV (then NODE_ENV) picks the environment, exactly like app.config.ts.
 *   - Each environment maps to its committed ios/xcconfig/*.xcconfig template.
 *   - A CI-provided API_BASE_URL overrides the template's PPT_API_BASE_URL
 *     (the same precedence app.config.ts uses for the runtime layer), and the
 *     URL `//` is escaped so xcconfig does not treat it as a comment.
 *   - Without an override, production keeps the __SET_BY_CI__ sentinel.
 */

import * as path from 'node:path';
import {
  type AppEnvironment,
  buildXcconfigContents,
  resolveAppEnv,
  TEMPLATE_BY_ENV,
} from './plugins/withIosBuildConfig';

const PROJECT_ROOT = __dirname;

describe('resolveAppEnv', () => {
  it('honours an explicit APP_ENV', () => {
    expect(resolveAppEnv({ APP_ENV: 'staging' } as NodeJS.ProcessEnv)).toBe('staging');
    expect(resolveAppEnv({ APP_ENV: 'production' } as NodeJS.ProcessEnv)).toBe('production');
    expect(resolveAppEnv({ APP_ENV: 'development' } as NodeJS.ProcessEnv)).toBe('development');
  });

  it('falls back to NODE_ENV when APP_ENV is unset/invalid', () => {
    expect(resolveAppEnv({ NODE_ENV: 'production' } as NodeJS.ProcessEnv)).toBe('production');
    expect(resolveAppEnv({ NODE_ENV: 'development' } as NodeJS.ProcessEnv)).toBe('development');
    expect(resolveAppEnv({ APP_ENV: 'nonsense' } as NodeJS.ProcessEnv)).toBe('development');
    expect(resolveAppEnv({} as NodeJS.ProcessEnv)).toBe('development');
  });
});

describe('TEMPLATE_BY_ENV', () => {
  it('maps each environment to its committed xcconfig template', () => {
    expect(TEMPLATE_BY_ENV).toEqual({
      development: 'Development.xcconfig',
      staging: 'Staging.xcconfig',
      production: 'Production.xcconfig',
    });
  });
});

describe('buildXcconfigContents — per-environment templates', () => {
  const cases: Array<[AppEnvironment, string, string]> = [
    ['development', 'development', 'http:/$()/localhost:8080'],
    ['staging', 'staging', 'https:/$()/staging-api.ppt.example.com'],
    ['production', 'production', '__SET_BY_CI__'],
  ];

  for (const [env, appEnvMarker, apiUrl] of cases) {
    it(`reads the ${env} template (PPT_APP_ENV + API URL)`, () => {
      const out = buildXcconfigContents(PROJECT_ROOT, env, {} as NodeJS.ProcessEnv);
      expect(out).toContain(`PPT_APP_ENV = ${appEnvMarker}`);
      expect(out).toContain(`PPT_API_BASE_URL = ${apiUrl}`);
      expect(out).toContain('PRODUCT_BUNDLE_IDENTIFIER = three.two.bit.ppt.management');
    });
  }

  it('keeps the production sentinel when no CI override is present', () => {
    const out = buildXcconfigContents(PROJECT_ROOT, 'production', {} as NodeJS.ProcessEnv);
    expect(out).toContain('PPT_API_BASE_URL = __SET_BY_CI__');
  });
});

describe('buildXcconfigContents — CI API_BASE_URL override', () => {
  it('overrides PPT_API_BASE_URL for production and escapes the URL `//`', () => {
    const out = buildXcconfigContents(PROJECT_ROOT, 'production', {
      API_BASE_URL: 'https://api.ppt.realhost.sk',
    } as NodeJS.ProcessEnv);
    expect(out).toContain('PPT_API_BASE_URL = https:/$()/api.ppt.realhost.sk');
    // The setting line itself must no longer carry the sentinel (the comment
    // block above it may still *mention* __SET_BY_CI__ — that's fine).
    const settingLine = out.split('\n').find((l) => /^\s*PPT_API_BASE_URL\s*=/.test(l));
    expect(settingLine).toBeDefined();
    expect(settingLine).not.toContain('__SET_BY_CI__');
  });

  it('also overrides for non-production environments', () => {
    const out = buildXcconfigContents(PROJECT_ROOT, 'staging', {
      API_BASE_URL: 'https://staging.real.sk',
    } as NodeJS.ProcessEnv);
    expect(out).toContain('PPT_API_BASE_URL = https:/$()/staging.real.sk');
    expect(out).not.toContain('staging-api.ppt.example.com');
  });
});

describe('committed template files exist', () => {
  it('each env template is present under ios/xcconfig/', () => {
    const fs = require('node:fs') as typeof import('node:fs');
    for (const tpl of Object.values(TEMPLATE_BY_ENV)) {
      const p = path.join(PROJECT_ROOT, 'ios', 'xcconfig', tpl);
      expect(fs.existsSync(p)).toBe(true);
    }
  });
});
