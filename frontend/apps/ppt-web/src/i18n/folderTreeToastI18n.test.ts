/// <reference types="vitest/globals" />
/**
 * Documents folder-tree toast / notification i18n regression.
 *
 * The documents-view folder tree (`features/documents/components/FolderTree.tsx`)
 * raised its create/rename/delete error toasts and its expand/collapse +
 * document-count `aria-label`s with the copy hardcoded in the source — and
 * *mixed-locale*: the toast fall-backs were English (`'Rename failed'`,
 * `'Failed to delete folder'`, …) while the aria-labels were Slovak
 * (`'Zbaliť priečinok'`, `'Rozbaliť priečinok'`, `'<n> dokumentov'`). Those
 * strings bypass react-i18next, so the toast text never matched the manager's
 * chosen language and the screen-reader labels were Slovak for everyone (see
 * code-review-ppt-web-ui-documents-toast-hardcoded-mixed-locale; sibling of
 * code-review-ppt-web-core-route-group-toast-i18n).
 *
 * This locks the fix two ways, exactly like the route-group toast guard:
 *
 *  1. Key presence — every toast/aria key the component now resolves must
 *     exist, and be a non-empty string, in all six shipped locale bundles. A
 *     key absent from a non-en bundle degrades silently via `fallbackLng`, so
 *     only an explicit assertion catches it.
 *  2. Source guard — `FolderTree.tsx` may neither assign a bare string literal
 *     to a toast `title:` / `message:` field nor embed the old hardcoded
 *     Slovak aria-label copy; all of it must be driven by `t(...)`.
 *
 * Both checks fail on the pre-fix tree (hardcoded literals present, the
 * `documentCount` key missing) and pass once every string goes through a
 * translation key defined in all six locales.
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
 * Keys the folder tree resolves for its toasts and notification aria-labels.
 * The `documentCount` pluralised form is new to `documents.folder`; the rest
 * already existed and are asserted here so a future drop is caught.
 */
const REQUIRED_KEYS: string[] = [
  // expand / collapse toggle aria-labels
  'documents.folder.collapse',
  'documents.folder.expand',
  // document-count badge aria-label (pluralised — one + other are the CLDR
  // categories every locale defines; few/many are locale-specific extras)
  'documents.folder.documentCount_one',
  'documents.folder.documentCount_other',
  // create / rename / delete error toasts
  'documents.folder.renameError',
  'documents.folder.renameErrorMessage',
  'documents.folder.deleteError',
  'documents.folder.deleteErrorMessage',
  'documents.folder.createError',
  'documents.folder.createErrorMessage',
];

/**
 * Resolve the folder-tree component path from the test's working directory.
 * Vitest runs with cwd at the package dir; walk upward until the component is
 * found so the guard is robust to being launched from the workspace root.
 */
function folderTreeSourcePath(): string {
  const rel = 'src/features/documents/components/FolderTree.tsx';
  let dir = process.cwd();
  for (let i = 0; i < 6; i++) {
    const candidate = resolve(dir, rel);
    if (existsSync(candidate)) return candidate;
    const apps = resolve(dir, `apps/ppt-web/${rel}`);
    if (existsSync(apps)) return apps;
    const parent = dirname(dir);
    if (parent === dir) break;
    dir = parent;
  }
  throw new Error(`could not locate ${rel} from cwd ${process.cwd()}`);
}

/** Old hardcoded Slovak aria-label copy that must no longer appear in source. */
const FORBIDDEN_LITERALS = ['Zbaliť priečinok', 'Rozbaliť priečinok', 'dokumentov'];

describe('documents folder-tree toast i18n', () => {
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

  describe('source guard: FolderTree.tsx must not hardcode notification copy', () => {
    const source = readFileSync(folderTreeSourcePath(), 'utf8');

    it('assigns no bare string literal to a toast title/message', () => {
      // Direct literal assignment of human copy, e.g. `title: 'Created'`. A
      // translated field reads `title: t(...)`. The pattern requires at least
      // one letter inside the quotes so an empty `message: ''` is allowed.
      const matches = source.match(/\b(?:title|message):\s*['"][^'"]*[A-Za-z]/g) ?? [];
      expect(
        matches,
        `FolderTree.tsx still hardcodes a toast title/message literal (${matches.length} occurrence(s)) — route to t(...)`
      ).toEqual([]);
    });

    for (const literal of FORBIDDEN_LITERALS) {
      it(`no longer embeds the hardcoded Slovak aria-label "${literal}"`, () => {
        expect(
          source.includes(literal),
          `FolderTree.tsx still contains the hardcoded aria-label copy "${literal}" — route to t(...)`
        ).toBe(false);
      });
    }
  });
});
