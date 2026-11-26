/// <reference types="vitest/globals" />
/**
 * Buildings feature i18n regression guard.
 *
 * The Buildings pages — list, detail, create/edit form, units table + unit
 * form, residents dialog, location map, and the resident "My Unit" view
 * (`features/buildings/**` + `features/my-unit/pages/MyUnitPage.tsx` +
 * `routes/groups/buildings.tsx`) — route every user-facing string through
 * `t('buildings.*')` (plus a handful of shared `common.*` / `errors.*` /
 * `facilities.nav.*` / `aria.*` keys). Nothing is rendered as a hardcoded
 * literal.
 *
 * Because untranslated JSX literals are valid TypeScript, neither typecheck nor
 * lint can catch a regression where a future edit reintroduces a raw English
 * string, or where a `buildings.*` key is dropped from a locale bundle and
 * silently falls back to the key / to English. This test locks both:
 *
 *  1. Bundle parity — every `buildings.*` leaf present in en.json exists as a
 *     non-empty string in all six shipped locales, with no per-locale drift.
 *  2. Render-layer resolution — every i18n key the Buildings pages actually
 *     read (static keys + the enumerated dynamic `t(`buildings.<ns>.${v}`)`
 *     variants) resolves through `t()` to a real, non-empty translation that is
 *     NOT the key itself, in English and in a non-English locale (sk) — i.e. the
 *     strings genuinely come from the bundles, not from a missing-key fallback.
 *  3. Interpolation parity — keys carrying `{{placeholder}}` tokens keep them in
 *     every locale, and the rendered output substitutes them.
 */

import { createInstance } from 'i18next';
import cs from '../../messages/cs.json';
import de from '../../messages/de.json';
import en from '../../messages/en.json';
import hu from '../../messages/hu.json';
import pl from '../../messages/pl.json';
import sk from '../../messages/sk.json';

type Json = Record<string, unknown>;

const BUNDLES: Record<string, Json> = { en, sk, cs, de, pl, hu };

/** Dotted leaf-key paths reachable from a nested object. */
function leafPaths(obj: unknown, prefix = ''): string[] {
  if (obj === null || typeof obj !== 'object') return [prefix];
  return Object.entries(obj as Json).flatMap(([k, v]) =>
    leafPaths(v, prefix ? `${prefix}.${k}` : k)
  );
}

function getPath(obj: unknown, path: string): unknown {
  return path.split('.').reduce<unknown>((acc, part) => {
    if (acc === null || typeof acc !== 'object') return undefined;
    return (acc as Json)[part];
  }, obj);
}

/** Expand a dynamic `t(`${prefix}.${value}`)` call site into its full key set. */
const withPrefix = (prefix: string, values: readonly string[]): string[] =>
  values.map((v) => `${prefix}.${v}`);

/** Every `buildings.*` leaf-key path, taken from en.json as the reference. */
const BUILDINGS_PATHS: string[] = leafPaths((en as Json).buildings, 'buildings');

/**
 * The concrete i18n keys the Buildings pages pass to `t(...)`. Static keys are
 * listed verbatim; dynamic `t(`buildings.<ns>.${value}`)` call sites are
 * expanded to the full set of enum values they can receive (from the backing
 * union types / option arrays in the components).
 */
const STATUS = ['active', 'under_construction', 'renovation', 'inactive'];
const BUILDING_TYPE = ['residential', 'commercial', 'mixed', 'industrial'];
const UNIT_TYPE = ['apartment', 'commercial', 'parking', 'storage', 'other'];
const OCCUPANCY = ['unknown', 'owner_occupied', 'rented', 'vacant'];
const RESIDENT_TYPE = ['owner', 'tenant', 'family_member', 'subtenant'];
const COMMON_AREA_TYPE = [
  'staircase',
  'elevator',
  'lobby',
  'hallway',
  'basement',
  'attic',
  'parking',
  'garage',
  'garden',
  'playground',
  'pool',
  'gym',
  'laundry_room',
  'storage_room',
  'other',
];
const FORM_ERRORS = [
  'streetRequired',
  'cityRequired',
  'postalCodeRequired',
  'floorsInvalid',
  'entrancesInvalid',
  'yearInvalid',
];

