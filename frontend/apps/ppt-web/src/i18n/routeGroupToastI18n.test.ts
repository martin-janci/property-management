/// <reference types="vitest/globals" />
/**
 * Route-group action-toast i18n regression.
 *
 * Several `routes/groups/*.tsx` route wrappers raised success/error toasts
 * (`showToast({ title, message })`) from their mutation `onSubmit` / `onError`
 * handlers with the `title` and `message` hardcoded in English. Those strings
 * bypass react-i18next entirely, so a manager on sk/cs/de/pl/hu saw English
 * toasts regardless of their chosen language (see
 * code-review-ppt-web-core-route-group-toast-i18n).
 *
 * This locks the fix in two ways:
 *
 *  1. Key presence — every toast key the fix introduced must exist, and be a
 *     non-empty string, in all six shipped locale bundles. A key absent from a
 *     non-en bundle degrades silently (react-i18next falls back to en.json via
 *     `fallbackLng`), so only an explicit assertion catches it.
 *  2. Source guard — none of the affected route-group modules may assign a bare
 *     string literal to a toast `title:` / `message:` field; those fields must
 *     be driven by `t(...)`. (A literal inside `t('key', { defaultValue: '…' })`
 *     is fine — only a direct `title: '…'` / `message: '…'` is rejected.)
 *
 * Both checks fail on the pre-fix tree (hardcoded literals present, keys
 * missing) and pass once every toast goes through a translation key that is
 * defined in all six locales.
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

/**
 * Toast keys the fix introduced, grouped by the namespaces they live in. Shared
 * one-word status titles ("Created", "Save failed", …) live under `common.*`;
 * the descriptive per-domain copy lives under each feature namespace.
 */
const REQUIRED_TOAST_KEYS: string[] = [
  // shared status titles + generic fallbacks
  'common.created',
  'common.updated',
  'common.saved',
  'common.deleted',
  'common.submitted',
  'common.done',
  'common.createFailed',
  'common.updateFailed',
  'common.saveFailed',
  'common.deleteFailed',
  'common.submitFailed',
  'common.pleaseTryAgain',
  // community.tsx
  'community.groupCreated',
  'community.groupJoined',
  'community.joinFailed',
  'community.groupLeft',
  'community.leaveFailed',
  // faults.tsx
  'faults.reported',
  'faults.reportFailed',
  'faults.updated',
  'faults.updateFailed',
  // iot.tsx
  'iot.sensorDeleted',
  'iot.sensorDeleteFailed',
  'iot.sensorRegistered',
  'iot.sensorRegisterFailed',
  'iot.sensorUpdated',
  'iot.sensorUpdateFailed',
  // leases.tsx
  'leases.created',
  'leases.createFailed',
  'leases.violationRecorded',
  'leases.violationCreateFailed',
  // meters.tsx
  'meters.readingSubmitted',
  'meters.submitFailed',
  'meters.correctionSubmitted',
  'meters.updateFailed',
  'meters.readingValidated',
  'meters.validationFailedTitle',
  'meters.validationFailed',
  // person-months.tsx
  'personMonths.unitsUpdated',
  'personMonths.bulkFailedTitle',
  'personMonths.bulkFailed',
  'personMonths.entryRemoved',
  'personMonths.deleteFailed',
  'personMonths.saved',
  'personMonths.saveFailed',
];

/**
 * Resolve the ppt-web package root from the test's working directory. Vitest
 * runs with cwd at the package dir; walk upward until `src/routes/groups` is
 * found so the guard is robust to being launched from the workspace root.
 */
function routeGroupsDir(): string {
  let dir = process.cwd();
  for (let i = 0; i < 6; i++) {
    const candidate = resolve(dir, 'src/routes/groups');
    if (existsSync(candidate)) return candidate;
    const apps = resolve(dir, 'apps/ppt-web/src/routes/groups');
    if (existsSync(apps)) return apps;
    const parent = dirname(dir);
    if (parent === dir) break;
    dir = parent;
  }
  throw new Error(`could not locate src/routes/groups from cwd ${process.cwd()}`);
}

/** Route-group modules whose action toasts were de-hardcoded. */
const GUARDED_MODULES = [
  'community.tsx',
  'faults.tsx',
  'iot.tsx',
  'leases.tsx',
  'meters.tsx',
  'person-months.tsx',
];

describe('route-group action toast i18n', () => {
  it('covers exactly the six shipped locale bundles', () => {
    expect(Object.keys(BUNDLES).sort()).toEqual(['cs', 'de', 'en', 'hu', 'pl', 'sk']);
  });

  for (const [locale, bundle] of Object.entries(BUNDLES)) {
    describe(`locale: ${locale}`, () => {
      for (const path of REQUIRED_TOAST_KEYS) {
        it(`defines a non-empty ${path}`, () => {
          const value = getPath(bundle, path);
          expect(value, `${path} missing in ${locale}.json`).toBeTypeOf('string');
          expect((value as string).trim().length).toBeGreaterThan(0);
        });
      }
    });
  }

  describe('source guard: toasts must not hardcode title/message literals', () => {
    for (const module of GUARDED_MODULES) {
      it(`${module} assigns no bare string literal to a toast title/message`, () => {
        const source = readFileSync(resolve(routeGroupsDir(), module), 'utf8');
        // Direct literal assignment of human copy, e.g. `title: 'Created'` /
        // `message: "x"`. A translated field reads `title: t(...)`; a literal
        // only ever survives inside `t('key', { defaultValue: '…' })`, which
        // this pattern ignores. An empty `message: ''` (a deliberately blank
        // toast body paired with a translated title) is also allowed — the
        // pattern requires at least one letter inside the quotes.
        const matches = source.match(/\b(?:title|message):\s*['"][^'"]*[A-Za-z]/g) ?? [];
        expect(
          matches,
          `${module} still hardcodes a toast title/message literal (${matches.length} occurrence(s)) — route to t(...)`
        ).toEqual([]);
      });
    }
  });
});
