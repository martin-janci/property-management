/**
 * AgencyBranding — preview price locale formatting (regression).
 *
 * Bug: the branding preview card hardcoded the sample price as the literal
 * string `€450,000`, which is wrong for the non-`en` locales we ship
 * (sk/cs/de group with spaces and place the € after the amount; hu/pl differ
 * too). The price must be rendered through the shared locale-aware
 * `formatPrice` helper so it formats correctly for the active locale.
 *
 * These tests drive the active locale via the mocked `useLocale` and assert
 * the `.preview-price` text equals `formatPrice(450000, locale)` and is NOT
 * the old hardcoded literal. Scoped to the price only (the sibling alt-text
 * fix lives elsewhere).
 */

import { render } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { formatPrice } from '@/lib/format';

// The sample price shown in the preview card.
const SAMPLE_PRICE = 450000;

// Mutable locale so each test can select which locale the component sees.
// This file-level mock overrides the global next-intl mock from
// src/test/setup.tsx for this test module.
let currentLocale = 'en';
vi.mock('next-intl', () => ({
  useTranslations: () => (key: string) => key,
  useLocale: () => currentLocale,
  useMessages: () => ({}),
}));

vi.mock('@ppt/reality-api-client', () => ({
  useMyAgency: () => ({
    data: { id: 'agency-1', name: 'Test Agency' },
    isLoading: false,
    isError: false,
    error: null,
    refetch: vi.fn(),
  }),
  useAgencyBranding: () => ({
    data: { primaryColor: '#111111', secondaryColor: '#222222', accentColor: '#333333' },
    isLoading: false,
    isError: false,
    error: null,
    refetch: vi.fn(),
  }),
  useUpdateBranding: () => ({ mutateAsync: vi.fn(), isPending: false }),
}));

import { AgencyBranding } from './AgencyBranding';

describe('AgencyBranding — preview price is locale-aware (not hardcoded)', () => {
  it('formats the sample price for the sk locale (not the hardcoded literal)', () => {
    currentLocale = 'sk';
    const { container } = render(<AgencyBranding />);

    const priceEl = container.querySelector('.preview-price');
    expect(priceEl?.textContent).toBe(formatPrice(SAMPLE_PRICE, 'sk'));
    // Regression: must no longer render the en-only hardcoded string.
    expect(priceEl?.textContent).not.toBe('€450,000');
  });

  it.each(['en', 'cs', 'de', 'hu', 'pl'])(
    'formats the sample price for the %s locale',
    (locale) => {
      currentLocale = locale;
      const { container } = render(<AgencyBranding />);

      const priceEl = container.querySelector('.preview-price');
      expect(priceEl?.textContent).toBe(formatPrice(SAMPLE_PRICE, locale));
    }
  );
});
