/// <reference types="vitest/globals" />
/**
 * `marketplace.verificationBadge.*` namespace i18n key-presence regression.
 *
 * The marketplace `VerificationBadge` / `VerificationStatusBadge` components
 * (`src/features/marketplace/components/VerificationBadge.tsx`) shipped with
 * hardcoded English literals for their expiry suffixes (" (Expired)",
 * " (Expiring soon)", "(Soon)") and their badge/status/type labels instead of
 * going through react-i18next like the rest of ppt-web (see issue #2824,
 * follow-up to PR #2819). Non-English users saw English text in the badge
 * tooltip and labels — invisible to typecheck + lint because the literals were
 * valid JSX.
 *
 * The strings now live under the `marketplace.verificationBadge` namespace and
 * are consumed via `t('marketplace.verificationBadge.*')`. This test locks it
 * in: every leaf key present under `marketplace.verificationBadge` in `en.json`
 * must exist — and be a non-empty string — in all six shipped locale bundles.
 * It fails on `main` (the namespace does not exist in any bundle) and passes
 * once the block is added to all six.
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

/** The full set of marketplace.verificationBadge.* leaf-key paths, from en.json. */
const BADGE_PATHS: string[] = leafPaths(
  (en as Json).marketplace && ((en as Json).marketplace as Json).verificationBadge,
  'marketplace.verificationBadge'
);

describe('marketplace.verificationBadge.* i18n keys', () => {
  it('covers exactly the six shipped locale bundles', () => {
    expect(Object.keys(BUNDLES).sort()).toEqual(['cs', 'de', 'en', 'hu', 'pl', 'sk']);
  });

  it('reference key set includes the expiry suffixes', () => {
    expect(BADGE_PATHS).toEqual(
      expect.arrayContaining([
        'marketplace.verificationBadge.expired',
        'marketplace.verificationBadge.expiringSoon',
        'marketplace.verificationBadge.soon',
      ])
    );
  });

  for (const [locale, bundle] of Object.entries(BUNDLES)) {
    describe(`locale: ${locale}`, () => {
      for (const path of BADGE_PATHS) {
        it(`defines a non-empty ${path}`, () => {
          const value = getPath(bundle, path);
          expect(value, `${path} missing in ${locale}.json`).toBeTypeOf('string');
          expect((value as string).trim().length).toBeGreaterThan(0);
        });
      }
    });
  }

  it('keeps every locale in lockstep — no locale has missing verificationBadge keys', () => {
    for (const [locale, bundle] of Object.entries(BUNDLES)) {
      const present = BADGE_PATHS.filter((p) => typeof getPath(bundle, p) === 'string');
      expect(
        present,
        `${locale}.json verificationBadge-key set drifted from the reference`
      ).toEqual(BADGE_PATHS);
    }
  });
});
