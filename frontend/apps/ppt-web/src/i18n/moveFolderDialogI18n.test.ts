/// <reference types="vitest/globals" />
/**
 * MoveFolderDialog i18n regression guard.
 *
 * The move-to-folder dialog (`features/documents/components/MoveFolderDialog.tsx`)
 * renders every user-facing string — the header title, the destination label,
 * the root/"no folder" option, the document-count summary, the tree
 * load-error / empty states, the footer buttons, and the modal + toggle
 * aria-labels — and each one must go through react-i18next rather than a
 * hardcoded literal. A string that bypasses `t(...)` never matches the
 * manager's chosen language, and a key missing from a non-`en` bundle degrades
 * silently via `fallbackLng`, so only an explicit assertion catches it (see
 * code-review-ppt-web-ui-movefolder-i18n; sibling of
 * code-review-ppt-web-ui-documents-toast-hardcoded / folderTreeToastI18n).
 *
 * This locks it two ways, exactly like the folder-tree toast guard:
 *
 *  1. Key presence — every key the dialog resolves must exist, and be a
 *     non-empty string, in all six shipped locale bundles. The pluralised
 *     `documents.move.documentCount` is asserted via its `_one` / `_other`
 *     CLDR categories (every locale defines both; `_few` / `_many` are
 *     locale-specific extras not required here).
 *  2. Source guard — `MoveFolderDialog.tsx` must drive its copy through
 *     `t(...)` (react-i18next imported and used) and must not assign a bare
 *     human-readable string literal to an `aria-label` / `title` /
 *     `placeholder` attribute.
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
 * Every translation key MoveFolderDialog.tsx resolves. Keep this list in sync
 * with the component — a key dropped from the source but left here still
 * passes (the bundle keeps the string), but a key added to the component and
 * missing from a bundle is caught below.
 */
const REQUIRED_KEYS: string[] = [
  // folder-tree option toggle + states (shared documents.folder namespace)
  'documents.folder.collapse',
  'documents.folder.expand',
  'documents.folder.breadcrumbRoot',
  'documents.folder.loadError',
  'documents.folder.empty',
  // modal + navigation aria-labels
  'aria.folderLocation',
  'aria.moveDocuments',
  'aria.close',
  // dialog chrome
  'documents.browse.moveToFolder',
  'documents.move.destination',
  'documents.move.rootNoFolder',
  'documents.move.allDocumentsNoFolder',
  'documents.move.moving',
  'documents.move.moveHere',
  // pluralised document-count summary (one + other are the universal CLDR
  // categories every locale defines; few/many are locale-specific extras)
  'documents.move.documentCount_one',
  'documents.move.documentCount_other',
  // shared button copy
  'common.cancel',
];

/**
 * Resolve the dialog component path from the test's working directory. Vitest
 * runs with cwd at the package dir; walk upward until the component is found so
 * the guard is robust to being launched from the workspace root.
 */
function moveFolderDialogSourcePath(): string {
  const rel = 'src/features/documents/components/MoveFolderDialog.tsx';
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

describe('MoveFolderDialog i18n', () => {
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

  describe('source guard: MoveFolderDialog.tsx must not hardcode user-facing copy', () => {
    const source = readFileSync(moveFolderDialogSourcePath(), 'utf8');

    it('imports and uses react-i18next', () => {
      expect(source, 'MoveFolderDialog.tsx must import useTranslation').toMatch(/useTranslation/);
      const calls = source.match(/\bt\(/g) ?? [];
      expect(
        calls.length,
        'MoveFolderDialog.tsx resolves no translation keys — copy is not going through t(...)'
      ).toBeGreaterThan(0);
    });

    it('assigns no bare string literal to an aria-label / title / placeholder', () => {
      // A translated attribute reads `aria-label={t(...)}`; a hardcoded one
      // reads `aria-label="Move here"`. The pattern requires at least one
      // letter inside the quotes so an empty attribute is allowed.
      const matches = source.match(/\b(?:aria-label|title|placeholder)=["'][^"']*[A-Za-z]/g) ?? [];
      expect(
        matches,
        `MoveFolderDialog.tsx hardcodes an aria-label/title/placeholder literal (${matches.length} occurrence(s)) — route to t(...)`
      ).toEqual([]);
    });
  });
});
