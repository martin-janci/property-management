/// <reference types="vitest/globals" />
/**
 * Accounting Invoice feature i18n + native-dialog regression.
 *
 * The ppt-web Accounting Invoice surface (list + form + management page) shipped
 * entirely in hardcoded English: every label, status, table header, button and
 * validation string was a bare literal, and the feature used the browser-native
 * `alert()` / `confirm()` instead of the app's standard inline-validation +
 * `ConfirmationDialog` + toast pattern. A manager on sk/cs/de/pl/hu therefore saw
 * English copy throughout and got unstyled native browser dialogs
 * (see code-review-ppt-web-ui-invoice-feature-no-i18n).
 *
 * This locks the fix two ways:
 *
 *  1. Key presence — every `accounting.*` key the fix introduced must exist, and
 *     be a non-empty string, in all six shipped locale bundles. A key absent from
 *     a non-en bundle degrades silently (react-i18next falls back to en.json via
 *     `fallbackLng`), so only an explicit assertion catches it.
 *  2. Source guard — none of the three Accounting Invoice modules may call the
 *     native `alert(` / `confirm(` dialogs; destructive confirmation goes through
 *     `ConfirmationDialog` and validation through inline error state.
 *
 * Both checks fail on the pre-fix tree (hardcoded literals + native dialogs) and
 * pass once the feature is translated and the native dialogs are replaced.
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

/** Every leaf `accounting.*` key the fix introduced. */
const REQUIRED_KEYS: string[] = [
  'accounting.title',
  'accounting.subtitle',
  'accounting.createInvoice',
  'accounting.newInvoice',
  'accounting.createFailed',
  'accounting.deleteFailed',
  'accounting.deleteConfirmTitle',
  'accounting.deleteConfirmMessage',
  'accounting.emptyTitle',
  'accounting.emptyBody',
  'accounting.due',
  'accounting.vat',
  'accounting.editLabel',
  'accounting.deleteLabel',
  'accounting.status.draft',
  'accounting.status.issued',
  'accounting.status.paid',
  'accounting.status.partiallyPaid',
  'accounting.status.overdue',
  'accounting.form.contact',
  'accounting.form.selectContact',
  'accounting.form.invoiceNumber',
  'accounting.form.variableSymbol',
  'accounting.form.issueDate',
  'accounting.form.dueDate',
  'accounting.form.currency',
  'accounting.form.vatCountry',
  'accounting.form.country.cz',
  'accounting.form.country.sk',
  'accounting.form.lineItems',
  'accounting.form.addItem',
  'accounting.form.itemDescriptionPlaceholder',
  'accounting.form.customPercent',
  'accounting.form.remove',
  'accounting.form.baseAmount',
  'accounting.form.vatAmount',
  'accounting.form.total',
  'accounting.form.updateInvoice',
  'accounting.form.col.description',
  'accounting.form.col.qty',
  'accounting.form.col.unitPrice',
  'accounting.form.col.vatRate',
  'accounting.form.col.total',
  'accounting.form.vatRate.standard',
  'accounting.form.vatRate.reduced',
  'accounting.form.vatRate.firstReduced',
  'accounting.form.vatRate.zero',
  'accounting.form.errors.requiredFields',
  'accounting.form.errors.invalidNumbers',
];

/**
 * Resolve the ppt-web package root from the test's working directory. Vitest
 * runs with cwd at the package dir; walk upward until `src/features/accounting`
 * is found so the guard is robust to being launched from the workspace root.
 */
function accountingDir(): string {
  let dir = process.cwd();
  for (let i = 0; i < 6; i++) {
    const candidate = resolve(dir, 'src/features/accounting');
    if (existsSync(candidate)) return candidate;
    const apps = resolve(dir, 'apps/ppt-web/src/features/accounting');
    if (existsSync(apps)) return apps;
    const parent = dirname(dir);
    if (parent === dir) break;
    dir = parent;
  }
  throw new Error(`could not locate src/features/accounting from cwd ${process.cwd()}`);
}

/** Accounting Invoice modules that must not use native alert/confirm. */
const GUARDED_MODULES = [
  'components/AccountingInvoiceList.tsx',
  'components/AccountingInvoiceForm.tsx',
  'pages/AccountingInvoiceManagementPage.tsx',
];

describe('Accounting Invoice i18n + native-dialog', () => {
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

  describe('source guard: no native alert/confirm dialogs', () => {
    for (const module of GUARDED_MODULES) {
      it(`${module} does not call alert()/confirm()`, () => {
        const source = readFileSync(resolve(accountingDir(), module), 'utf8');
        // Match a call to the global alert/confirm (optionally window-prefixed),
        // not substrings like `confirmLabel` or `confirmationDialog`.
        expect(source).not.toMatch(/(?<![.\w])(?:window\.)?alert\s*\(/);
        expect(source).not.toMatch(/(?<![.\w])(?:window\.)?confirm\s*\(/);
      });
    }
  });
});
