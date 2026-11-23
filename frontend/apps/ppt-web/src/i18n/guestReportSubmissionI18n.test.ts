/// <reference types="vitest/globals" />
/**
 * Guest report submission page i18n regression guard
 * (code-review finding: ppt-web guest report-submission flow rendered
 * hardcoded English instead of routing through react-i18next).
 *
 * `features/government-portal/pages/GuestReportSubmissionPage.tsx` is the
 * guest-registration submission wizard (country/connection/template selection,
 * data preview, submit progress, completion). Every user-facing string it
 * renders — the header, wizard step labels, validation errors, the
 * country/connection/template selectors, the preview table headers, and the
 * submitting/complete states — is routed through react-i18next under the
 * `governmentPortal.guestReport.*` namespace, with full key parity across the
 * six shipped locale bundles.
 *
 * This locks the page against regressing back to hardcoded English literals two
 * ways (mirrors `reportsPageI18n.test.ts`):
 *
 *  1. Key presence — every `governmentPortal.guestReport.*` leaf the page
 *     consumes must exist, non-empty, in all six
 *     `messages/{en,sk,cs,de,pl,hu}.json` bundles.
 *  2. Source guard — `GuestReportSubmissionPage.tsx` must (a) reference each key
 *     through `t('governmentPortal.guestReport.*')` and (b) carry none of the
 *     distinctive hardcoded English copy. Both fail on a tree that inlines the
 *     English strings and pass once they are routed through `t(...)`.
 */

