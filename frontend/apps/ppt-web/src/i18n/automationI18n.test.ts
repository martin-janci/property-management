/// <reference types="vitest/globals" />
/**
 * `automation.*` namespace i18n key-presence regression.
 *
 * The workflow-automation feature (Epic 43 — automation rules, template
 * library, execution monitoring) originally shipped every visible string
 * hardcoded in English, with only three `aria.*` labels going through i18n
 * (see code-review-ppt-web-ui-automation-feature-no-i18n). All of that copy
 * was moved into a dedicated `automation.*` namespace and translated into the
 * six shipped locales.
 *
 * Missing translation keys degrade silently: `t('automation.foo')` falls back
 * to `en.json` via `fallbackLng`, so a key absent from sk/cs/de/pl/hu is
 * invisible to typecheck + lint but renders English to non-English users — the
 * exact bug this work fixed. This test locks it in: every leaf key present
 * under `automation` in `en.json` must exist — and be a non-empty string — in
 * all six shipped locale bundles.
 *
 * It fails on `main` (no `automation` namespace exists in any bundle) and
 * passes once the namespace is present and in lockstep across all six locales.
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

/** The full set of automation.* leaf-key paths, taken from en.json as reference. */
const AUTOMATION_PATHS: string[] = leafPaths((en as Json).automation, 'automation');

describe('automation.* i18n keys', () => {
  it('covers exactly the six shipped locale bundles', () => {
    expect(Object.keys(BUNDLES).sort()).toEqual(['cs', 'de', 'en', 'hu', 'pl', 'sk']);
  });

  it('reference key set is non-trivial', () => {
    expect(AUTOMATION_PATHS.length).toBeGreaterThanOrEqual(200);
  });

  for (const [locale, bundle] of Object.entries(BUNDLES)) {
    describe(`locale: ${locale}`, () => {
      for (const path of AUTOMATION_PATHS) {
        it(`defines a non-empty ${path}`, () => {
          const value = getPath(bundle, path);
          expect(value, `${path} missing in ${locale}.json`).toBeTypeOf('string');
          expect((value as string).trim().length).toBeGreaterThan(0);
        });
      }
    });
  }

  it('keeps every locale in lockstep — no locale has missing automation keys', () => {
    for (const [locale, bundle] of Object.entries(BUNDLES)) {
      const present = AUTOMATION_PATHS.filter((p) => typeof getPath(bundle, p) === 'string');
      expect(present, `${locale}.json automation-key set drifted from the reference`).toEqual(
        AUTOMATION_PATHS
      );
    }
  });
});
