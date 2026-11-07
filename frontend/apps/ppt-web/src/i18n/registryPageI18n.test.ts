/// <reference types="vitest/globals" />
/// <reference types="vite/client" />
/**
 * RegistryPage chrome (title, subtitle, pets/vehicles tab labels, load-error)
 * must be routed through i18n and present in every shipped locale.
 *
 * Regression guard for the code-review finding that
 * `features/registry/pages/RegistryPage.tsx` hard-coded English for the page
 * heading, subtitle, the two tab labels and the load-error banner while the
 * rest of the file already used `t()` — shipping untranslated text for
 * sk/cs/de/pl/hu. This locks the `registry.page.*` keys into all six bundles
 * and asserts the page no longer carries those English literals.
 */
import cs from '../../messages/cs.json';
import de from '../../messages/de.json';
import en from '../../messages/en.json';
import hu from '../../messages/hu.json';
import pl from '../../messages/pl.json';
import sk from '../../messages/sk.json';

type Json = Record<string, unknown>;
const ALL: Record<string, Json> = { en, sk, cs, de, pl, hu };

/** Leaf keys the RegistryPage chrome reads, relative to `registry.page`. */
const PAGE_KEYS = ['title', 'subtitle', 'tabs.pets', 'tabs.vehicles', 'loadError'] as const;

const leaf = (bundle: Json, path: string): unknown =>
  path.split('.').reduce<unknown>((acc, part) => {
    if (acc && typeof acc === 'object') return (acc as Json)[part];
    return undefined;
  }, (bundle.registry as Json)?.page);

describe('registry page i18n', () => {
  for (const [lang, bundle] of Object.entries(ALL)) {
    it(`${lang} has every registry.page chrome key as a non-empty string`, () => {
      const missing = PAGE_KEYS.filter((k) => {
        const v = leaf(bundle, k);
        return typeof v !== 'string' || v.trim() === '';
      });
      expect(missing).toEqual([]);
    });
  }

  it('RegistryPage.tsx routes its chrome through t() instead of hard-coding English', () => {
    const sources = import.meta.glob('../features/registry/pages/RegistryPage.tsx', {
      query: '?raw',
      import: 'default',
      eager: true,
    }) as Record<string, string>;
    const src = Object.values(sources)[0];
    expect(src, 'RegistryPage.tsx not found').toBeTruthy();

    const banned = [
      'Building Registries',
      'Manage pet and vehicle registrations for your building',
      'Failed to load registrations. Please try again later.',
    ];
    const leaked = banned.filter((s) => src.includes(s));
    expect(leaked, `hard-coded English still present: ${leaked.join(' | ')}`).toEqual([]);

    for (const k of PAGE_KEYS) {
      expect(src, `missing t('registry.page.${k}')`).toContain(`registry.page.${k}`);
    }
  });
});