import { existsSync, readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
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

/** Keys the guest report submission page renders through `t(...)`. */
const REQUIRED_KEYS = [
  'governmentPortal.guestReport.backToDashboard',
  'governmentPortal.guestReport.title',
  'governmentPortal.guestReport.subtitle',
  'governmentPortal.guestReport.steps.selectOptions',
  'governmentPortal.guestReport.steps.previewData',
  'governmentPortal.guestReport.steps.submit',
  'governmentPortal.guestReport.errors.selectConnection',
  'governmentPortal.guestReport.errors.selectTemplate',
  'governmentPortal.guestReport.errors.selectGuest',
  'governmentPortal.guestReport.errors.submissionFailed',
  'governmentPortal.guestReport.country.sk',
  'governmentPortal.guestReport.country.cz',
  'governmentPortal.guestReport.country.skDescription',
  'governmentPortal.guestReport.country.czDescription',
  'governmentPortal.guestReport.testMode',
  'governmentPortal.guestReport.select.country',
  'governmentPortal.guestReport.select.portalConnection',
  'governmentPortal.guestReport.select.noConnections',
  'governmentPortal.guestReport.select.noCode',
  'governmentPortal.guestReport.select.reportTemplate',
  'governmentPortal.guestReport.select.noTemplates',
  'governmentPortal.guestReport.select.templatePlaceholder',
  'governmentPortal.guestReport.select.guestsHeading',
  'governmentPortal.guestReport.select.selectAll',
  'governmentPortal.guestReport.select.noGuests',
  'governmentPortal.guestReport.actions.cancel',
  'governmentPortal.guestReport.actions.previewSubmission',
  'governmentPortal.guestReport.actions.backToSelection',
  'governmentPortal.guestReport.actions.submitToPortal',
  'governmentPortal.guestReport.preview.title',
  'governmentPortal.guestReport.preview.subtitle',
  'governmentPortal.guestReport.preview.destinationPortal',
  'governmentPortal.guestReport.preview.unknown',
  'governmentPortal.guestReport.preview.reportTemplate',
  'governmentPortal.guestReport.preview.country',
  'governmentPortal.guestReport.preview.guestsToSubmit',
  'governmentPortal.guestReport.preview.guestCount',
  'governmentPortal.guestReport.preview.guestData',
  'governmentPortal.guestReport.preview.table.name',
  'governmentPortal.guestReport.preview.table.dob',
  'governmentPortal.guestReport.preview.table.nationality',
  'governmentPortal.guestReport.preview.table.document',
  'governmentPortal.guestReport.preview.table.checkIn',
  'governmentPortal.guestReport.preview.table.unit',
  'governmentPortal.guestReport.submitting.title',
  'governmentPortal.guestReport.submitting.message',
  'governmentPortal.guestReport.submitting.percentComplete',
  'governmentPortal.guestReport.submitting.processing',
  'governmentPortal.guestReport.complete.title',
  'governmentPortal.guestReport.complete.message',
  'governmentPortal.guestReport.complete.guestsSubmitted',
  'governmentPortal.guestReport.complete.portal',
];

/**
 * Distinctive English copy that must not survive as a hardcoded JSX text node in
 * the page. Each literal is the *angle-bracket-delimited* JSX form the pre-i18n
 * tree inlined (e.g. `>Submit Guest Report<`) — deliberately NOT the bare
 * English phrase, because the fix routes strings through
 * `t(key, { defaultValue: 'English' })` and so keeps the English copy in source
 * as a `defaultValue`. Matching the `>…<` wrapper flags a re-inlined JSX literal
 * while never colliding with a legitimate `defaultValue` string (which carries
 * no `>`/`</tag>` delimiters). Each was a single-line JSX node pre-fix and is
 * absent post-fix, so the guard fails on the hardcoded tree and passes once the
 * strings are wired through react-i18next.
 */
const FORBIDDEN_LITERALS = [
  '>Submit Guest Report<',
  '>Select Country</h2>',
  '>Portal Connection</h2>',
  '>Report Template</h2>',
  '>Submission Preview</h2>',
  '>Guest Data</h3>',
  '>Submission Complete!<',
  '>Guests Submitted<',
];

/**
 * Resolve the ppt-web `src/features/government-portal/pages` dir from the test's
 * cwd (Vitest runs with cwd at the package dir; walk upward so the guard also
 * works from the workspace root). Mirrors `reportsPageI18n.test.ts`.
 */
function governmentPortalPagesDir(): string {
  let dir = process.cwd();
  for (let i = 0; i < 6; i++) {
    const candidate = resolve(dir, 'src/features/government-portal/pages');
    if (existsSync(candidate)) return candidate;
    const apps = resolve(dir, 'apps/ppt-web/src/features/government-portal/pages');
    if (existsSync(apps)) return apps;
    const parent = dirname(dir);
    if (parent === dir) break;
    dir = parent;
  }
  throw new Error(
    `could not locate src/features/government-portal/pages from cwd ${process.cwd()}`
  );
}

describe('guest report submission page i18n', () => {
  it('covers exactly the six shipped locale bundles', () => {
    expect(Object.keys(BUNDLES).sort()).toEqual(['cs', 'de', 'en', 'hu', 'pl', 'sk']);
  });

  for (const [locale, bundle] of Object.entries(BUNDLES)) {
    describe(`locale: ${locale}`, () => {
      for (const path of REQUIRED_KEYS) {
        it(`defines a non-empty ${path}`, () => {
          const value = getPath(bundle, path);
          expect(value, `${path} missing in ${locale}.json`).toBeTypeOf('string');
          expect((value as string).trim().length).toBeGreaterThan(0);
        });
      }
    });
  }

  describe('source guard: GuestReportSubmissionPage routes every string through react-i18next', () => {
    const source = readFileSync(
      resolve(governmentPortalPagesDir(), 'GuestReportSubmissionPage.tsx'),
      'utf8'
    );

    for (const key of REQUIRED_KEYS) {
      it(`references t('${key}')`, () => {
        expect(
          source.includes(`'${key}'`),
          `GuestReportSubmissionPage.tsx no longer references t('${key}') — keep the page wired through react-i18next`
        ).toBe(true);
      });
    }

    for (const literal of FORBIDDEN_LITERALS) {
      it(`contains no hardcoded '${literal}'`, () => {
        expect(
          source.includes(literal),
          `GuestReportSubmissionPage.tsx still renders the hardcoded English string '${literal}' — route it through t('governmentPortal.guestReport.*')`
        ).toBe(false);
      });
    }
  });
});
