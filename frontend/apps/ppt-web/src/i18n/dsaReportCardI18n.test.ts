/// <reference types="vitest/globals" />
/**
 * `dsa.card.*` namespace i18n key-presence regression.
 *
 * The DSA transparency-report card
 * (`src/features/compliance/components/DsaTransparencyReportCard.tsx`) shipped
 * with hardcoded English literals for its headings, status badge, stat labels,
 * section titles and action buttons ("DSA Transparency Report", "Summary
 * Statistics", "Publish Report", "Download PDF", "Published on …", the
 * draft/generated/published/archived status labels, …) instead of going through
 * react-i18next like the rest of the compliance feature — the sibling
 * `DsaReportsPage` (#2847) was already fully localized, but the card it renders
 * was not. Non-English managers saw English text inside every report card,
 * invisible to typecheck + lint because the literals were valid JSX.
 *
 * The strings now live under the `dsa.card` namespace and are consumed via
 * `t('dsa.card.*')`. This test locks it in: every leaf key present under
 * `dsa.card` in `en.json` must exist — and be a non-empty string — in all six
 * shipped locale bundles. It fails on `main` (the namespace does not exist in
 * any bundle) and passes once the block is added to all six. Mirrors
 * `verificationBadgeI18n.test.ts` (#2824).
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

/** The full set of dsa.card.* leaf-key paths, from en.json. */
const CARD_PATHS: string[] = leafPaths(
  (en as Json).dsa && ((en as Json).dsa as Json).card,
  'dsa.card'
);

describe('dsa.card.* i18n keys', () => {
  it('covers exactly the six shipped locale bundles', () => {
    expect(Object.keys(BUNDLES).sort()).toEqual(['cs', 'de', 'en', 'hu', 'pl', 'sk']);
  });

  it('reference key set includes the card title, status labels and actions', () => {
    expect(CARD_PATHS).toEqual(
      expect.arrayContaining([
        'dsa.card.title',
        'dsa.card.period',
        'dsa.card.status.draft',
        'dsa.card.status.generated',
        'dsa.card.status.published',
        'dsa.card.status.archived',
        'dsa.card.publish',
        'dsa.card.download',
        'dsa.card.publishedOn',
      ])
    );
  });

  for (const [locale, bundle] of Object.entries(BUNDLES)) {
    describe(`locale: ${locale}`, () => {
      for (const path of CARD_PATHS) {
        it(`defines a non-empty ${path}`, () => {
          const value = getPath(bundle, path);
          expect(value, `${path} missing in ${locale}.json`).toBeTypeOf('string');
          expect((value as string).trim().length).toBeGreaterThan(0);
        });
      }
    });
  }

  it('keeps every locale in lockstep — no locale has missing dsa.card keys', () => {
    for (const [locale, bundle] of Object.entries(BUNDLES)) {
      const present = CARD_PATHS.filter((p) => typeof getPath(bundle, p) === 'string');
      expect(present, `${locale}.json dsa.card-key set drifted from the reference`).toEqual(
        CARD_PATHS
      );
    }
  });

  it('preserves the {{start}}/{{end}} and {{date}} interpolation placeholders', () => {
    for (const [locale, bundle] of Object.entries(BUNDLES)) {
      const period = getPath(bundle, 'dsa.card.period') as string;
      expect(period, `${locale}.json dsa.card.period lost a placeholder`).toContain('{{start}}');
      expect(period, `${locale}.json dsa.card.period lost a placeholder`).toContain('{{end}}');
      const publishedOn = getPath(bundle, 'dsa.card.publishedOn') as string;
      expect(publishedOn, `${locale}.json dsa.card.publishedOn lost a placeholder`).toContain(
        '{{date}}'
      );
    }
  });
});
