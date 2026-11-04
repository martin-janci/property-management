/// <reference types="vitest/globals" />
/// <reference types="vite/client" />
/**
 * The whole `rentals` namespace must be translated in all six shipped locales:
 * every key en has (plural keys in every CLDR form the locale uses), the same
 * interpolation placeholders, no stale keys — and every static `rentals.*` key
 * the rentals pages / route wrappers read must exist in en.
 */
import cs from '../../messages/cs.json';
import de from '../../messages/de.json';
import en from '../../messages/en.json';
import hu from '../../messages/hu.json';
import pl from '../../messages/pl.json';
import sk from '../../messages/sk.json';

type Json = Record<string, unknown>;
const LOCALES: Record<string, Json> = { sk, cs, de, pl, hu };
const PLURAL_SUFFIX = /_(zero|one|two|few|many|other)$/;

function flatten(obj: Json, prefix: string, out: Record<string, string> = {}) {
  for (const [key, value] of Object.entries(obj)) {
    const path = `${prefix}.${key}`;
    if (value && typeof value === 'object') flatten(value as Json, path, out);
    else out[path] = value as string;
  }
  return out;
}

const placeholders = (s: string) => [...s.matchAll(/\{\{\s*(\w+)\s*\}\}/g)].map((m) => m[1]).sort();

const EN = flatten((en as Json).rentals as Json, 'rentals');
const EN_PLAIN = Object.keys(EN).filter((k) => !PLURAL_SUFFIX.test(k));
const EN_PLURAL_BASES = [
  ...new Set(
    Object.keys(EN)
      .filter((k) => PLURAL_SUFFIX.test(k))
      .map((k) => k.replace(PLURAL_SUFFIX, ''))
  ),
];
/** Union of the placeholders over en's plural forms of one base. */
const pluralPlaceholders = (base: string) =>
  new Set(
    Object.keys(EN)
      .filter((k) => k.replace(PLURAL_SUFFIX, '') === base && k !== base)
      .flatMap((k) => placeholders(EN[k]))
  );

describe('rentals i18n completeness', () => {
  it('en has a sizeable rentals namespace (sanity)', () => {
    expect(EN_PLAIN.length).toBeGreaterThan(300);
    expect(EN_PLURAL_BASES.length).toBeGreaterThan(10);
  });

  for (const [lang, bundle] of Object.entries(LOCALES)) {
    const LOC = flatten(((bundle as Json).rentals ?? {}) as Json, 'rentals');

    it(`${lang} has every en rentals key as a non-empty string`, () => {
      const missing = EN_PLAIN.filter((k) => typeof LOC[k] !== 'string' || LOC[k].trim() === '');
      expect(missing).toEqual([]);
    });

    it(`${lang} has every plural form its language uses`, () => {
      const forms = new Intl.PluralRules(lang).resolvedOptions().pluralCategories;
      const missing = EN_PLURAL_BASES.flatMap((base) =>
        forms.map((f) => `${base}_${f}`).filter((k) => typeof LOC[k] !== 'string')
      );
      expect(missing).toEqual([]);
    });

    it(`${lang} has no rentals keys that en dropped`, () => {
      const enBases = new Set(Object.keys(EN).map((k) => k.replace(PLURAL_SUFFIX, '')));
      const stale = Object.keys(LOC).filter((k) => !enBases.has(k.replace(PLURAL_SUFFIX, '')));
      expect(stale).toEqual([]);
    });

    it(`${lang} keeps the en interpolation placeholders`, () => {
      const wrong: string[] = [];
      for (const k of EN_PLAIN) {
        if (typeof LOC[k] !== 'string') continue;
        if (placeholders(LOC[k]).join() !== placeholders(EN[k]).join()) wrong.push(k);
      }
      for (const k of Object.keys(LOC).filter((key) => PLURAL_SUFFIX.test(key))) {
        const allowed = pluralPlaceholders(k.replace(PLURAL_SUFFIX, ''));
        if (placeholders(LOC[k]).some((p) => !allowed.has(p))) wrong.push(k);
      }
      expect(wrong).toEqual([]);
    });
  }

  it('every static rentals.* key read by the rentals UI exists in en', () => {
    const sources = {
      ...import.meta.glob('../features/rentals/**/*.tsx', {
        query: '?raw',
        import: 'default',
        eager: true,
      }),
      ...import.meta.glob('../routes/groups/rentals.tsx', {
        query: '?raw',
        import: 'default',
        eager: true,
      }),
    } as Record<string, string>;
    expect(Object.keys(sources).length).toBeGreaterThan(10);
    const enKeys = new Set(Object.keys(EN));
    const enBases = new Set(Object.keys(EN).map((k) => k.replace(PLURAL_SUFFIX, '')));
    const missing: string[] = [];
    let checked = 0;
    for (const [file, src] of Object.entries(sources)) {
      if (file.includes('.test.')) continue;
      for (const m of src.matchAll(/['"`](rentals\.[A-Za-z0-9_.]+)['"`]/g)) {
        const key = m[1];
        checked += 1;
        if (!enKeys.has(key) && !enBases.has(key)) missing.push(`${file}: ${key}`);
      }
    }
    expect(checked).toBeGreaterThan(200);
    expect(missing).toEqual([]);
  });

  it('uses the agreed Slovak and Czech property-management terminology', () => {
    const SK = flatten((sk as Json).rentals as Json, 'rentals');
    const CS = flatten((cs as Json).rentals as Json, 'rentals');
    expect(SK['rentals.dashboard.occupancyRate']).toBe('Obsadenosť');
    expect(SK['rentals.bookings.title']).toBe('Rezervácie');
    expect(SK['rentals.bookingDetail.checkIn']).toBe('Príchod');
    expect(SK['rentals.bookingDetail.checkOut']).toBe('Odchod');
    expect(SK['rentals.bookingDetail.primaryGuest']).toBe('Hlavný hosť');
    expect(SK['rentals.dashboard.title']).toBe('Krátkodobé prenájmy');
    expect(CS['rentals.dashboard.occupancyRate']).toBe('Obsazenost');
    expect(CS['rentals.bookings.title']).toBe('Rezervace');
    expect(CS['rentals.bookingDetail.checkOut']).toBe('Odjezd');
  });
});
