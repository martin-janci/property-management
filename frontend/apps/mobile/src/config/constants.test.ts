/**
 * Regression guard for issue #1663 (follow-up to #1658 / Epic 85 story 85.1).
 *
 * `src/config/constants.ts` must derive `API_BASE_URL` from the single source
 * of truth — `Constants.expoConfig.extra.API_BASE_URL` (populated by
 * `app.config.ts` from `.env.<APP_ENV>`), via `getApiBaseUrl()` in
 * `src/config/api.ts`. It must NEVER:
 *   - fall back to a hardcoded placeholder host (`api.ppt.example.com`), nor
 *   - read `process.env.EXPO_PUBLIC_API_BASE_URL` (or any `EXPO_PUBLIC_*`) at
 *     module scope.
 *
 * This exact regression has shipped more than once (#523 removed it, #535/#620
 * reintroduced it in app.config.ts, #1658 removed it again from constants.ts).
 * This test locks the contract so it cannot silently come back.
 */

const PLACEHOLDER_HOST = 'api.ppt.example.com';

/**
 * Load `./constants` fresh against a given `expoConfig.extra` payload.
 *
 * `jest.isolateModules` + `jest.doMock` give each scenario its own module
 * registry so the module-scope `API_BASE_URL = getApiBaseUrl()` evaluates
 * against the mock we set for that scenario.
 */
function loadConstantsWithExtra(extra: Record<string, unknown> | undefined): {
  API_BASE_URL: string;
  CONSTANTS: { API_BASE_URL: string };
} {
  let mod!: typeof import('./constants');
  jest.isolateModules(() => {
    jest.doMock('expo-constants', () => ({
      expoConfig: extra === undefined ? {} : { extra },
    }));
    // Keep the platform branch in getApiBaseUrl() deterministic.
    jest.doMock('react-native', () => ({ Platform: { OS: 'ios' } }), { virtual: true });
    mod = require('./constants');
  });
  return mod;
}

describe('constants — API_BASE_URL single source of truth (#1663)', () => {
  const ORIGINAL_ENV = process.env;

  beforeEach(() => {
    jest.resetModules();
    process.env = { ...ORIGINAL_ENV };
  });

  afterEach(() => {
    process.env = ORIGINAL_ENV;
    jest.dontMock('expo-constants');
    jest.dontMock('react-native');
  });

  it('resolves API_BASE_URL from Constants.expoConfig.extra.API_BASE_URL', () => {
    const { API_BASE_URL, CONSTANTS } = loadConstantsWithExtra({
      API_BASE_URL: 'https://configured.example',
    });

    expect(API_BASE_URL).toBe('https://configured.example');
    expect(CONSTANTS.API_BASE_URL).toBe('https://configured.example');
  });

  it('never falls back to the api.ppt.example.com placeholder host', () => {
    // No extra configured at all — must NOT yield the legacy placeholder.
    const { API_BASE_URL } = loadConstantsWithExtra(undefined);

    expect(API_BASE_URL).not.toContain(PLACEHOLDER_HOST);
  });

  it('ignores process.env.EXPO_PUBLIC_API_BASE_URL — extra wins', () => {
    process.env.EXPO_PUBLIC_API_BASE_URL = 'https://leaked.example';

    const { API_BASE_URL } = loadConstantsWithExtra({
      API_BASE_URL: 'https://configured.example',
    });

    expect(API_BASE_URL).toBe('https://configured.example');
    expect(API_BASE_URL).not.toBe('https://leaked.example');
  });

  it('does not read EXPO_PUBLIC_API_BASE_URL even when extra is absent', () => {
    // The forbidden regression: a process.env fallback at module scope. With no
    // extra configured the value must come from the dev/platform default in
    // getApiBaseUrl(), never from EXPO_PUBLIC_*.
    process.env.EXPO_PUBLIC_API_BASE_URL = 'https://leaked.example';

    const { API_BASE_URL } = loadConstantsWithExtra(undefined);

    expect(API_BASE_URL).not.toBe('https://leaked.example');
    expect(API_BASE_URL).not.toContain(PLACEHOLDER_HOST);
  });
});
