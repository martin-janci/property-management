/// <reference types="vitest/globals" />
/**
 * Locale plural-category completeness regression.
 *
 * `localeLeafParity.test.ts` locks *base-key* parity — it strips the trailing
 * `_zero|_one|_two|_few|_many|_other` suffix before comparing, so a key that
 * exists in en as a proper plural (`key_one` + `key_other`) is considered
 * present in pl as long as pl carries *any* suffixed variant. That leaves a
 * blind spot: a locale can carry `key_one` + `key_other` but be missing the
 * `_few`/`_many` forms its own CLDR plural rules require.
 *
 * i18next's JSON v4 plural format (the default — `config.ts` sets no
 * `compatibilityJSON`) resolves a count to a category via `Intl.PluralRules`
 * for the *active* language and looks up `key_<category>`. When that exact
 * suffix is missing, resolution falls through to `fallbackLng` (en), whose
 * only categories are `one`/`other` — so a Polish screen rendering a "few"
 * count (2–4) or a "many" count (0, 5+) for such a key shows the **English**
 * string. This shipped repeatedly via the i18n leaf-parity sweeps, which kept
 * base-key parity green while leaving `_few`/`_many` holes (e.g. pl
 * `automation.*`, `dashboard.*`, `shell.inbox.*`; sk/cs `automation.*`,
 * `moderation.stats.overdue`).
 *
 * This test closes that gap. For every base key en treats as a real plural
 * (en carries both `_one` and `_other`), each non-reference locale that
 * carries the key's top-level namespace must provide every plural-category
 * suffix its own `Intl.PluralRules` lists. de/hu need only `one`/`other`
 * (which the leaf-parity guard already guarantees), so in practice this binds
 * the west-slavic trio sk/cs/pl to carry `_few` and `_many` too.
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

/** Collect every leaf path (dot-joined) in a bundle. */
function leafPaths(obj: Json, prefix = '', out: Set<string> = new Set()): Set<string> {
  for (const [key, value] of Object.entries(obj)) {
    const path = prefix ? `${prefix}.${key}` : key;
    if (value !== null && typeof value === 'object' && !Array.isArray(value)) {
      leafPaths(value as Json, path, out);
    } else {
      out.add(path);
    }
  }
  return out;
}

/** Map each plural base (suffix stripped) to the set of suffixes it carries. */
function pluralGroups(bundle: Json): Map<string, Set<string>> {
  const groups = new Map<string, Set<string>>();
  for (const path of leafPaths(bundle)) {
    const m = path.match(PLURAL_SUFFIX);
    if (!m) continue;
    const base = path.replace(PLURAL_SUFFIX, '');
    (groups.get(base) ?? groups.set(base, new Set()).get(base)!).add(m[1]);
  }
  return groups;
}

/** CLDR plural categories a locale needs, per the platform's Intl.PluralRules. */
function requiredCategories(locale: string): string[] {
  return new Intl.PluralRules(locale).resolvedOptions().pluralCategories;
}

const EN_GROUPS = pluralGroups(en);
/** Base keys en treats as a real plural (both one and other present). */
const EN_REAL_PLURALS = [...EN_GROUPS.entries()]
  .filter(([, cats]) => cats.has('one') && cats.has('other'))
  .map(([base]) => base)
  .sort();

describe('locale plural-category completeness', () => {
  it('covers exactly the six shipped locale bundles', () => {
    expect(Object.keys(BUNDLES).sort()).toEqual(['cs', 'de', 'en', 'hu', 'pl', 'sk']);
  });

  it('en carries at least one real plural group to anchor the sweep', () => {
    expect(EN_REAL_PLURALS.length).toBeGreaterThan(0);
  });

  for (const locale of NON_REFERENCE) {
    it(`${locale}.json carries every CLDR plural category it needs for en's plural keys`, () => {
      const bundle = BUNDLES[locale];
      const carriedNamespaces = new Set(Object.keys(bundle));
      const groups = pluralGroups(bundle);
      const required = requiredCategories(locale);

      const incomplete: string[] = [];
      for (const base of EN_REAL_PLURALS) {
        // Skip whole namespaces the locale doesn't carry — that's the
        // namespace parity suite's concern, not this one.
        if (!carriedNamespaces.has(base.split('.')[0])) continue;
        const present = groups.get(base);
        if (!present) {
          incomplete.push(`${base} (no plural forms)`);
          continue;
        }
        const missing = required.filter((cat) => !present.has(cat));
        if (missing.length > 0) {
          incomplete.push(`${base} (missing ${missing.map((c) => `_${c}`).join(', ')})`);
        }
      }

      expect(
        incomplete,
        `${locale}.json has plural key(s) missing a CLDR category required by Intl.PluralRules('${locale}') ` +
          `[${required.join(', ')}]. Missing categories fall back to the en text at runtime: ` +
          incomplete.join('; ')
      ).toEqual([]);
    });
  }
});
