/// <reference types="vitest/globals" />
/**
 * CriticalNotifications + Outages-errors i18n key-presence regression.
 *
 * Two code-review PRs added user-facing copy that landed only in the
 * fully-translated primary trio (en/sk/cs), leaving the partially-translated
 * de/pl/hu bundles on the raw English fallback:
 *   - #3440 routed the Critical Notifications banner, modal, stats card and
 *     create form through `t()` under the `criticalNotifications.*` namespace,
 *     which de/pl/hu did not carry at all.
 *   - #3457 added the outages list fetch-error copy
 *     (`outages.errors.fetchFailedTitle` / `fetchFailedDescription` / `retry`),
 *     and `outages.errors` was absent from de/pl/hu entirely.
 *
 * `localeNamespaceParity.test.ts` deliberately asserts only *top-level*
 * namespace parity and documents that de/pl/hu are legitimately incomplete at
 * the leaf level, so it cannot catch a specific key a view depends on going
 * missing. This is the per-feature leaf-parity companion (same shape as
 * `newsArticleDetailI18n.test.ts`): every key these two features read must
 * resolve — as a non-empty string — in all six shipped bundles, with the
 * banner plural keys requiring at least `_one` and `_other`.
 *
 * It failed before the follow-up fix (#3466 — de/pl/hu lacked the whole
 * `criticalNotifications` namespace and `outages.errors` subtree) and passes
 * once those are translated into all six bundles.
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
 * Non-plural keys read by the Critical Notifications feature (#3440) and the
 * Outages list/detail error paths (#3457). Each must resolve to a non-empty
 * string in every bundle.
 */
const REQUIRED_KEYS: readonly string[] = [
  // criticalNotifications — list/detail chrome
  'criticalNotifications.title',
  'criticalNotifications.subtitle',
  'criticalNotifications.createNew',
  'criticalNotifications.createDescription',
  'criticalNotifications.backToList',
  'criticalNotifications.noResults',
  'criticalNotifications.requiresAction',
  'criticalNotifications.acknowledged',
  'criticalNotifications.acknowledgedAt',
  'criticalNotifications.createdAt',
  'criticalNotifications.message',
  'criticalNotifications.acknowledgmentInfo',
  'criticalNotifications.acknowledge',
  'criticalNotifications.acknowledmentStats',
  'criticalNotifications.stats.total',
  'criticalNotifications.stats.acknowledged',
  'criticalNotifications.stats.pending',
  'criticalNotifications.filter.all',
  'criticalNotifications.filter.pending',
  'criticalNotifications.filter.acknowledged',
  // banner (non-plural)
  'criticalNotifications.banner.viewNow',
  // modal
  'criticalNotifications.modal.heading',
  'criticalNotifications.modal.counter',
  'criticalNotifications.modal.postedOn',
  'criticalNotifications.modal.acknowledging',
  'criticalNotifications.modal.acknowledgeNext',
  'criticalNotifications.modal.acknowledge',
  // stats card
  'criticalNotifications.statsCard.heading',
  'criticalNotifications.statsCard.percentAcknowledged',
  'criticalNotifications.statsCard.ofTotal',
  'criticalNotifications.statsCard.totalUsers',
  'criticalNotifications.statsCard.acknowledged',
  'criticalNotifications.statsCard.pending',
  // create form
  'criticalNotifications.form.titleLabel',
  'criticalNotifications.form.titlePlaceholder',
  'criticalNotifications.form.messageLabel',
  'criticalNotifications.form.messagePlaceholder',
  'criticalNotifications.form.messageHelp',
  'criticalNotifications.form.warningStrong',
  'criticalNotifications.form.warningText',
  'criticalNotifications.form.cancel',
  'criticalNotifications.form.submit',
  'criticalNotifications.form.submitting',
  'criticalNotifications.form.validation.titleRequired',
  'criticalNotifications.form.validation.titleTooLong',
  'criticalNotifications.form.validation.messageRequired',
  // outages.errors
  'outages.errors.titleRequired',
  'outages.errors.descriptionRequired',
  'outages.errors.buildingsRequired',
  'outages.errors.startRequired',
  'outages.errors.outageNotFound',
  'outages.errors.outageNotFoundDesc',
  'outages.errors.fetchFailedTitle',
  'outages.errors.fetchFailedDescription',
  'outages.errors.retry',
];

/**
 * i18next plural keys read by the Critical Notifications banner. Plural
 * *categories* legitimately differ per locale (sk/cs/pl carry `_few`/`_many`;
 * en/de/hu do not), so we require only the universal minimum — `_one` and
 * `_other` — in every bundle.
 */
const PLURAL_KEYS: readonly string[] = [
  'criticalNotifications.banner.short',
  'criticalNotifications.banner.long',
];

describe('CriticalNotifications + Outages-errors i18n keys', () => {
  it('covers exactly the six shipped locale bundles', () => {
    expect(Object.keys(BUNDLES).sort()).toEqual(['cs', 'de', 'en', 'hu', 'pl', 'sk']);
  });

  it('reference key set spans both features', () => {
    expect(REQUIRED_KEYS).toEqual(
      expect.arrayContaining([
        'criticalNotifications.title',
        'criticalNotifications.form.submit',
        'criticalNotifications.statsCard.heading',
        'outages.errors.fetchFailedTitle',
        'outages.errors.retry',
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

  it('keeps every locale in lockstep — no locale has missing keys', () => {
    for (const [locale, bundle] of Object.entries(BUNDLES)) {
      const present = REQUIRED_KEYS.filter((p) => typeof getPath(bundle, p) === 'string');
      expect(present, `${locale}.json key set drifted from the reference`).toEqual(REQUIRED_KEYS);
    }
  });
});
