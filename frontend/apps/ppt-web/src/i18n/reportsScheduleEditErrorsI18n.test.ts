/// <reference types="vitest/globals" />
/**
 * Report schedule-editing validation i18n regression
 * (code-review finding: ppt-web core validation-error i18n gap).
 *
 * The Story 81.1 report-schedule editing flow validated user input but rendered
 * the resulting validation messages as hardcoded English string literals, so
 * they never translated for sk/cs/de/pl/hu users:
 *
 *   - `EditScheduleModal.tsx`  — 'A valid cron expression is required',
 *                                'At least one recipient is required'
 *   - `RecipientManager.tsx`   — 'Please enter a valid email address',
 *                                'This email is already added',
 *                                `Invalid emails: ${...}`
 *
 * The fix routes all five through react-i18next under a new
 * `reports.schedule.editErrors.*` namespace (sibling of the existing
 * `reports.schedule.createErrors`), added with full parity across all six
 * bundles.
 *
 * This locks the fix two ways (mirrors `groupDetailNotFoundI18n.test.ts`):
 *
 *  1. Key presence — every `editErrors` leaf must exist, non-empty, in all six
 *     `messages/{en,sk,cs,de,pl,hu}.json` bundles.
 *  2. Source guard — the two components must no longer carry the hardcoded
 *     English validation literals. This fails on the pre-fix tree (the literals
 *     are present) and passes once the strings are routed through `t(...)`.
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

/** New translated keys the schedule-editing validation branch must use. */
const REQUIRED_KEYS = [
  'reports.schedule.editErrors.invalidCron',
  'reports.schedule.editErrors.recipientsRequired',
  'reports.schedule.editErrors.invalidEmail',
  'reports.schedule.editErrors.duplicateEmail',
  'reports.schedule.editErrors.invalidEmailsList',
];

/** Hardcoded English validation literals that must not survive the fix. */
const FORBIDDEN_LITERALS: Record<string, string[]> = {
  'EditScheduleModal.tsx': [
    'A valid cron expression is required',
    'At least one recipient is required',
  ],
  'RecipientManager.tsx': [
    'Please enter a valid email address',
    'This email is already added',
    'Invalid emails:',
  ],
};

/**
 * Resolve the ppt-web `src/features/reports/components` dir from the test's cwd
 * (Vitest runs with cwd at the package dir; walk upward so the guard also works
 * from the workspace root). Mirrors `groupDetailNotFoundI18n.test.ts`.
 */
function reportsComponentsDir(): string {
  let dir = process.cwd();
  for (let i = 0; i < 6; i++) {
    const candidate = resolve(dir, 'src/features/reports/components');
    if (existsSync(candidate)) return candidate;
    const apps = resolve(dir, 'apps/ppt-web/src/features/reports/components');
    if (existsSync(apps)) return apps;
    const parent = dirname(dir);
    if (parent === dir) break;
    dir = parent;
  }
  throw new Error(`could not locate src/features/reports/components from cwd ${process.cwd()}`);
}

describe('report schedule-editing validation i18n', () => {
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

  describe('source guard: schedule-editing components use no hardcoded validation literals', () => {
    const dir = reportsComponentsDir();
    for (const [file, literals] of Object.entries(FORBIDDEN_LITERALS)) {
      const source = readFileSync(resolve(dir, file), 'utf8');
      for (const literal of literals) {
        it(`${file} no longer contains hardcoded '${literal}'`, () => {
          expect(
            source.includes(literal),
            `${file} still renders the hardcoded English validation string '${literal}' — route it through t('reports.schedule.editErrors.*')`
          ).toBe(false);
        });
      }
    }
  });
});
