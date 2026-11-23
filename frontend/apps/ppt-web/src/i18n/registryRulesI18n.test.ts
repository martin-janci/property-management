/// <reference types="vitest/globals" />
/// <reference types="vite/client" />
/**
 * Registry Rules page + form (heading, subtitle, error states, info callout and
 * every form label / placeholder / button) must be routed through i18n and
 * present in every shipped locale.
 *
 * Regression guard for the code-review finding that
 * `features/registry/pages/RegistryRulesPage.tsx` and
 * `features/registry/components/RegistryRulesForm.tsx` hard-coded English copy
 * (page heading + subtitle, the error-state titles/bodies, the "About Registry
 * Rules" callout, and the whole rules form) — shipping untranslated text for
 * sk/cs/de/pl/hu. The form had no `useTranslation` at all. This locks the
 * `registry.rules.{page,error,info,form}.*` keys into all six bundles and
 * asserts the two source files no longer carry those English literals.
 */
import cs from '../../messages/cs.json';
import de from '../../messages/de.json';
import en from '../../messages/en.json';
import hu from '../../messages/hu.json';
import pl from '../../messages/pl.json';
import sk from '../../messages/sk.json';

type Json = Record<string, unknown>;
const ALL: Record<string, Json> = { en, sk, cs, de, pl, hu };

/** Leaf keys the Registry Rules page + form read, relative to `registry.rules`. */
const RULES_KEYS = [
  'page.title',
  'page.subtitle',
  'error.buildingRequiredTitle',
  'error.loadFailedTitle',
  'error.buildingRequiredBody',
  'error.loadFailedBody',
  'error.goBack',
  'info.iconLabel',
  'info.heading',
  'info.item1',
  'info.item2',
  'info.item3',
  'info.item4',
  'form.petRulesHeading',
  'form.petsAllowed',
  'form.petsRequireApproval',
  'form.maxPetsPerUnit',
  'form.allowedPetTypes',
  'form.bannedBreeds',
  'form.bannedBreedPlaceholder',
  'form.add',
  'form.maxPetWeight',
  'form.vehicleRulesHeading',
  'form.vehiclesRequireApproval',
  'form.maxVehiclesPerUnit',
  'form.additionalNotes',
  'form.notesPlaceholder',
  'form.noLimit',
  'form.cancel',
  'form.save',
  'form.saving',
] as const;

const leaf = (bundle: Json, path: string): unknown =>
  path.split('.').reduce<unknown>(
    (acc, part) => {
      if (acc && typeof acc === 'object') return (acc as Json)[part];
      return undefined;
    },
    (bundle.registry as Json)?.rules
  );

describe('registry rules i18n', () => {
  for (const [lang, bundle] of Object.entries(ALL)) {
    it(`${lang} has every registry.rules page/form key as a non-empty string`, () => {
      const missing = RULES_KEYS.filter((k) => {
        const v = leaf(bundle, k);
        return typeof v !== 'string' || v.trim() === '';
      });
      expect(missing).toEqual([]);
    });
  }

  it('RegistryRulesPage.tsx + RegistryRulesForm.tsx route copy through t() instead of hard-coding English', () => {
    const sources = import.meta.glob(
      [
        '../features/registry/pages/RegistryRulesPage.tsx',
        '../features/registry/components/RegistryRulesForm.tsx',
      ],
      { query: '?raw', import: 'default', eager: true }
    ) as Record<string, string>;
    const combined = Object.values(sources).join('\n');
    expect(Object.keys(sources).length, 'both registry-rules sources must be found').toBe(2);

    const banned = [
      'Registry Rules',
      'Configure pet and vehicle registration rules for your building',
      'Building ID Required',
      'Failed to Load Registry Rules',
      'About Registry Rules',
      'Pet Rules',
      'Pets are allowed in this building',
      'Vehicle Rules',
      'Save Rules',
      'No limit',
    ];
    const leaked = banned.filter((s) => combined.includes(s));
    expect(leaked, `hard-coded English still present: ${leaked.join(' | ')}`).toEqual([]);

    for (const k of RULES_KEYS) {
      expect(combined, `missing t('registry.rules.${k}')`).toContain(`registry.rules.${k}`);
    }
  });
});
