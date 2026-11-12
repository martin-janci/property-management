/// <reference types="vitest/globals" />
/**
 * OAuth-grants (authorized applications) i18n key-presence regression.
 *
 * The OAuth grants / authorized-apps screen
 * (`src/features/oauth-grants/pages/OAuthGrantsPage.tsx`) drives every
 * user-facing string through `useTranslation` + `Trans` under the
 * `oauthGrants.*` namespace, plus the shared `aria.authorizedApplications`
 * screen-reader label. A code-review finding
 * (`code-review-ppt-web-ui-oauth-grants-hardcoded-i18n`) flagged hardcoded
 * English on this screen; by the time it was triaged the component already
 * routed everything through `t(...)` and all six shipped bundles carried the
 * full `oauthGrants.*` subtree — i.e. the finding was stale.
 *
 * i18n leaf gaps degrade silently: `t('oauthGrants.foo')` falls back to
 * `en.json` via `fallbackLng`, so a key missing from sk/cs/de/pl/hu is
 * invisible to typecheck + lint (the keys are valid strings) and to
 * `localeNamespaceParity.test.ts` (which asserts top-level namespace
 * structure, not leaf completeness) — it just announces English to
 * non-English users. This test locks the current good state so a future
 * edit cannot reintroduce the gap: every leaf key present under `oauthGrants`
 * in `en.json` — plus `aria.authorizedApplications` — must exist and be a
 * non-empty string in all six shipped locale bundles.
 */

import cs from '../../messages/cs.json';
import de from '../../messages/de.json';
import en from '../../messages/en.json';
import hu from '../../messages/hu.json';
import pl from '../../messages/pl.json';
import sk from '../../messages/sk.json';

type Json = Record<string, unknown>;

const BUNDLES: Record<string, Json> = { en, sk, cs, de, pl, hu };

/** Dotted leaf-key paths reachable from a nested object. */
function leafPaths(obj: unknown, prefix = ''): string[] {
  if (obj === null || typeof obj !== 'object') return [prefix];
  return Object.entries(obj as Json).flatMap(([k, v]) =>
    leafPaths(v, prefix ? `${prefix}.${k}` : k)
  );
}

function getPath(obj: unknown, path: string): unknown {
  return path.split('.').reduce<unknown>((acc, part) => {
    if (acc === null || typeof acc !== 'object') return undefined;
    return (acc as Json)[part];
  }, obj);
}

/** The full set of OAuth-grants leaf-key paths, taken from en.json as reference. */
const OAUTH_GRANTS_PATHS: string[] = [
  ...leafPaths(getPath(en, 'oauthGrants'), 'oauthGrants'),
  'aria.authorizedApplications',
];

describe('oauthGrants.* i18n keys', () => {
  it('covers exactly the six shipped locale bundles', () => {
    expect(Object.keys(BUNDLES).sort()).toEqual(['cs', 'de', 'en', 'hu', 'pl', 'sk']);
  });

  it('reference key set spans the OAuth-grants screen surface', () => {
    expect(OAUTH_GRANTS_PATHS).toEqual(
      expect.arrayContaining([
        'oauthGrants.title',
        'oauthGrants.description',
        'oauthGrants.empty',
        'oauthGrants.authorized',
        'oauthGrants.permissions',
        'oauthGrants.scopesLabel',
        'oauthGrants.revoke',
        'oauthGrants.revokeFor',
        'oauthGrants.revoking',
        'oauthGrants.cancel',
        'oauthGrants.confirmTitle',
        'oauthGrants.confirmMessage',
        'oauthGrants.revoked',
        'oauthGrants.revokedMessage',
        'oauthGrants.revokeFailed',
        'oauthGrants.unexpected',
        'oauthGrants.loading',
        'oauthGrants.loadFailed',
        'oauthGrants.retry',
        'aria.authorizedApplications',
      ])
    );
  });

  for (const [locale, bundle] of Object.entries(BUNDLES)) {
    describe(`locale: ${locale}`, () => {
      for (const path of OAUTH_GRANTS_PATHS) {
        it(`defines a non-empty ${path}`, () => {
          const value = getPath(bundle, path);
          expect(value, `${path} missing in ${locale}.json`).toBeTypeOf('string');
          expect((value as string).trim().length).toBeGreaterThan(0);
        });
      }
    });
  }

  it('keeps every locale in lockstep — no locale has missing OAuth-grants keys', () => {
    for (const [locale, bundle] of Object.entries(BUNDLES)) {
      const present = OAUTH_GRANTS_PATHS.filter((p) => typeof getPath(bundle, p) === 'string');
      expect(present, `${locale}.json OAuth-grants key set drifted from the reference`).toEqual(
        OAUTH_GRANTS_PATHS
      );
    }
  });
});