/** Keys read with no interpolation and no plural `count` — plain `t(key)`. */
const PLAIN_PAGE_KEYS: string[] = [
  // BuildingsPage / BuildingList / BuildingCard
  'buildings.title',
  'buildings.view',
  'buildings.units',
  'buildings.floors',
  'buildings.yearBuilt',
  'buildings.errors.loadFailed',
  'buildings.errors.unexpected',
  'buildings.errors.notFoundBody',
  'buildings.errors.generic',
  'buildings.list.addBuilding',
  'buildings.list.emptyTitle',
  'buildings.list.emptyBody',
  'buildings.list.statusFilter',
  'buildings.list.typeFilter',
  'buildings.list.allStatuses',
  'buildings.list.allTypes',
  // BuildingDetailPage
  'buildings.detail.type',
  'buildings.detail.totalArea',
  'buildings.detail.personMonths',
  'buildings.detail.commonAreas',
  'buildings.detail.noCommonAreas',
  'buildings.detail.noFloors',
  // BuildingFormPage
  'buildings.form.createTitle',
  'buildings.form.editTitle',
  'buildings.form.name',
  'buildings.form.street',
  'buildings.form.postalCode',
  'buildings.form.city',
  'buildings.form.country',
  'buildings.form.floorCount',
  'buildings.form.totalEntrances',
  'buildings.form.yearBuilt',
  'buildings.form.description',
  'buildings.form.saving',
  'buildings.form.saveFailed',
  // BuildingLocationMap
  'buildings.map.title',
  'buildings.map.openInOsm',
  'buildings.map.iframeTitleFallback',
  // UnitForm
  'buildings.unitForm.addTitle',
  'buildings.unitForm.editTitle',
  'buildings.unitForm.designation',
  'buildings.unitForm.designationPlaceholder',
  'buildings.unitForm.floor',
  'buildings.unitForm.entrance',
  'buildings.unitForm.type',
  'buildings.unitForm.occupancy',
  'buildings.unitForm.size',
  'buildings.unitForm.rooms',
  'buildings.unitForm.share',
  'buildings.unitForm.description',
  'buildings.unitForm.saving',
  'buildings.unitForm.errors.designationRequired',
  'buildings.unitForm.errors.floorInteger',
  'buildings.unitForm.errors.shareRequired',
  'buildings.unitForm.errors.shareRange',
  'buildings.unitForm.errors.sizeInvalid',
  'buildings.unitForm.errors.roomsInvalid',
  // UnitsSection
  'buildings.unitsSection.showArchived',
  'buildings.unitsSection.loadFailed',
  'buildings.unitsSection.loadFailedBody',
  'buildings.unitsSection.empty',
  'buildings.unitsSection.unit',
  'buildings.unitsSection.status',
  'buildings.unitsSection.active',
  'buildings.unitsSection.archived',
  'buildings.unitsSection.restore',
  'buildings.unitsSection.archive',
  'buildings.unitsSection.archiving',
  'buildings.unitsSection.archiveTitle',
  'buildings.unitsSection.floorGround',
  // UnitResidentsDialog
  'buildings.residents.manage',
  'buildings.residents.empty',
  'buildings.residents.primary',
  'buildings.residents.end',
  'buildings.residents.addTitle',
  'buildings.residents.person',
  'buildings.residents.choosePerson',
  'buildings.residents.typeLabel',
  'buildings.residents.primaryLabel',
  'buildings.residents.notListed',
  'buildings.residents.add',
  'buildings.residents.adding',
  'buildings.residents.errors.loadFailed',
  'buildings.residents.errors.addFailed',
  'buildings.residents.errors.duplicate',
  // MyUnitPage
  'buildings.myUnit.title',
  'buildings.myUnit.subtitle',
  'buildings.myUnit.loading',
  'buildings.myUnit.unknownError',
  'buildings.myUnit.emptyNotRegistered',
  'buildings.myUnit.emptyContactManager',
  'buildings.myUnit.chooseUnit',
  'buildings.myUnit.size',
  'buildings.myUnit.ownershipShare',
  'buildings.myUnit.building',
  'buildings.myUnit.residency',
  'buildings.myUnit.role',
  'buildings.myUnit.primaryContact',
  'buildings.myUnit.active',
  'buildings.myUnit.ended',
  'buildings.myUnit.since',
  'buildings.myUnit.until',
  'buildings.myUnit.privacyNote',
  // Shared keys the Buildings pages consume
  'common.backToBuildings',
  'common.save',
  'common.saveChanges',
  'common.cancel',
  'common.close',
  'common.edit',
  'common.actions',
  'common.optional',
  'common.loading',
  'common.previous',
  'common.next',
  'common.yes',
  'common.no',
  'errors.buildingNotFound',
  'facilities.nav.buildingFacilities',
  'facilities.nav.label',
  'facilities.nav.book',
  'facilities.nav.myBookings',
  'aria.closeDialog',
  'aria.cancel',
  'aria.building',
  'aria.unitDetails',
  'aria.yourResidency',
  // Enumerated dynamic variants
  ...withPrefix('buildings.status', STATUS),
  ...withPrefix('buildings.type', BUILDING_TYPE),
  ...withPrefix('buildings.unitType', UNIT_TYPE),
  ...withPrefix('buildings.occupancy', OCCUPANCY),
  ...withPrefix('buildings.residents.type', RESIDENT_TYPE),
  ...withPrefix('buildings.myUnit.residentType', RESIDENT_TYPE),
  ...withPrefix('buildings.commonAreaType', COMMON_AREA_TYPE),
  ...withPrefix('buildings.form.errors', FORM_ERRORS),
];

