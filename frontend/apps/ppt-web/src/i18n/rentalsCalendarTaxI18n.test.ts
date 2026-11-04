/// <reference types="vitest/globals" />
/**
 * The rentals calendar and daňový výkaz pages must be translated in all six
 * shipped locales (the rest of the `rentals` namespace is still en-only).
 * Every key these pages read must be a non-empty string in each bundle,
 * including every CLDR plural form the locale uses.
 */
import cs from '../../messages/cs.json';
import de from '../../messages/de.json';
import en from '../../messages/en.json';
import hu from '../../messages/hu.json';
import pl from '../../messages/pl.json';
import sk from '../../messages/sk.json';

type Json = Record<string, unknown>;
const BUNDLES: Record<string, Json> = { en, sk, cs, de, pl, hu };

function getPath(obj: unknown, path: string): unknown {
  return path.split('.').reduce<unknown>((acc, part) => {
    if (acc === null || typeof acc !== 'object') return undefined;
    return (acc as Json)[part];
  }, obj);
}

const CALENDAR = [
  'title',
  'subtitle',
  'selectUnit',
  'selectUnitPrompt',
  'selectUnitDesc',
  'today',
  'previousMonth',
  'nextMonth',
  'booking',
  'blocked',
  'maintenance',
  'more',
  'tip',
  'tipDesc',
  'blockDates',
  'blockDatesButton',
  'blockDatesDesc',
  'blockFrom',
  'blockTo',
  'invalidRange',
  'reason',
  'notes',
  'reasonPlaceholder',
  'confirmBlock',
  'blockReasons.owner_use',
  'blockReasons.maintenance',
  'blockReasons.blocked',
  'removeBlock',
  'noEvents',
  'noUnits',
  'noUnitsDesc',
  'unitsLoadFailed',
  'loadFailed',
  'overlap',
  'blockCreated',
  'blockCreateFailed',
  'blockRemoved',
  'blockRemoveFailed',
].map((k) => `rentals.calendar.${k}`);

const PLURALS = ['daysCount', 'bookingsCount', 'blocksCount'].map((k) => `rentals.calendar.${k}`);

const TAX = [
  'pageTitle',
  'pageSubtitle',
  'configureReport',
  'building',
  'selectBuilding',
  'authority',
  'periodType',
  'periodTypes.monthly',
  'periodTypes.quarterly',
  'periodTypes.annual',
  'yearLabel',
  'month',
  'quarter',
  'quarterLabel',
  'showPreview',
  'generating',
  'previewTitle',
  'saveReport',
  'saving',
  'totalGuests',
  'totalBookings',
  'countries',
  'byNationality',
  'nationality',
  'guests',
  'share',
  'unknownNationality',
  'noGuestsInPeriod',
  'countingRule',
  'incomeNotAvailable',
  'noReportGenerated',
  'noReportGeneratedDesc',
  'noBuildings',
  'noBuildingsDesc',
  'savedReports',
  'noSavedReports',
  'period',
  'status',
  'actions',
  'statuses.draft',
  'statuses.generated',
  'statuses.submitted',
  'statuses.confirmed',
  'download',
  'markSubmitted',
  'saved',
  'submittedToast',
  'errors.previewFailed',
  'errors.saveFailed',
  'errors.buildingsFailed',
  'errors.submitFailed',
  'authorities.SK_UHUL',
  'authorities.CZ_CIZPOL',
  'authorities.AT_ZMR',
  'authorities.DE_MELDEWESEN',
].map((k) => `rentals.tax.${k}`);

describe('rentals calendar + tax report i18n', () => {
  for (const [lang, bundle] of Object.entries(BUNDLES)) {
    it(`${lang} has every key`, () => {
      const missing = [...CALENDAR, ...TAX].filter((key) => {
        const v = getPath(bundle, key);
        return typeof v !== 'string' || v.trim() === '';
      });
      expect(missing).toEqual([]);
    });

    it(`${lang} has every plural form`, () => {
      const forms = new Intl.PluralRules(lang).resolvedOptions().pluralCategories;
      const missing = PLURALS.flatMap((key) =>
        forms.map((form) => `${key}_${form}`).filter((k) => typeof getPath(bundle, k) !== 'string')
      );
      expect(missing).toEqual([]);
    });
  }

  it('uses the agreed Slovak terminology', () => {
    expect(getPath(sk, 'rentals.tax.pageTitle')).toBe('Daňový výkaz');
    expect(getPath(sk, 'rentals.calendar.booking')).toBe('Rezervácia');
    expect(getPath(sk, 'rentals.tax.building')).toBe('Budova');
  });
});
