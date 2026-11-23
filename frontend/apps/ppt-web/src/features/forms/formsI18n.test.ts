/// <reference types="vitest/globals" />
/**
 * Forms feature i18n regression guard.
 *
 * The Epic 54 "Forms Management" surfaces — the manager list/create/view pages
 * (`FormsPage`/`FormList`/`FormCard`/`FormBuilder`/`CreateFormPage`/
 * `ViewFormPage`/`SubmissionList`) and the resident-facing fill flow
 * (`AvailableFormsPage`/`FillFormPage`/`FormRenderer`) — are among the most
 * literal-heavy screens in `src/features/forms/**`: every field label, filter
 * option, status badge, table header, empty state, validation message and
 * button carried hardcoded English. That bypassed react-i18next entirely, so a
 * manager or resident on sk/cs/de/pl/hu saw English regardless of their chosen
 * language. The feature now drives all human-readable copy through
 * `t('forms.*')` (with generic words reusing `common.*`); this test locks that
 * in so a future edit cannot silently reintroduce a bare literal — which
 * typecheck + lint cannot see, because a hardcoded JSX string is valid TSX.
 *
 * It guards the feature three ways:
 *
 *  1. Source guard — each guarded module must call `useTranslation()`, and must
 *     render no bare JSX text node nor quoted `placeholder` / `aria-label`
 *     literal that contains letters. Non-translatable attribute values (a URL
 *     scheme like `https://`, a numeric hint like `0.00`) are allowed.
 *  2. Key presence — every `forms.*` key the modules consume (collected from
 *     source, plus the dynamically-built status/field-type keys) must exist,
 *     and be a non-empty string, in all six shipped locale bundles. A key
 *     absent from a non-en bundle degrades silently (react-i18next falls back
 *     to en.json via `fallbackLng`), so only an explicit assertion catches it.
 *
 * Both checks fail on a tree where any form carries hardcoded copy or is
 * missing a locale, and pass once every form string goes through a translation
 * key defined in all six locales.
 */

