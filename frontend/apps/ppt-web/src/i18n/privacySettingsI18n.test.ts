/// <reference types="vitest/globals" />
/**
 * Privacy-settings i18n key-presence regression.
 *
 * ppt-web ships two privacy-settings surfaces, both driven by `useTranslation`:
 *   - the GDPR screen (`src/features/privacy/pages/PrivacySettingsPage.tsx`)
 *     under the `gdprPrivacy.*` namespace, and
 *   - the neighbors directory privacy screen + form
 *     (`src/features/neighbors/{pages/PrivacySettingsPage,components/PrivacySettingsForm}.tsx`)
 *     under the `neighbors.privacy.*` namespace, plus the shared `common.saving`
 *     submit-button label.
 *
 * Every string already flows through `t(...)`, but the `neighbors.privacy.*`
 * subtree and `common.saving` existed only in the fully-translated trio
 * (en/sk/cs): de/pl/hu carried a stub `neighbors.privacy` with only
 * `saved`/`saveFailed`, so German/Polish/Hungarian users fell back to raw
 * English across the entire neighbors privacy screen. That gap was invisible to
 * typecheck + lint (the keys are valid strings) and to
 * `localeNamespaceParity.test.ts` (which asserts top-level namespace structure,
 * not leaf completeness).
 *
 * This test locks leaf parity for the privacy-settings namespaces: every leaf
 * key present under `gdprPrivacy` and `neighbors.privacy` in `en.json` — plus
 * `common.saving` — must exist and be a non-empty string in all six shipped
 * locale bundles. It fails on `dev` (de/pl/hu lack the `neighbors.privacy`
 * leaves and `common.saving`) and passes once they are filled in all six.
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

/** The full set of privacy-settings leaf-key paths, from en.json. */
const PRIVACY_PATHS: string[] = [
  ...leafPaths(getPath(en, 'gdprPrivacy'), 'gdprPrivacy'),
  ...leafPaths(getPath(en, 'neighbors.privacy'), 'neighbors.privacy'),
  'common.saving',
];

describe('privacy-settings i18n keys', () => {
  it('covers exactly the six shipped locale bundles', () => {
    expect(Object.keys(BUNDLES).sort()).toEqual(['cs', 'de', 'en', 'hu', 'pl', 'sk']);
  });

  it('reference key set spans both privacy surfaces', () => {
    expect(PRIVACY_PATHS).toEqual(
      expect.arrayContaining([
        'gdprPrivacy.title',
        'gdprPrivacy.preferences.visibility',
        'gdprPrivacy.deletion.request',
        'neighbors.privacy.title',
        'neighbors.privacy.visibilityPublic',
        'neighbors.privacy.listedInDirectory',
        'neighbors.privacy.showEmail',
        'common.saving',
      ])
    );
  });

  for (const [locale, bundle] of Object.entries(BUNDLES)) {
    describe(`locale: ${locale}`, () => {
      for (const path of PRIVACY_PATHS) {
        it(`defines a non-empty ${path}`, () => {
          const value = getPath(bundle, path);
          expect(value, `${path} missing in ${locale}.json`).toBeTypeOf('string');
          expect((value as string).trim().length).toBeGreaterThan(0);
        });
      }
    });
  }

  it('keeps every locale in lockstep — no locale has missing privacy keys', () => {
    for (const [locale, bundle] of Object.entries(BUNDLES)) {
      const present = PRIVACY_PATHS.filter((p) => typeof getPath(bundle, p) === 'string');
      expect(present, `${locale}.json privacy-key set drifted from the reference`).toEqual(
        PRIVACY_PATHS
      );
    }
  });
});
