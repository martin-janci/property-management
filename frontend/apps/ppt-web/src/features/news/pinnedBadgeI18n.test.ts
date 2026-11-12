/// <reference types="vitest/globals" />
/**
 * Pinned-badge i18n leaf-parity regression.
 *
 * A code-review finding claimed the ppt-web "pinned" badge carried a
 * hard-coded (non-i18n) label. It was stale: every pinned badge/indicator
 * already routes through `t(...)` and every key ships a real translation in
 * all six locales. This test pins that down so the badge can never regress to
 * a literal English string or a missing translation (fallback render):
 *
 *   1. leaf parity — each pinned-badge key resolves to a non-empty string in
 *      every one of the six shipped locales (no `defaultValue` fallback, no
 *      blank cell);
 *   2. usage — every badge render site references its `t(...)` key and renders
 *      no hard-coded literal (`>Pinned<`).
 *
 * The badge appears across the news and announcements features, so this guard
 * spans both namespaces (`news.*`, `announcements.*`, `aria.*`).
 *
 * cf. PR #3465 — the leaf-parity regression-test pattern for a stale
 * hard-coded-label finding.
 */

import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import cs from '../../../messages/cs.json';
import de from '../../../messages/de.json';
import en from '../../../messages/en.json';
import hu from '../../../messages/hu.json';
import pl from '../../../messages/pl.json';
import sk from '../../../messages/sk.json';

type Json = { [key: string]: unknown };

const BUNDLES: Record<string, Json> = { en, sk, cs, de, pl, hu };

/** The i18n keys behind every pinned badge / indicator in ppt-web. */
const PINNED_KEYS = ['announcements.pinned', 'news.pinned', 'aria.pinnedArticle'] as const;

function lookup(bundle: Json, dottedKey: string): unknown {
  let cur: unknown = bundle;
  for (const part of dottedKey.split('.')) {
    if (cur !== null && typeof cur === 'object' && part in (cur as Json)) {
      cur = (cur as Json)[part];
    } else {
      return undefined;
    }
  }
  return cur;
}

describe('pinned-badge i18n leaf parity', () => {
  it('covers exactly the six shipped locale bundles', () => {
    expect(Object.keys(BUNDLES).sort()).toEqual(['cs', 'de', 'en', 'hu', 'pl', 'sk']);
  });

  for (const key of PINNED_KEYS) {
    describe(`key: ${key}`, () => {
      for (const [locale, bundle] of Object.entries(BUNDLES)) {
        it(`${locale} has a non-empty translation`, () => {
          const value = lookup(bundle, key);
          expect(typeof value, `${locale}.json is missing "${key}"`).toBe('string');
          expect(
            (value as string).trim().length,
            `${locale}.json has an empty "${key}"`
          ).toBeGreaterThan(0);
        });
      }
    });
  }

  it('gives each locale a distinct string from English (no copy-paste fallback)', () => {
    // sk/cs/de/pl/hu must actually be translated, not left as the English word.
    for (const key of PINNED_KEYS) {
      const english = lookup(en, key) as string;
      for (const locale of ['sk', 'cs', 'de', 'pl', 'hu']) {
        expect(
          lookup(BUNDLES[locale], key),
          `${locale}.json "${key}" is still the English string`
        ).not.toBe(english);
      }
    }
  });
});

describe('pinned-badge render sites route through t()', () => {
  const here = dirname(fileURLToPath(import.meta.url));
  // Every component/page that renders a pinned badge or indicator.
  const SITES: Array<{ file: string; key: string }> = [
    { file: resolve(here, 'pages/ArticleDetailPage.tsx'), key: 'news.pinned' },
    { file: resolve(here, 'components/NewsArticleCard.tsx'), key: 'aria.pinnedArticle' },
    {
      file: resolve(here, '../announcements/components/AnnouncementCard.tsx'),
      key: 'announcements.pinned',
    },
    {
      file: resolve(here, '../announcements/pages/ViewAnnouncementPage.tsx'),
      key: 'announcements.pinned',
    },
  ];

  for (const { file, key } of SITES) {
    const name = file.split('/src/')[1] ?? file;
    it(`${name} uses t('${key}') and renders no hard-coded label`, () => {
      const source = readFileSync(file, 'utf8');
      expect(source, `${name} should reference t('${key}')`).toContain(`t('${key}')`);
      // No literal JSX text node reading "Pinned" (hard-coded label regression).
      expect(/>\s*Pinned\s*</.test(source), `${name} renders a hard-coded "Pinned" literal`).toBe(
        false
      );
    });
  }
});
