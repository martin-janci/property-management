/// <reference types="vitest/globals" />
/**
 * Financial reports export-button i18n regression guard
 * (code-review finding: ppt-web financial feature rendered hardcoded
 * user-facing strings not routed through react-i18next).
 *
 * `features/financial/pages/FinancialReportsPage.tsx` renders two export
 * buttons whose idle labels — "PDF" and "Excel" — used to be inlined as
 * hardcoded JSX string literals (`... : 'PDF'` / `... : 'Excel'`) while the
 * rest of the page was already localized. They are now routed through
 * react-i18next under `financial.reports.pdf` / `financial.reports.excel`,
 * matching the existing `financial.invoices.pdf` precedent, with full parity
 * across the six shipped locale bundles.
 *
 * This locks the buttons against regressing back to hardcoded literals two
 * ways (mirrors `reportsPageI18n.test.ts`):
 *
 *  1. Key presence — every consumed `financial.reports.*` leaf must exist,
 *     non-empty, in all six `messages/{en,sk,cs,de,pl,hu}.json` bundles.
 *  2. Source guard — `FinancialReportsPage.tsx` must (a) reference each key
 *     through `t('financial.reports.*')` and (b) carry none of the old
 *     hardcoded `: 'PDF'` / `: 'Excel'` JSX literals. Both fail on a tree that
 *     inlines the English strings and pass once routed through `t(...)`.
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

/** Keys the export buttons render through `t(...)`. */
const REQUIRED_KEYS = [
  'financial.reports.export',
  'financial.reports.exporting',
  'financial.reports.pdf',
  'financial.reports.excel',
];

/**
 * Distinctive hardcoded fragments that must not survive in the page source
 * (the pre-i18n shape inlined the idle button labels as JSX literals).
 */
const FORBIDDEN_LITERALS = [": 'PDF'", ": 'Excel'"];

/**
 * Resolve the ppt-web `src/features/financial/pages` dir from the test's cwd
 * (Vitest runs with cwd at the package dir; walk upward so the guard also
 * works from the workspace root). Mirrors `reportsPageI18n.test.ts`.
 */
function financialPagesDir(): string {
  let dir = process.cwd();
  for (let i = 0; i < 6; i++) {
    const candidate = resolve(dir, 'src/features/financial/pages');
    if (existsSync(candidate)) return candidate;
    const apps = resolve(dir, 'apps/ppt-web/src/features/financial/pages');
    if (existsSync(apps)) return apps;
    const parent = dirname(dir);
    if (parent === dir) break;
    dir = parent;
  }
  throw new Error(`could not locate src/features/financial/pages from cwd ${process.cwd()}`);
}

describe('financial reports export i18n', () => {
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

  describe('source guard: FinancialReportsPage routes export labels through react-i18next', () => {
    const source = readFileSync(resolve(financialPagesDir(), 'FinancialReportsPage.tsx'), 'utf8');

    for (const key of REQUIRED_KEYS) {
      it(`references t('${key}')`, () => {
        expect(
          source.includes(`'${key}'`),
          `FinancialReportsPage.tsx no longer references t('${key}') — keep the export buttons wired through react-i18next`
        ).toBe(true);
      });
    }

    for (const literal of FORBIDDEN_LITERALS) {
      it(`contains no hardcoded ${literal.trim()}`, () => {
        expect(
          source.includes(literal),
          `FinancialReportsPage.tsx still inlines the hardcoded export label ${literal.trim()} — route it through t('financial.reports.*')`
        ).toBe(false);
      });
    }
  });
});
