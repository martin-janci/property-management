/// <reference types="vitest/globals" />
/**
 * `community.*` namespace i18n key-presence regression.
 *
 * The community feature (feed/posts, groups, events, marketplace, comments —
 * `src/features/community/**`) shipped English-only: only `PostCard.tsx` called
 * `useTranslation`, and even it left most literals ("Edit", "Delete", "Like",
 * "Pinned", …) hardcoded in JSX. Every other card, list, form and page rendered
 * raw English strings, invisible to typecheck + lint because the literals were
 * valid JSX. Non-English users saw English text across the whole feature.
 *
 * The strings now live under the `community.*` namespace and are consumed via
 * `t('community.*')` in the components. This test locks it in: every leaf key
 * present under `community` in `en.json` must exist — and be a non-empty
 * string — in all six shipped locale bundles. It fails on `main` (the new
 * sub-namespaces do not exist in any bundle) and passes once the blocks are
 * added to all six.
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

/** The full set of community.* leaf-key paths, from en.json. */
const COMMUNITY_PATHS: string[] = leafPaths((en as Json).community, 'community');

describe('community.* i18n keys', () => {
  it('covers exactly the six shipped locale bundles', () => {
    expect(Object.keys(BUNDLES).sort()).toEqual(['cs', 'de', 'en', 'hu', 'pl', 'sk']);
  });

  it('reference key set spans every community sub-feature', () => {
    expect(COMMUNITY_PATHS).toEqual(
      expect.arrayContaining([
        'community.post.like',
        'community.comments.empty',
        'community.feed.title',
        'community.groupList.title',
        'community.groupForm.create',
        'community.groupMembers.roles.owner',
        'community.groupDetail.joinGroup',
        'community.events.title',
        'community.eventCard.btnGoing',
        'community.eventForm.create',
        'community.marketplace.title',
        'community.itemCard.contactSeller',
        'community.itemForm.create',
      ])
    );
  });

  for (const [locale, bundle] of Object.entries(BUNDLES)) {
    describe(`locale: ${locale}`, () => {
      for (const path of COMMUNITY_PATHS) {
        it(`defines a non-empty ${path}`, () => {
          const value = getPath(bundle, path);
          expect(value, `${path} missing in ${locale}.json`).toBeTypeOf('string');
          expect((value as string).trim().length).toBeGreaterThan(0);
        });
      }
    });
  }

  it('keeps every locale in lockstep — no locale has missing community keys', () => {
    for (const [locale, bundle] of Object.entries(BUNDLES)) {
      const present = COMMUNITY_PATHS.filter((p) => typeof getPath(bundle, p) === 'string');
      expect(present, `${locale}.json community-key set drifted from the reference`).toEqual(
        COMMUNITY_PATHS
      );
    }
  });
});