/** Keys read with interpolation values, and the tokens they must keep. */
const INTERPOLATED: Record<string, { values: Record<string, unknown>; tokens: string[] }> = {
  'buildings.detail.floorFallback': { values: { number: 3 }, tokens: ['{{number}}'] },
  'buildings.map.iframeTitle': { values: { name: 'Hviezdna 7' }, tokens: ['{{name}}'] },
  'buildings.unitForm.errors.designationTooLong': { values: { max: 50 }, tokens: ['{{max}}'] },
  'buildings.unitsSection.floorBasement': { values: { level: 2 }, tokens: ['{{level}}'] },
  'buildings.residents.title': { values: { unit: '3B' }, tokens: ['{{unit}}'] },
  'buildings.myUnit.loadError': { values: { message: 'boom' }, tokens: ['{{message}}'] },
  'buildings.myUnit.unitHeading': { values: { designation: '3B' }, tokens: ['{{designation}}'] },
  'buildings.myUnit.entrance': { values: { entrance: 'A' }, tokens: ['{{entrance}}'] },
};

/** Plural keys read with a `count`, and the expected CLDR variants in en. */
const PLURAL_KEYS: string[] = [
  'buildings.list.subtitle',
  'buildings.card.unitCount',
  'buildings.card.floorCount',
];

/** Build a fully-initialised i18next instance for one locale (sync, in-memory). */
function makeInstance(lng: string) {
  const inst = createInstance();
  // initImmediate:false keeps init synchronous for in-memory resources, so the
  // returned `t` is usable immediately.
  inst.init({
    lng,
    fallbackLng: false, // disable fallback so a missing key echoes the key, not English
    supportedLngs: Object.keys(BUNDLES),
    resources: Object.fromEntries(Object.entries(BUNDLES).map(([l, b]) => [l, { translation: b }])),
    interpolation: { escapeValue: false },
    initImmediate: false,
  });
  return inst;
}

/**
 * Assert a `t()` result is a genuine translation: a non-empty string that is
 * NOT the key itself (which is what i18next echoes on a missing-key fallback).
 * `note` disambiguates repeated calls for one key (e.g. plural branches).
 */
