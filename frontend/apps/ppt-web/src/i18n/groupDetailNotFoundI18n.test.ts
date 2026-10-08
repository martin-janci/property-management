/// <reference types="vitest/globals" />
/**
 * Group-detail not-found i18n regression (issue #3040, follow-up to PR #3034).
 *
 * `GroupDetailPageInner` in `routes/groups/community.tsx` renders a not-found
 * branch (`error || !group`) with two i18n keys. The original fix used fresh,
 * never-defined keys `community.groups.notFound` / `community.groups.backToGroups`,
 * so every locale — including non-English users — silently fell back to the
 * hardcoded English `defaultValue` (the "English leaks through because the key
 * is missing" class PR #3036 guards against).
 *
 * A fully-translated `errors.groupNotFound` already exists in all six bundles
 * (and is used by the sibling `GroupDetailPageRoute` guard); `common.back` is
 * likewise shipped everywhere. The fix reuses those instead.
 *
 * This locks the fix two ways:
 *
 *  1. Key presence — the reused keys must exist, non-empty, in all six bundles.
 *  2. Source guard — `community.tsx` must not reference the never-translated
 *     `community.groups.notFound` / `community.groups.backToGroups` keys.
 *     This fails on the pre-fix tree (those keys are present) and passes once
 *     the not-found branch is routed through the existing translated keys.
 */

import { existsSync, readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import cs from '../../messages/cs.json';
import de from '../../messages/de.json';
import en from '../../messages/en.json';
import hu from '../../messages/hu.json';
import pl from '../../messages/pl.json';
import sk from '../../messages/sk.json';

type Json = Record<string, unknown>;

const BUNDLES: Record<string, Json> = { en, sk, cs, de, pl, hu };

function getPath(obj: unknown, path: string): unknown {
  return path.split('.').reduce<unknown>((acc, part) => {
    if (acc === null || typeof acc !== 'object') return undefined;
    return (acc as Json)[part];
  }, obj);
}

/** Existing translated keys the not-found branch must reuse. */
const REQUIRED_KEYS = ['errors.groupNotFound', 'common.back'];

/** i18n keys that were never added to any bundle and must not survive. */
const FORBIDDEN_KEYS = ['community.groups.notFound', 'community.groups.backToGroups'];

/**
 * Resolve the ppt-web `src/routes/groups` dir from the test's cwd (Vitest runs
 * with cwd at the package dir; walk upward so the guard also works from the
 * workspace root). Mirrors `routeGroupToastI18n.test.ts`.
 */
function routeGroupsDir(): string {
  let dir = process.cwd();
  for (let i = 0; i < 6; i++) {
    const candidate = resolve(dir, 'src/routes/groups');
    if (existsSync(candidate)) return candidate;
    const apps = resolve(dir, 'apps/ppt-web/src/routes/groups');
    if (existsSync(apps)) return apps;
    const parent = dirname(dir);
    if (parent === dir) break;
    dir = parent;
  }
  throw new Error(`could not locate src/routes/groups from cwd ${process.cwd()}`);
}

describe('group-detail not-found i18n', () => {
  it('covers exactly the six shipped locale bundles', () => {
    expect(Object.keys(BUNDLES).sort()).toEqual(['cs', 'de', 'en', 'hu', 'pl', 'sk']);
  });

  for (const [locale, bundle] of Object.entries(BUNDLES)) {
    describe(`locale: ${locale}`, () => {
      for (const path of REQUIRED_KEYS) {
        it(`defines a non-empty ${path}`, () => {
          const value = getPath(bundle, path);
          expect(value, `${path} missing in ${locale}.json`).toBeTypeOf('string');
          expect((value as string).trim().length).toBeGreaterThan(0);
        });
      }
    });
  }

  describe('source guard: community.tsx must not use untranslated group keys', () => {
    const source = readFileSync(resolve(routeGroupsDir(), 'community.tsx'), 'utf8');
    for (const key of FORBIDDEN_KEYS) {
      it(`does not reference ${key} (never defined in any bundle)`, () => {
        expect(
          source.includes(key),
          `community.tsx still references the never-translated '${key}' — reuse an existing translated key (errors.groupNotFound / common.back)`
        ).toBe(false);
      });
    }
  });
});
