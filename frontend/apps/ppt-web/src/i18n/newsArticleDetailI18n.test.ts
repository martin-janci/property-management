/// <reference types="vitest/globals" />
/**
 * ArticleDetail (`news.*` + shared) i18n key-presence regression.
 *
 * The Article Detail view — `ArticleDetailPage` plus its `ArticleComments` and
 * `ArticleReactions` children (`src/features/news/**`) — already routes every
 * user-facing literal through `t()`, but a code-review pass (task
 * `code-review-ppt-web-ui-articledetail-i18n-gaps`) found two keys it consumes
 * were missing from the partially-translated bundles: `common.selectOrganization`
 * and `news.errors.loadFailed` were absent from de/pl/hu, so those users saw the
 * raw English fallback for the "select an organization" guard and the
 * article-load failure message.
 *
 * `localeNamespaceParity.test.ts` deliberately asserts only *top-level*
 * namespace parity and documents that de/pl/hu are legitimately incomplete at
 * the leaf level. That guard therefore cannot catch a specific key the view
 * depends on going missing. This test is the per-feature leaf-parity companion
 * (same shape as `communityI18n.test.ts`): every key the ArticleDetail view
 * reads must resolve — as a non-empty string — in all six shipped bundles, with
 * the three i18next plural keys requiring at least `_one` and `_other`.
 *
 * It fails on `main` (de/pl/hu lack the two keys above) and passes once they are
 * added to all six bundles.
 */

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

/**
 * Non-plural keys read by the ArticleDetail view (page + ArticleComments +
 * ArticleReactions). Each must resolve to a non-empty string in every bundle.
 */
const REQUIRED_KEYS: readonly string[] = [
  // shared
  'common.selectOrganization',
  'common.retry',
  'common.save',
  'common.cancel',
  'common.edit',
  'common.delete',
  'errors.articleNotFound',
  'aria.reactions',
  'aria.comments',
  'aria.articleReactions',
  // page chrome
  'news.loadingArticle',
  'news.backToNews',
  'news.pinned',
  // page error paths
  'news.errors.loadFailed',
  'news.errors.reactionFailed',
  'news.errors.commentPostFailed',
  'news.errors.commentUpdateFailed',
  'news.errors.commentDeleteFailed',
  // comments
  'news.comments.title',
  'news.comments.placeholder',
  'news.comments.replyPlaceholder',
  'news.comments.charCount',
  'news.comments.post',
  'news.comments.posting',
  'news.comments.reply',
  'news.comments.deleted',
  'news.comments.empty',
  // reactions
  'news.reactions.like',
  'news.reactions.love',
  'news.reactions.surprised',
  'news.reactions.sad',
  'news.reactions.angry',
  'news.reactions.aria',
  'news.reactions.ariaActive',
];

/**
 * i18next plural keys read by the view. Plural *categories* legitimately differ
 * per locale (sk/cs/pl carry `_few`/`_many`; en/de/hu do not), so we require
 * only the universal minimum — `_one` and `_other` — in every bundle.
 */
const PLURAL_KEYS: readonly string[] = [
  'news.viewCount',
  'news.comments.replyCount',
  'news.reactions.total',
];

describe('ArticleDetail i18n keys', () => {
  it('covers exactly the six shipped locale bundles', () => {
    expect(Object.keys(BUNDLES).sort()).toEqual(['cs', 'de', 'en', 'hu', 'pl', 'sk']);
  });

  it('reference key set spans every ArticleDetail sub-area', () => {
    expect(REQUIRED_KEYS).toEqual(
      expect.arrayContaining([
        'common.selectOrganization',
        'errors.articleNotFound',
        'news.errors.loadFailed',
        'news.comments.title',
        'news.reactions.like',
      ])
    );
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

      for (const base of PLURAL_KEYS) {
        for (const suffix of ['_one', '_other'] as const) {
          const path = `${base}${suffix}`;
          it(`defines a non-empty ${path}`, () => {
            const value = getPath(bundle, path);
            expect(value, `${path} missing in ${locale}.json`).toBeTypeOf('string');
            expect((value as string).trim().length).toBeGreaterThan(0);
          });
        }
      }
    });
  }

  it('keeps every locale in lockstep — no locale has missing ArticleDetail keys', () => {
    for (const [locale, bundle] of Object.entries(BUNDLES)) {
      const present = REQUIRED_KEYS.filter((p) => typeof getPath(bundle, p) === 'string');
      expect(present, `${locale}.json ArticleDetail-key set drifted from the reference`).toEqual(
        REQUIRED_KEYS
      );
    }
  });
});