function expectRealTranslation(out: string, key: string, locale: string, note = '') {
  const where = note ? `${locale} ${note}` : locale;
  expect(typeof out).toBe('string');
  expect(out.trim().length, `${key} resolved empty in ${where}`).toBeGreaterThan(0);
  expect(out, `${key} fell back to the raw key in ${where}`).not.toBe(key);
}

describe('buildings.* i18n keys', () => {
  it('covers exactly the six shipped locale bundles', () => {
    expect(Object.keys(BUNDLES).sort()).toEqual(['cs', 'de', 'en', 'hu', 'pl', 'sk']);
  });

  it('reference key set spans every Buildings sub-view', () => {
    expect(BUILDINGS_PATHS).toEqual(
      expect.arrayContaining([
        'buildings.title',
        'buildings.list.emptyTitle',
        'buildings.card.unitCount_one',
        'buildings.detail.noFloors',
        'buildings.form.createTitle',
        'buildings.unitForm.addTitle',
        'buildings.unitsSection.archiveTitle',
        'buildings.residents.addTitle',
        'buildings.map.title',
        'buildings.myUnit.title',
      ])
    );
  });

  // ── 1. Bundle parity ────────────────────────────────────────────────────
  for (const [locale, bundle] of Object.entries(BUNDLES)) {
    describe(`locale: ${locale}`, () => {
      for (const path of BUILDINGS_PATHS) {
        it(`defines a non-empty ${path}`, () => {
          const value = getPath(bundle, path);
          expect(value, `${path} missing in ${locale}.json`).toBeTypeOf('string');
          expect((value as string).trim().length).toBeGreaterThan(0);
        });
      }
    });
  }

  it('keeps every locale in lockstep — no locale has missing buildings keys', () => {
    for (const [locale, bundle] of Object.entries(BUNDLES)) {
      const present = BUILDINGS_PATHS.filter((p) => typeof getPath(bundle, p) === 'string');
      expect(present, `${locale}.json buildings-key set drifted from the reference`).toEqual(
        BUILDINGS_PATHS
      );
    }
  });

  // ── 2. Render-layer resolution through t() ──────────────────────────────
  for (const locale of ['en', 'sk']) {
    describe(`resolves through t() in ${locale}`, () => {
      const t = makeInstance(locale).getFixedT(locale);

      for (const key of PLAIN_PAGE_KEYS) {
        it(`t('${key}') returns a real translation`, () => {
          expectRealTranslation(t(key), key, locale);
        });
      }

      for (const [key, spec] of Object.entries(INTERPOLATED)) {
        it(`t('${key}') interpolates in ${locale}`, () => {
          const out = t(key, spec.values as never);
          expect(out, `${key} fell back to the raw key in ${locale}`).not.toBe(key);
          for (const token of spec.tokens) {
            expect(out, `${key} left ${token} un-substituted in ${locale}`).not.toContain(token);
          }
        });
      }

      for (const key of PLURAL_KEYS) {
        it(`t('${key}', {count}) resolves both plural branches in ${locale}`, () => {
          expectRealTranslation(t(key, { count: 1 } as never), key, locale, '(count=1)');
          expectRealTranslation(t(key, { count: 5 } as never), key, locale, '(count=5)');
        });
      }
    });
  }

  // ── 3. Interpolation placeholder parity across locales ──────────────────
  for (const [locale, bundle] of Object.entries(BUNDLES)) {
    it(`${locale} preserves {{placeholder}} tokens`, () => {
      const broken = Object.entries(INTERPOLATED).flatMap(([key, spec]) => {
        const v = getPath(bundle, key);
        if (typeof v !== 'string') return [`${key} missing in ${locale}`];
        return spec.tokens
          .filter((tok) => !v.includes(tok))
          .map((tok) => `${key} missing ${tok} in ${locale}`);
      });
      expect(broken).toEqual([]);
    });
  }

  // ── 4. Slovak terminology spot-check (default locale is sk) ─────────────
  it('uses the shipped Slovak terminology', () => {
    expect(getPath(sk, 'buildings.title')).toBe('Budovy');
    expect(getPath(sk, 'buildings.map.title')).toBe('Poloha');
  });
});
