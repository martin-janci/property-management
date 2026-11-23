import { describe, expect, it } from 'vitest';
import cs from '../../messages/cs.json';
import de from '../../messages/de.json';
import en from '../../messages/en.json';
import hu from '../../messages/hu.json';
import pl from '../../messages/pl.json';
import sk from '../../messages/sk.json';
import { locales } from './config';

// Regression guard for the account listing-edit wizard (UC-51.5) i18n
// extraction. The multi-step edit wizard
// (`/[locale]/account/listings/[id]/edit`) renders all of its copy — step
// titles, field labels, transaction/property/status options, summary and
// photo controls — via `useTranslations('pages.listingEdit')`. Before
// extraction these were hardcoded Slovak literals, so cs/de/en/pl/hu visitors
// always saw Slovak. `page.i18n.test.tsx` pins that the component reads keys
// (not literals); this test pins the complementary half: every supported
// locale actually defines the whole `pages.listingEdit` key set as non-empty
// strings, with identical key sets across locales. If a locale drops a key,
// next-intl leaks the raw key (or falls back) to that locale's users.

const messages: Record<string, Record<string, unknown>> = { cs, de, en, hu, pl, sk };

/** Flatten a nested message subtree into dotted leaf keys → string values. */
function flattenStrings(
  node: unknown,
  prefix = '',
  out: Record<string, unknown> = {}
): Record<string, unknown> {
  if (node && typeof node === 'object' && !Array.isArray(node)) {
    for (const [key, value] of Object.entries(node as Record<string, unknown>)) {
      flattenStrings(value, prefix ? `${prefix}.${key}` : key, out);
    }
  } else {
    out[prefix] = node;
  }
  return out;
}

function listingEditNode(locale: string): Record<string, unknown> {
  const pages = messages[locale].pages as Record<string, unknown> | undefined;
  return flattenStrings(pages?.listingEdit ?? {});
}

describe('account listing-edit wizard i18n coverage', () => {
  it('every supported locale has a message file loaded', () => {
    for (const locale of locales) {
      expect(messages[locale], `missing messages for locale "${locale}"`).toBeDefined();
    }
  });

  // The Slovak file is the authoring reference; its key set is the contract.
  const referenceKeys = Object.keys(listingEditNode('sk')).sort();

  it('defines a non-empty pages.listingEdit namespace', () => {
    expect(referenceKeys.length).toBeGreaterThan(0);
  });

  for (const locale of locales) {
    describe(`locale: ${locale}`, () => {
      it('defines every pages.listingEdit key as a non-empty string', () => {
        const node = listingEditNode(locale);
        for (const key of referenceKeys) {
          expect(typeof node[key], `pages.listingEdit.${key} missing in ${locale}`).toBe('string');
          expect(
            (node[key] as string).length,
            `pages.listingEdit.${key} empty in ${locale}`
          ).toBeGreaterThan(0);
        }
      });

      it('exposes the exact same key set as the reference (parity)', () => {
        const actual = Object.keys(listingEditNode(locale)).sort();
        expect(actual, `pages.listingEdit key set drift in ${locale}`).toEqual(referenceKeys);
      });
    });
  }

  it('keeps interpolation placeholders in every locale', () => {
    for (const locale of locales) {
      const node = listingEditNode(locale);
      // `successBody`/`breadcrumb` interpolate {id}; `stepLabel` interpolates
      // {step}/{total}/{stepTitle}. A locale that drops a placeholder breaks the
      // rendered sentence, so pin the load-bearing ones.
      expect(node.successBody as string, `successBody {id} in ${locale}`).toMatch(/\{id\}/);
      expect(node.stepLabel as string, `stepLabel {step} in ${locale}`).toMatch(/\{step\}/);
    }
  });
});
