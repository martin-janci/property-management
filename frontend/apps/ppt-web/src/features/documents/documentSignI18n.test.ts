/// <reference types="vitest/globals" />
/**
 * Document-signing (signer page) i18n key-presence regression (gap-84-2).
 *
 * The signer-facing `/sign` page (`DocumentSignPage`, Epic 84.2) reads every
 * string from the `documentSign.*` namespace. When the page was added, those
 * keys had to land in ALL six locale bundles (en/sk/cs/de/pl/hu) — not just
 * `en.json` — or the whole public signing surface would silently fall back to
 * English for sk/cs/de/pl/hu signers (invisible to typecheck + lint because
 * `t(...)` degrades via `fallbackLng`).
 *
 * This test locks that in: every leaf key present under `documentSign` in
 * `en.json` must exist — and be a non-empty string — in all six bundles.
 *
 * Fails on `main` (no `documentSign` block exists in any locale); passes once
 * the page ships its strings in every bundle.
 */

import cs from '../../../messages/cs.json';
import de from '../../../messages/de.json';
import en from '../../../messages/en.json';
import hu from '../../../messages/hu.json';
import pl from '../../../messages/pl.json';
import sk from '../../../messages/sk.json';

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

/** The full set of documentSign leaf-key paths, taken from en.json as reference. */
const SIGN_PATHS: string[] = leafPaths((en as Json).documentSign, 'documentSign');

describe('document-sign i18n keys', () => {
  it('covers exactly the six shipped locale bundles', () => {
    expect(Object.keys(BUNDLES).sort()).toEqual(['cs', 'de', 'en', 'hu', 'pl', 'sk']);
  });

  it('reference key set is non-trivial', () => {
    // title/subtitle/loading/... + 5 form.* + 7 errors.* = 23 leaves.
    expect(SIGN_PATHS.length).toBeGreaterThanOrEqual(23);
  });

  for (const [locale, bundle] of Object.entries(BUNDLES)) {
    describe(`locale: ${locale}`, () => {
      for (const path of SIGN_PATHS) {
        it(`defines a non-empty ${path}`, () => {
          const value = getPath(bundle, path);
          expect(value, `${path} missing in ${locale}.json`).toBeTypeOf('string');
          expect((value as string).trim().length).toBeGreaterThan(0);
        });
      }
    });
  }

  it('keeps every locale in lockstep — no locale has missing documentSign keys', () => {
    for (const [locale, bundle] of Object.entries(BUNDLES)) {
      const present = SIGN_PATHS.filter((p) => typeof getPath(bundle, p) === 'string');
      expect(present, `${locale}.json documentSign-key set drifted from the reference`).toEqual(
        SIGN_PATHS
      );
    }
  });
});
