/// <reference types="vitest/globals" />
/**
 * Regression test for the active-locale helper used to thread the user's
 * selected language into `Intl.*` / `toLocale*` formatting.
 *
 * Before this helper existed, ppt-web called `toLocaleDateString()` /
 * `toLocaleString()` / `new Intl.NumberFormat(undefined, …)` without a locale,
 * so dates and numbers rendered in the JS runtime default locale instead of the
 * language the manager picked. `getActiveLocale()` must track `i18n.language`.
 */

import i18n from 'i18next';
import { afterAll, describe, expect, it } from 'vitest';
import { getActiveLocale } from './locale';

describe('getActiveLocale', () => {
  afterAll(async () => {
    // Restore the setup-file default so sibling tests are unaffected.
    await i18n.changeLanguage('en');
  });

  it('returns the active i18n language', async () => {
    await i18n.changeLanguage('sk');
    expect(getActiveLocale()).toBe('sk');

    await i18n.changeLanguage('de');
    expect(getActiveLocale()).toBe('de');
  });

  it('reflects a runtime language switch immediately', async () => {
    await i18n.changeLanguage('cs');
    const first = getActiveLocale();
    await i18n.changeLanguage('pl');
    const second = getActiveLocale();

    expect(first).toBe('cs');
    expect(second).toBe('pl');
    expect(first).not.toBe(second);
  });

  it('threads into Intl.NumberFormat so grouping follows the locale', async () => {
    // en-US groups with a comma and uses a dot decimal; sk uses a space group
    // and a comma decimal. Formatting the same value with getActiveLocale()
    // must differ between the two languages.
    await i18n.changeLanguage('en');
    const en = new Intl.NumberFormat(getActiveLocale()).format(1234567.89);
    await i18n.changeLanguage('sk');
    const sk = new Intl.NumberFormat(getActiveLocale()).format(1234567.89);

    expect(en).toContain(',');
    expect(en).not.toBe(sk);
  });
});
