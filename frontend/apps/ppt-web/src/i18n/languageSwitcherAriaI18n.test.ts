/// <reference types="vitest/globals" />
/**
 * `aria.selectLanguage` i18n key-presence regression.
 *
 * The LanguageSwitcher control is the one element a non-English screen-reader
 * user relies on to find and change their language, yet its `aria-label` was
 * hardcoded to the English string "Select language" (see
 * code-review-ppt-web-ui-lang-switcher-aria). The label now routes through
 * `t('aria.selectLanguage')`, which falls back to `en.json` via `fallbackLng`
 * when a locale is missing the key — so a missing key announces English to
 * sk/cs/de/pl/hu users without tripping typecheck or lint.
 *
 * This locks the key into all six shipped locale bundles. It overlaps the
 * broader `ariaI18n.test.ts` parity sweep on purpose: it pins the specific key
 * this fix added so the regression is named and cannot silently drift out.
 */

import cs from '../../messages/cs.json';
import de from '../../messages/de.json';
import en from '../../messages/en.json';
import hu from '../../messages/hu.json';
import pl from '../../messages/pl.json';
import sk from '../../messages/sk.json';

type Json = Record<string, unknown>;

const BUNDLES: Record<string, Json> = { en, sk, cs, de, pl, hu };

describe('aria.selectLanguage i18n key', () => {
  for (const [locale, bundle] of Object.entries(BUNDLES)) {
    it(`defines a non-empty aria.selectLanguage in ${locale}.json`, () => {
      const aria = bundle.aria as Json | undefined;
      const value = aria?.selectLanguage;
      expect(value, `aria.selectLanguage missing in ${locale}.json`).toBeTypeOf('string');
      expect((value as string).trim().length).toBeGreaterThan(0);
    });
  }
});
