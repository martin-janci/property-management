/// <reference types="vitest/globals" />
/**
 * Disputes i18n completeness — the whole `disputes.*` namespace.
 *
 * The narrower `disputeListI18n` / `fileDisputeI18n` / `draftAutosaveI18n`
 * tests pin specific keys. This one guards the namespace as a whole so the
 * dispute screens (list, filing, detail, mediation) cannot regress to
 * hard-coded or fallback English:
 *
 *  1. key parity — every locale has exactly the same `disputes.*` keys;
 *  2. plurals — every pluralised key has every form `Intl.PluralRules`
 *     requires for that locale (sk/cs/pl: one/few/many/other);
 *  3. placeholders — each translation keeps the same `{{vars}}` and `<n>` tags
 *     as English, so nothing renders a literal `{{name}}`;
 *  4. usage — every literal `disputes.*` key in the disputes source (feature
 *     folder + route group) exists in en.json.
 */

import { readdirSync, readFileSync, statSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import cs from '../../../messages/cs.json';
import de from '../../../messages/de.json';
import en from '../../../messages/en.json';
import hu from '../../../messages/hu.json';
import pl from '../../../messages/pl.json';
import sk from '../../../messages/sk.json';

type Json = { [key: string]: unknown };

const BUNDLES: Record<string, Json> = { en, sk, cs, de, pl, hu };
const PLURAL_SUFFIX = /_(zero|one|two|few|many|other)$/;

function flatten(obj: unknown, prefix = ''): Record<string, string> {
  const out: Record<string, string> = {};
  for (const [key, value] of Object.entries(obj as Json)) {
    const path = prefix ? `${prefix}.${key}` : key;
    if (value !== null && typeof value === 'object') Object.assign(out, flatten(value, path));
    else out[path] = String(value);
  }
  return out;
}

const FLAT: Record<string, Record<string, string>> = Object.fromEntries(
  Object.entries(BUNDLES).map(([locale, bundle]) => [locale, flatten(bundle.disputes, 'disputes')])
);

/** Keys with any plural suffix collapsed to their base. */
function baseKeys(flat: Record<string, string>): Set<string> {
  return new Set(Object.keys(flat).map((k) => k.replace(PLURAL_SUFFIX, '')));
}

function pluralBases(flat: Record<string, string>): Set<string> {
  return new Set(
    Object.keys(flat)
      .filter((k) => PLURAL_SUFFIX.test(k))
      .map((k) => k.replace(PLURAL_SUFFIX, ''))
  );
}

function tokens(value: string): string[] {
  const vars = value.match(/\{\{\s*[\w.]+\s*\}\}/g) ?? [];
  const tags = value.match(/<\/?\d+>/g) ?? [];
  return [...new Set([...vars.map((v) => v.replace(/\s/g, '')), ...tags])].sort();
}

/** English reference string for a (possibly plural-suffixed) key. */
function englishFor(key: string): string | undefined {
  const flat = FLAT.en;
  if (key in flat) return flat[key];
  const base = key.replace(PLURAL_SUFFIX, '');
  return flat[`${base}_other`];
}

describe('disputes.* i18n completeness', () => {
  it('covers exactly the six shipped locale bundles', () => {
    expect(Object.keys(BUNDLES).sort()).toEqual(['cs', 'de', 'en', 'hu', 'pl', 'sk']);
  });

  const reference = [...baseKeys(FLAT.en)].sort();

  for (const locale of Object.keys(BUNDLES)) {
    describe(`locale: ${locale}`, () => {
      it('has the same disputes.* keys as en (no missing, no extra)', () => {
        expect([...baseKeys(FLAT[locale])].sort()).toEqual(reference);
      });

      it('has no empty disputes.* values', () => {
        const empty = Object.entries(FLAT[locale])
          .filter(([, v]) => v.trim().length === 0)
          .map(([k]) => k);
        expect(empty).toEqual([]);
      });

      it('has every plural form the locale requires', () => {
        const required = new Intl.PluralRules(locale)
          .resolvedOptions()
          .pluralCategories.filter((c) => ['one', 'few', 'many', 'other'].includes(c));
        const missing: string[] = [];
        for (const base of pluralBases(FLAT.en)) {
          for (const category of required) {
            if (!(`${base}_${category}` in FLAT[locale])) missing.push(`${base}_${category}`);
          }
        }
        expect(missing, `${locale}.json is missing plural forms`).toEqual([]);
      });

      it('keeps the English placeholders and tags in every value', () => {
        const drifted: string[] = [];
        for (const [key, value] of Object.entries(FLAT[locale])) {
          const english = englishFor(key);
          if (english === undefined) continue; // parity test reports it
          if (tokens(value).join() !== tokens(english).join()) drifted.push(key);
        }
        expect(drifted, `${locale}.json placeholders differ from en`).toEqual([]);
      });
    });
  }

  it('defines in en.json every literal disputes.* key the disputes code uses', () => {
    const here = dirname(fileURLToPath(import.meta.url));
    const files: string[] = [resolve(here, '../../routes/groups/disputes.tsx')];
    const walk = (dir: string) => {
      for (const name of readdirSync(dir)) {
        const path = join(dir, name);
        if (statSync(path).isDirectory()) walk(path);
        else if (/\.tsx?$/.test(name) && !/\.test\.tsx?$/.test(name)) files.push(path);
      }
    };
    walk(here);
    expect(files.length).toBeGreaterThan(10);

    const used = new Set<string>();
    for (const file of files) {
      const source = readFileSync(file, 'utf8');
      for (const match of source.matchAll(/['"`](disputes\.[\w.]+)['"`]/g)) used.add(match[1]);
    }
    expect(used.size).toBeGreaterThan(100);

    const enFlat = FLAT.en;
    const missing = [...used].filter((key) => !(key in enFlat) && !(`${key}_other` in enFlat));
    expect(missing).toEqual([]);
  });
});
