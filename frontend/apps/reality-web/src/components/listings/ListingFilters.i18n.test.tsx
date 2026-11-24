/**
 * ListingFilters i18n / API-filter-contract tests
 *
 * Locks in the property-type filter contract introduced alongside the
 * property_type i18n helper (PR #3609): the selectable property-type labels
 * are localized for DISPLAY, but the value that flows into the API filter
 * (`filters.propertyType`) must always be the stable enum code
 * (`apartment`, `house`, ...) — never the localized label — in every locale.
 *
 * These assertions fail if a localized label ever leaks into the filter
 * contract (e.g. if the component started passing `t(labelKey)` instead of the
 * enum `value` to `onFiltersChange`).
 */

import type { PropertyType } from '@ppt/reality-api-client';
import { fireEvent, render, screen } from '@testing-library/react';
import { beforeAll, describe, expect, it, vi } from 'vitest';

import csMessages from '../../../messages/cs.json';
import deMessages from '../../../messages/de.json';
import enMessages from '../../../messages/en.json';
import skMessages from '../../../messages/sk.json';

// Mutable locale/messages holder shared with the hoisted next-intl mock below.
const intl = vi.hoisted(() => ({
  locale: 'en' as string,
  // Populated in beforeAll (after the JSON imports have resolved).
  messages: {} as Record<string, Record<string, Record<string, string>>>,
}));

// Override the global next-intl mock (src/test/setup.tsx returns the key
// verbatim). Here we return the *real* localized labels so that a leak of a
// label into the filter value is observable: the label differs from the code.
vi.mock('next-intl', () => ({
  useTranslations: (namespace?: string) => (key: string) => {
    const ns = namespace ? intl.messages[intl.locale]?.[namespace] : undefined;
    return ns?.[key] ?? key;
  },
  useLocale: () => intl.locale,
  useMessages: () => intl.messages[intl.locale] ?? {},
  NextIntlClientProvider: ({ children }: { children: React.ReactNode }) => children,
}));

import { ListingFilters } from './ListingFilters';

beforeAll(() => {
  intl.messages = {
    sk: skMessages as never,
    cs: csMessages as never,
    de: deMessages as never,
    en: enMessages as never,
  };
});

const LOCALES = ['sk', 'cs', 'de', 'en'] as const;

// Stable enum code -> expected API filter value. These are the contract.
const PROPERTY_TYPE_CODES: PropertyType[] = [
  'apartment',
  'house',
  'land',
  'commercial',
  'office',
  'garage',
];

function labelFor(locale: string, code: string): string {
  const label = intl.messages[locale]?.filters?.[code];
  if (!label) throw new Error(`missing filters.${code} label for locale ${locale}`);
  return label;
}

describe('ListingFilters property-type API filter contract', () => {
  for (const locale of LOCALES) {
    describe(`locale=${locale}`, () => {
      it('displays localized labels but filters by the stable enum code', () => {
        intl.locale = locale;

        for (const code of PROPERTY_TYPE_CODES) {
          const onFiltersChange = vi.fn();
          const { unmount } = render(
            <ListingFilters filters={{}} onFiltersChange={onFiltersChange} />
          );

          const localizedLabel = labelFor(locale, code);

          // Display field IS localized.
          const labelNode = screen.getByText(localizedLabel);
          expect(labelNode).toBeInTheDocument();

          // Click the checkbox for this property type.
          const checkbox = labelNode.closest('label')?.querySelector('input[type="checkbox"]');
          expect(checkbox).not.toBeNull();
          fireEvent.click(checkbox as HTMLInputElement);

          // The value handed to the API filter is the stable enum code...
          expect(onFiltersChange).toHaveBeenCalledTimes(1);
          const next = onFiltersChange.mock.calls[0][0];
          expect(next.propertyType).toEqual([code]);

          // ...and is explicitly NOT the localized label (the leak guard).
          expect(next.propertyType).not.toContain(localizedLabel);

          unmount();
        }
      });
    });
  }

  it('keeps enum codes ASCII-stable regardless of locale (no label substitution)', () => {
    // Codes are the same bytes in every locale; localized labels are not. This
    // guards against the whole options table being driven off translations.
    for (const locale of LOCALES) {
      intl.locale = locale;
      const onFiltersChange = vi.fn();
      const { unmount } = render(<ListingFilters filters={{}} onFiltersChange={onFiltersChange} />);

      const apartmentLabel = labelFor(locale, 'apartment');
      fireEvent.click(
        screen
          .getByText(apartmentLabel)
          .closest('label')
          ?.querySelector('input[type="checkbox"]') as HTMLInputElement
      );

      expect(onFiltersChange.mock.calls[0][0].propertyType).toEqual(['apartment']);
      unmount();
    }
  });
});
