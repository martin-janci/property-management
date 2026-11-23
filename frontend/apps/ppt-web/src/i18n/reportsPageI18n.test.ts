/// <reference types="vitest/globals" />
/**
 * Reports page i18n regression guard
 * (code-review finding: ppt-web reports page hardcoded user-facing strings).
 *
 * `features/reports/pages/ReportsPage.tsx` is the unified reports surface
 * (report-builder tab + schedule-management tab). Every user-facing string it
 * renders — the page title, the tab labels, the "New report" / "New schedule"
 * buttons, the saved-reports empty state, the created-on caption, the "Public"
 * badge and the "Create schedule" heading — is routed through react-i18next
 * under the `reports.page.*` namespace, with full parity across the six shipped
 * locale bundles.
 *
 * This locks the page against regressing back to hardcoded English literals two
 * ways (mirrors `reportsScheduleEditErrorsI18n.test.ts` /
 * `groupDetailNotFoundI18n.test.ts`):
 *
 *  1. Key presence — every `reports.page.*` leaf the page consumes must exist,
 *     non-empty, in all six `messages/{en,sk,cs,de,pl,hu}.json` bundles.
 *  2. Source guard — `ReportsPage.tsx` must (a) reference each key through
 *     `t('reports.page.*')` and (b) carry none of the distinctive hardcoded
 *     English copy. Both fail on a tree that inlines the English strings and
 *     pass once they are routed through `t(...)`.
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

/** Keys the Reports page renders through `t(...)`. */
const REQUIRED_KEYS = [
  'reports.page.title',
  'reports.page.tabs.reports',
  'reports.page.tabs.schedules',
  'reports.page.newReport',
  'reports.page.newSchedule',
  'reports.page.savedReports',
  'reports.page.noReports',
  'reports.page.createFirstReport',
  'reports.page.createdOn',
  'reports.page.public',
  'reports.page.createSchedule',
];

/**
 * Distinctive English copy that must not survive as a hardcoded JSX literal in
 * the page (the pre-i18n shape inlined these strings). The `createdOn` value
 * uses an interpolation (`Created {{date}}`) so it is covered by the key-parity
 * check above rather than a literal match.
 */
const FORBIDDEN_LITERALS = [
  'Reports & analytics',
  'Saved reports',
  'No reports created yet',
  'Create your first report',
  'Create schedule',
];

/**
 * Resolve the ppt-web `src/features/reports/pages` dir from the test's cwd
 * (Vitest runs with cwd at the package dir; walk upward so the guard also works
 * from the workspace root). Mirrors `reportsScheduleEditErrorsI18n.test.ts`.
 */
function reportsPagesDir(): string {
  let dir = process.cwd();
  for (let i = 0; i < 6; i++) {
    const candidate = resolve(dir, 'src/features/reports/pages');
    if (existsSync(candidate)) return candidate;
    const apps = resolve(dir, 'apps/ppt-web/src/features/reports/pages');
    if (existsSync(apps)) return apps;
    const parent = dirname(dir);
    if (parent === dir) break;
    dir = parent;
  }
  throw new Error(`could not locate src/features/reports/pages from cwd ${process.cwd()}`);
}

describe('reports page i18n', () => {
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

  describe('source guard: ReportsPage routes every string through react-i18next', () => {
    const source = readFileSync(resolve(reportsPagesDir(), 'ReportsPage.tsx'), 'utf8');

    for (const key of REQUIRED_KEYS) {
      it(`references t('${key}')`, () => {
        expect(
          source.includes(`'${key}'`),
          `ReportsPage.tsx no longer references t('${key}') — keep the page wired through react-i18next`
        ).toBe(true);
      });
    }

    for (const literal of FORBIDDEN_LITERALS) {
      it(`contains no hardcoded '${literal}'`, () => {
        expect(
          source.includes(literal),
          `ReportsPage.tsx still renders the hardcoded English string '${literal}' — route it through t('reports.page.*')`
        ).toBe(false);
      });
    }
  });
});