import { existsSync, readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import cs from '../../../messages/cs.json';
import de from '../../../messages/de.json';
import en from '../../../messages/en.json';
import hu from '../../../messages/hu.json';
import pl from '../../../messages/pl.json';
import sk from '../../../messages/sk.json';

type Json = Record<string, unknown>;

const BUNDLES: Record<string, Json> = { en, sk, cs, de, pl, hu };

function getPath(obj: unknown, path: string): unknown {
  return path.split('.').reduce<unknown>((acc, part) => {
    if (acc === null || typeof acc !== 'object') return undefined;
    return (acc as Json)[part];
  }, obj);
}

/**
 * Resolve the forms feature dir from the test's working directory. Vitest runs
 * with cwd at the package dir; walk upward until the feature folder is found so
 * the guard is robust to being launched from the workspace root.
 */
function formsDir(): string {
  let dir = process.cwd();
  for (let i = 0; i < 6; i++) {
    const candidate = resolve(dir, 'src/features/forms');
    if (existsSync(candidate)) return candidate;
    const apps = resolve(dir, 'apps/ppt-web/src/features/forms');
    if (existsSync(apps)) return apps;
    const parent = dirname(dir);
    if (parent === dir) break;
    dir = parent;
  }
  throw new Error(`could not locate src/features/forms from cwd ${process.cwd()}`);
}

/** The modules that render human-readable copy (FormsPage is a pure wrapper). */
const GUARDED_FORMS = [
  'components/FormList.tsx',
  'components/FormCard.tsx',
  'components/FormRenderer.tsx',
  'components/SubmissionList.tsx',
  'components/FormBuilder.tsx',
  'pages/AvailableFormsPage.tsx',
  'pages/CreateFormPage.tsx',
  'pages/ViewFormPage.tsx',
  'pages/FillFormPage.tsx',
];

/**
 * Status / field-type keys built dynamically in source via template literals
 * (`t(`forms.status.${status}`)` etc.), so the static scan below cannot see
 * them. Their concrete suffixes are the finite domains of the backing enums.
 */
const DYNAMIC_KEYS: string[] = [
  ...['draft', 'published', 'archived'].map((s) => `forms.status.${s}`),
  ...['pending', 'reviewed', 'approved', 'rejected'].map((s) => `forms.submissions.status.${s}`),
  ...[
    'text',
    'textarea',
    'number',
    'email',
    'phone',
    'date',
    'datetime',
    'checkbox',
    'radio',
    'select',
    'multiselect',
    'file',
    'signature',
  ].map((t) => `forms.fieldTypes.${t}`),
];

/** Every static `t('forms.*')` key referenced across the guarded modules. */
function collectStaticFormKeys(): string[] {
  const keys = new Set<string>();
  const re = /['"`](forms\.[A-Za-z0-9_.]+)['"`]/g;
  for (const form of GUARDED_FORMS) {
    const source = readFileSync(resolve(formsDir(), form), 'utf8');
    for (const m of source.matchAll(re)) keys.add(m[1]);
  }
  return [...keys].sort();
}

const REQUIRED_FORM_KEYS = [...new Set([...collectStaticFormKeys(), ...DYNAMIC_KEYS])].sort();

/**
 * A quoted attribute literal is allowed only when it carries no translatable
 * copy: a URL scheme hint (`https://`) or a numeric/punctuation hint (`0.00`).
 */
function isNonTranslatablePlaceholder(value: string): boolean {
  return /^https?:\/\//.test(value) || /^[\d.,\s%+-]*$/.test(value);
}

describe('forms feature i18n', () => {
  it('covers exactly the six shipped locale bundles', () => {
    expect(Object.keys(BUNDLES).sort()).toEqual(['cs', 'de', 'en', 'hu', 'pl', 'sk']);
  });

  it('collected a non-trivial set of forms.* keys from source', () => {
    expect(REQUIRED_FORM_KEYS.length).toBeGreaterThan(40);
  });

  describe('source guard: forms must drive copy through t(...)', () => {
    for (const form of GUARDED_FORMS) {
      const read = () => readFileSync(resolve(formsDir(), form), 'utf8');

      it(`${form} calls useTranslation`, () => {
        expect(read(), `${form} must use react-i18next`).toContain('useTranslation');
      });

      it(`${form} renders no hardcoded JSX text node`, () => {
        const source = read();
        // A JSX text node is prose sitting directly between `>` and `<`, e.g.
        // `>Create Form<`. Translated copy reads `>{t('…')}<`, so a hardcoded
        // label shows up as a run that starts with a letter and contains only
        // prose characters. The character class deliberately excludes code
        // punctuation (`:;=(){}/` + backtick) so it never spans a ternary /
        // arrow body between a `>` and a later `<`. The `(?<!=)` lookbehind
        // drops arrow / generic type spans such as `=> Promise<string>`.
        const matches = source.match(/(?<!=)>\s*[A-Za-z][A-Za-z0-9 .,'"&%*!?@+#-]*\s*</g) ?? [];
        expect(
          matches,
          `${form} still renders hardcoded JSX text (${matches.length}): ${matches.join(' | ')}`
        ).toEqual([]);
      });

      it(`${form} hardcodes no placeholder/aria-label literal`, () => {
        const source = read();
        const attrLiterals = [...source.matchAll(/(?:placeholder|aria-label)=["']([^"']*)["']/g)]
          .map((m) => m[1])
          .filter((v) => /[A-Za-z]/.test(v) && !isNonTranslatablePlaceholder(v));
        expect(
          attrLiterals,
          `${form} hardcodes a placeholder/aria-label literal: ${attrLiterals.join(' | ')}`
        ).toEqual([]);
      });
    }
  });

  for (const [locale, bundle] of Object.entries(BUNDLES)) {
    describe(`locale: ${locale}`, () => {
      for (const path of REQUIRED_FORM_KEYS) {
        it(`defines a non-empty ${path}`, () => {
          const value = getPath(bundle, path);
          expect(value, `${path} missing in ${locale}.json`).toBeTypeOf('string');
          expect((value as string).trim().length).toBeGreaterThan(0);
        });
      }
    });
  }
});
