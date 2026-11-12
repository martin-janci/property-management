/// <reference types="vitest/globals" />
/**
 * Community forms i18n regression guard.
 *
 * The community "create" forms — the feed post composer (`CreatePostForm.tsx`)
 * and the group / event / marketplace creation pages (`CreateGroupPage.tsx`,
 * `CreateEventPage.tsx`, `CreateItemPage.tsx`) — are the most literal-heavy
 * surfaces in `src/features/community/**`: every field has a label, a
 * placeholder, inline validation copy and a submit button. A code-review
 * finding flagged these forms as rendering hardcoded English, which bypasses
 * react-i18next entirely (a manager on sk/cs/de/pl/hu would see English labels
 * regardless of their chosen language). The forms now drive all human-readable
 * copy through `t('community.*')`; this test locks that in so a future edit
 * cannot silently reintroduce a bare literal (which typecheck + lint can't see,
 * because a hardcoded JSX string is valid TSX).
 *
 * It guards the forms three ways:
 *
 *  1. Source guard — each form module must call `useTranslation()`, and must
 *     render no bare JSX text node or quoted `placeholder` / `aria-label`
 *     literal that contains letters. Non-translatable placeholders (a URL
 *     scheme like `https://`, a numeric hint like `0.00`) are allowed.
 *  2. Key presence — every `community.*` form key the forms consume must exist,
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

/** Form keys the four community create forms consume. */
const REQUIRED_FORM_KEYS: string[] = [
  // CreatePostForm.tsx
  'community.createPost.contentPlaceholder',
  'community.createPost.posting',
  'community.createPost.submit',
  'community.hub.postTo',
  'community.hub.postType',
  'community.hub.postTitlePlaceholder',
  'community.hub.postAnonymously',
  // CreateGroupPage.tsx
  'community.groupForm.nameLabel',
  'community.groupForm.namePlaceholder',
  'community.groupForm.descriptionLabel',
  'community.groupForm.descriptionPlaceholder',
  'community.groupForm.visibilityLegend',
  'community.groupForm.errors.nameRequired',
  'community.groupForm.errors.nameMin',
  'community.groupForm.errors.nameMax',
  'community.groupForm.errors.descriptionMax',
  'community.groupForm.creating',
  'community.groupForm.create',
  // CreateEventPage.tsx
  'community.eventForm.titleLabel',
  'community.eventForm.titlePlaceholder',
  'community.eventForm.descriptionLabel',
  'community.eventForm.descriptionPlaceholder',
  'community.eventForm.locationLabel',
  'community.eventForm.locationDetailsLabel',
  'community.eventForm.dateTime',
  'community.eventForm.allDay',
  'community.eventForm.start',
  'community.eventForm.end',
  'community.eventForm.maxAttendeesLabel',
  'community.eventForm.errors.titleRequired',
  'community.eventForm.errors.titleMin',
  'community.eventForm.errors.startRequired',
  'community.eventForm.errors.endRequired',
  'community.eventForm.errors.endAfterStart',
  'community.eventForm.errors.invalidUrl',
  'community.eventForm.errors.invalidNumber',
  'community.eventForm.creating',
  'community.eventForm.create',
  // CreateItemPage.tsx
  'community.itemForm.titleLabel',
  'community.itemForm.titlePlaceholder',
  'community.itemForm.descriptionLabel',
  'community.itemForm.descriptionPlaceholder',
  'community.itemForm.priceLabel',
  'community.itemForm.errors.titleRequired',
  'community.itemForm.errors.titleMin',
  'community.itemForm.errors.invalidPrice',
  'community.itemForm.creating',
  'community.itemForm.create',
  // shared cancel action used by every page form
  'community.common.cancel',
];

/**
 * Resolve the community feature dir from the test's working directory. Vitest
 * runs with cwd at the package dir; walk upward until the feature folder is
 * found so the guard is robust to being launched from the workspace root.
 */
function communityDir(): string {
  let dir = process.cwd();
  for (let i = 0; i < 6; i++) {
    const candidate = resolve(dir, 'src/features/community');
    if (existsSync(candidate)) return candidate;
    const apps = resolve(dir, 'apps/ppt-web/src/features/community');
    if (existsSync(apps)) return apps;
    const parent = dirname(dir);
    if (parent === dir) break;
    dir = parent;
  }
  throw new Error(`could not locate src/features/community from cwd ${process.cwd()}`);
}

/** The four community create-form modules, relative to the feature dir. */
const GUARDED_FORMS = [
  'components/CreatePostForm.tsx',
  'pages/CreateGroupPage.tsx',
  'pages/CreateEventPage.tsx',
  'pages/CreateItemPage.tsx',
];

/**
 * A quoted attribute literal is allowed only when it carries no translatable
 * copy: a URL scheme hint (`https://`) or a numeric/punctuation hint (`0.00`).
 */
function isNonTranslatablePlaceholder(value: string): boolean {
  return /^https?:\/\//.test(value) || /^[\d.,\s%+-]*$/.test(value);
}

describe('community forms i18n', () => {
  it('covers exactly the six shipped locale bundles', () => {
    expect(Object.keys(BUNDLES).sort()).toEqual(['cs', 'de', 'en', 'hu', 'pl', 'sk']);
  });

  describe('source guard: forms must drive copy through t(...)', () => {
    for (const form of GUARDED_FORMS) {
      const read = () => readFileSync(resolve(communityDir(), form), 'utf8');

      it(`${form} calls useTranslation`, () => {
        expect(read(), `${form} must use react-i18next`).toContain('useTranslation');
      });

      it(`${form} renders no hardcoded JSX text node`, () => {
        const source = read();
        // A JSX text node is prose sitting directly between `>` and `<`, e.g.
        // `>Create Group<`. Translated copy reads `>{t('…')}<`, so a hardcoded
        // label shows up as a run that starts with a letter and contains only
        // prose characters. The character class deliberately excludes code
        // punctuation (`:;=(){}/` + backtick) and newlines, so it never spans a
        // ternary / arrow body that merely happens to live between a `>` and a
        // later `<` (e.g. `{errors[key]}</p> : null` — the first char after the
        // `>` is `:`, not a letter, so it never matches). Leading whitespace may
        // span newlines so own-line labels (`>\n  Cancel\n</button>`) are caught,
        // but the text body itself is single-line. The `(?<!=)` lookbehind drops
        // arrow / generic type spans such as `=> Promise<unknown>`.
        const matches = source.match(/(?<!=)>\s*[A-Za-z][A-Za-z0-9 .,'"&%*!?@+#-]*\s*</g) ?? [];
        expect(
          matches,
          `${form} still renders hardcoded JSX text (${matches.length}): ${matches.join(' | ')}`
        ).toEqual([]);
      });

      it(`${form} hardcodes no placeholder/aria-label literal`, () => {
        const source = read();
        // Quoted `placeholder="…"` / `aria-label="…"` attribute values. A
        // translated attribute uses braces (`placeholder={t('…')}`), so a
        // quoted literal with letters is a hardcoded string — unless it's a
        // non-translatable URL/number hint.
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
