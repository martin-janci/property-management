/// <reference types="vitest/globals" />
/**
 * Leases + meters i18n completeness.
 *
 * The `leases` and `meters` namespaces shipped almost English-only: ~95 % of
 * `leases.*` and ~95 % of `meters.*` keys were missing outside en.json, so a
 * manager on sk/cs/de/pl/hu silently fell back to English on every lease,
 * application, violation and meter screen (react-i18next `fallbackLng`). Count
 * strings were also glued together ("{n} days remaining", "file(s) selected"),
 * which cannot be translated into Slavic plural forms.
 *
 * This locks, for every shipped locale:
 *  1. key parity with en.json (plural keys compared by their base key),
 *  2. every CLDR plural category of the locale is present for plural keys,
 *  3. `{{placeholder}}` parity with the English source string,
 *  4. no empty values,
 * plus that every static `t('leases.…' | 'meters.…')` key used in the feature
 * code resolves in en.json, and that the former hard-coded English notices are
 * gone from the route wrappers.
 */

import { readdirSync, readFileSync, statSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import cs from '../../messages/cs.json';
import de from '../../messages/de.json';
import en from '../../messages/en.json';
import hu from '../../messages/hu.json';
import pl from '../../messages/pl.json';
import sk from '../../messages/sk.json';
import { TERMINATION_REASONS } from '../features/leases/components/LeaseActionModals';

type Json = Record<string, unknown>;

const LOCALES: Record<string, Json> = { sk, cs, de, pl, hu };
const NAMESPACES = ['leases', 'meters'] as const;
const PLURAL_SUFFIX = /_(zero|one|two|few|many|other)$/;

function flatten(obj: unknown, prefix: string, out: Record<string, string> = {}) {
  if (obj && typeof obj === 'object') {
    for (const [k, v] of Object.entries(obj as Json)) flatten(v, `${prefix}.${k}`, out);
  } else {
    out[prefix] = String(obj);
  }
  return out;
}

function namespaceKeys(bundle: Json): Record<string, string> {
  const out: Record<string, string> = {};
  for (const ns of NAMESPACES) flatten(bundle[ns], ns, out);
  return out;
}

const baseKey = (key: string) => key.replace(PLURAL_SUFFIX, '');
const placeholders = (value: string) =>
  [...value.matchAll(/\{\{\s*([\w.]+)\s*\}\}/g)].map((m) => m[1]).sort();

const enKeys = namespaceKeys(en);
const enBases = new Set(Object.keys(enKeys).map(baseKey));
const enPluralBases = new Set(
  Object.keys(enKeys)
    .filter((k) => PLURAL_SUFFIX.test(k))
    .map(baseKey)
);

/** English source value for a key (plural keys compare against `_other`). */
function enValue(base: string): string {
  return enKeys[base] ?? enKeys[`${base}_other`] ?? '';
}

describe('leases + meters i18n: en source', () => {
  it('has both namespaces with content', () => {
    expect(Object.keys(enKeys).filter((k) => k.startsWith('leases.')).length).toBeGreaterThan(300);
    expect(Object.keys(enKeys).filter((k) => k.startsWith('meters.')).length).toBeGreaterThan(140);
  });

  it('defines en plural keys as complete _one/_other pairs', () => {
    for (const base of enPluralBases) {
      expect(enKeys[`${base}_one`], `${base}_one`).toBeTruthy();
      expect(enKeys[`${base}_other`], `${base}_other`).toBeTruthy();
      expect(enKeys[base], `bare ${base} next to plural forms`).toBeUndefined();
    }
  });
});

describe.each(Object.entries(LOCALES))('leases + meters i18n: %s', (locale, bundle) => {
  const keys = namespaceKeys(bundle);
  const categories = new Intl.PluralRules(locale).resolvedOptions().pluralCategories;

  it('has exactly the en key set (by base key)', () => {
    const bases = new Set(Object.keys(keys).map(baseKey));
    const missing = [...enBases].filter((k) => !bases.has(k));
    const extra = [...bases].filter((k) => !enBases.has(k));
    expect(missing, 'missing keys').toEqual([]);
    expect(extra, 'keys not in en.json').toEqual([]);
  });

  it('provides every CLDR plural category for plural keys', () => {
    const problems: string[] = [];
    for (const base of enPluralBases) {
      if (keys[base] !== undefined) problems.push(`${base}: bare key next to plural forms`);
      for (const cat of categories) {
        if (!keys[`${base}_${cat}`]) problems.push(`${base}_${cat}`);
      }
    }
    expect(problems).toEqual([]);
  });

  it('keeps {{placeholder}} parity with en', () => {
    const problems: string[] = [];
    for (const [key, value] of Object.entries(keys)) {
      const expected = placeholders(enValue(baseKey(key)));
      const actual = placeholders(value);
      if (JSON.stringify(expected) !== JSON.stringify(actual)) {
        problems.push(`${key}: expected [${expected}] got [${actual}]`);
      }
    }
    expect(problems).toEqual([]);
  });

  it('has no empty values', () => {
    const empty = Object.entries(keys)
      .filter(([, v]) => v.trim() === '')
      .map(([k]) => k);
    expect(empty).toEqual([]);
  });
});

// ============================================================================
// Source ↔ bundle
// ============================================================================

const here = dirname(fileURLToPath(import.meta.url));
const srcRoot = resolve(here, '..');

function sourceFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return sourceFiles(path);
    return /\.tsx?$/.test(name) && !/\.test\.tsx?$/.test(name) ? [path] : [];
  });
}

const FEATURE_SOURCES = [
  ...sourceFiles(join(srcRoot, 'features/leases')),
  ...sourceFiles(join(srcRoot, 'features/meters')),
  join(srcRoot, 'routes/groups/leases.tsx'),
  join(srcRoot, 'routes/groups/meters.tsx'),
];

describe('leases + meters i18n: source usage', () => {
  it('every static leases.* / meters.* key used in code exists in en.json', () => {
    const missing: string[] = [];
    for (const file of FEATURE_SOURCES) {
      const src = readFileSync(file, 'utf8');
      for (const m of src.matchAll(/\bt\(\s*['"]((?:leases|meters)\.[\w.]+)['"]/g)) {
        const key = m[1];
        if (!enKeys[key] && !enKeys[`${key}_one`]) missing.push(`${key} (${file})`);
      }
    }
    expect(missing).toEqual([]);
  });

  it('has a label for every termination reason the dialog offers', () => {
    for (const reason of TERMINATION_REASONS) {
      expect(enKeys[`leases.actions.terminate.reasons.${reason}`], reason).toBeTruthy();
    }
  });

  it('route wrappers no longer hard-code English notices', () => {
    const leasesRoute = readFileSync(join(srcRoot, 'routes/groups/leases.tsx'), 'utf8');
    const metersRoute = readFileSync(join(srcRoot, 'routes/groups/meters.tsx'), 'utf8');
    for (const literal of ['Back to leases', 'could not be loaded', '`Loading']) {
      expect(leasesRoute).not.toContain(literal);
    }
    for (const literal of ['Loading meter…', 'Back to meters', 'could not be loaded']) {
      expect(metersRoute).not.toContain(literal);
    }
  });
});
