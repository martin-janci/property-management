/// <reference types="vitest/globals" />
/**
 * Locale interpolation-placeholder parity regression.
 *
 * `localeNamespaceParity.test.ts` and `localeLeafParity.test.ts` lock the
 * *structure* of the six message bundles (`messages/{en,sk,cs,de,pl,hu}.json`)
 * — which namespaces and which leaf keys exist. They say nothing about the
 * *contents* of a leaf value. That leaves one class of i18n breakage
 * unguarded: a leaf can carry an i18next interpolation placeholder — `{{id}}`,
 * `{{count}}`, `{{message}}` — that the `en` reference (the `fallbackLng`,
 * configured in `i18n/index.ts`) does not define.
 *
 * This matters because the six bundles are edited *identically* by every i18n
 * PR (one feature's keys land in all six files at once) and are therefore a
 * chronic merge-train conflict hotspot (see `localeNamespaceParity.test.ts`'s
 * header). A mis-resolved conflict, a copy/paste from the wrong row, or a
 * translator typo can introduce a placeholder a locale value uses but the
 * reference never supplies. i18next has nothing to substitute for such a
 * variable, so — unlike a dropped key, which falls back to en — it renders the
 * raw `{{var}}` braces straight into the UI for every user on that locale. No
 * existing guard catches it, and the key still resolves, so it ships silently.
 *
 * The invariant is deliberately one-directional:
 *
 *   NO-EXTRA — for every leaf a non-reference locale carries whose base key
 *              exists in the en reference, the set of `{{placeholder}}` names in
 *              the locale value must be a SUBSET of the names the en reference
 *              defines for that base key. An extra name is a guaranteed literal
 *              `{{...}}` leak.
 *
 * The reverse direction (a locale using *fewer* placeholders than en) is NOT
 * asserted: it is routinely legitimate. Singular plural forms drop the count by
 * design — en itself has `intervalHours_one: "Every hour"` (no `{{count}}`)
 * next to `intervalHours_other: "Every {{count}} hours"` — and some locales
 * phrase a message without echoing an id the reference happens to interpolate.
 * Those are translation choices, not leaks, so they stay out of scope.
 *
 * Plural variants (`_one`/`_few`/`_many`/`_other`/…) of one base key are unioned
 * on the en side before the subset check, so a locale's `_many` form may use any
 * placeholder that appears in any en variant of the same base key.
 */

import cs from '../../messages/cs.json';
import de from '../../messages/de.json';
import en from '../../messages/en.json';
import hu from '../../messages/hu.json';
import pl from '../../messages/pl.json';
import sk from '../../messages/sk.json';

type Json = Record<string, unknown>;

const BUNDLES: Record<string, Json> = { en, sk, cs, de, pl, hu };
const NON_REFERENCE = ['sk', 'cs', 'de', 'pl', 'hu'] as const;

/** Trailing i18next/CLDR plural-category suffix. */
const PLURAL_SUFFIX = /_(zero|one|two|few|many|other)$/;

/**
 * i18next double-brace interpolation token. The variable name is the leading
 * identifier; an optional `, format` tail (`{{count, number}}`) is ignored.
 */
const PLACEHOLDER = /\{\{\s*([\w$.]+)\s*(?:,[^}]*)?\}\}/g;

/** Collect every leaf value (dot-joined path -> string value) in a bundle. */
function leafEntries(
  obj: Json,
  prefix = '',
  out: Array<[string, string]> = []
): Array<[string, string]> {
  for (const [key, value] of Object.entries(obj)) {
    const path = prefix ? `${prefix}.${key}` : key;
    if (value !== null && typeof value === 'object' && !Array.isArray(value)) {
      leafEntries(value as Json, path, out);
    } else if (typeof value === 'string') {
      out.push([path, value]);
    }
  }
  return out;
}

/** Interpolation variable names used in a message string. */
function placeholderNames(value: string): Set<string> {
  const names = new Set<string>();
  for (const match of value.matchAll(PLACEHOLDER)) {
    names.add(match[1]);
  }
  return names;
}

/** base leaf key (plural suffix stripped) -> union of en placeholder names. */
function referencePlaceholders(): Map<string, Set<string>> {
  const byBase = new Map<string, Set<string>>();
  for (const [path, value] of leafEntries(en)) {
    const base = path.replace(PLURAL_SUFFIX, '');
    const names = byBase.get(base) ?? new Set<string>();
    for (const n of placeholderNames(value)) names.add(n);
    byBase.set(base, names);
  }
  return byBase;
}

const EN_PLACEHOLDERS = referencePlaceholders();

describe('locale interpolation-placeholder parity', () => {
  // Guard the locale set itself: a newly added bundle must be listed here or
  // the sweep below would silently skip it.
  it('covers exactly the six shipped locale bundles', () => {
    expect(Object.keys(BUNDLES).sort()).toEqual(['cs', 'de', 'en', 'hu', 'pl', 'sk']);
  });

  // NO-EXTRA — a locale leaf may not use an interpolation variable the en
  // reference does not define for that base key (it would render as literal
  // `{{var}}` braces to the user).
  for (const locale of NON_REFERENCE) {
    it(`${locale}.json uses no interpolation placeholder absent from en.json`, () => {
      const offenders: string[] = [];
      for (const [path, value] of leafEntries(BUNDLES[locale])) {
        const base = path.replace(PLURAL_SUFFIX, '');
        const reference = EN_PLACEHOLDERS.get(base);
        // Base key absent from en is a dead key — localeLeafParity's NO-EXTRA
        // owns that; skip so this guard reports only true placeholder leaks.
        if (reference === undefined) continue;
        const extra = [...placeholderNames(value)].filter((name) => !reference.has(name));
        if (extra.length > 0) {
          offenders.push(`${path} -> {{${extra.join('}}, {{')}}}`);
        }
      }
      expect(
        offenders,
        `${locale}.json has leaf value(s) using an interpolation variable the en ` +
          `fallback never supplies — these render as literal braces at runtime:\n  ${offenders.join('\n  ')}`
      ).toEqual([]);
    });
  }
});
