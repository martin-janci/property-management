/**
 * Regression for gap-85-2 (iOS build config by environment).
 *
 * `plugins/withIosBuildConfig.ts` selects the per-environment iOS .xcconfig at
 * prebuild and wires it into the generated Xcode project. The pure pieces —
 * environment resolution (`resolveAppEnv`) and template reading
 * (`buildXcconfigContents`) — are unit-tested here without needing a generated
 * `ios/` project or a macOS toolchain.
 *
 * The xcconfig files are native build-layer markers only (PPT_APP_ENV,
 * PRODUCT_BUNDLE_IDENTIFIER). Runtime config (API_BASE_URL, ENVIRONMENT, etc.)
 * is owned by `.env.<env>` -> app.config.ts -> ios.infoPlist and is NOT
 * duplicated in xcconfig to avoid drift (GH #1410).
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
  const cases: Array<[AppEnvironment, string]> = [
    ['development', 'development'],
    ['staging', 'staging'],
    ['production', 'production'],
  ];

  for (const [env, appEnvMarker] of cases) {
    it(`reads the ${env} template (PPT_APP_ENV + bundle id, no runtime config)`, () => {
      const out = buildXcconfigContents(PROJECT_ROOT, env);
      expect(out).toContain(`PPT_APP_ENV = ${appEnvMarker}`);
      expect(out).toContain('PRODUCT_BUNDLE_IDENTIFIER = three.two.bit.ppt.management');
      // Runtime config must not appear — it is owned by app.config.ts (GH #1410)
      expect(out).not.toContain('PPT_API_BASE_URL');
      expect(out).not.toContain('PPT_ALLOWS_ARBITRARY_LOADS');
      expect(out).not.toContain('PPT_BUILD_DISPLAY_NAME');
    });
  }
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
