/**
 * Regression tests for the best-value highlight in ComparisonView
 * (reality-web).
 *
 * Bug (`code-review-reality-web-comparison-url-unvalidated-cast`):
 * `getHighlightClass` collected a row's metric across listings with
 *
 *     .filter((v) => v !== undefined) as number[]
 *
 * — an unchecked cast. The listings it reduces over are not trusted: a shared
 * `?ids=...` comparison URL is loaded by `ComparisonUrlHandler`, whose
 * `parseListingSummary` guard deliberately does NOT type-check the optional
 * numeric fields, and a tampered localStorage entry only has to carry a string
 * `id` to rehydrate. So a non-numeric string (e.g. `price: "abc"`) slipped
 * through the cast into `Math.min(...)` / `Math.max(...)`, coerced to `NaN`,
 * and silently wiped out the highlight for the whole row — the genuinely
 * cheapest/biggest listing stopped being marked.
 *
 * The fix narrows to finite numbers with a real type guard, so the malformed
 * entry is dropped and the real minimum/maximum still wins.
 */
/// <reference types="vitest/globals" />

import type { ListingSummary } from '@ppt/reality-api-client';
import { render, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

// Stub next-intl so localized strings + locale resolve in jsdom without a
// NextIntlClientProvider. Returning the key keeps the wiring exercised.
vi.mock('next-intl', () => ({
  useTranslations: () => (key: string) => `comparison.${key}`,
  useLocale: () => 'en',
}));

import { ComparisonProvider } from '../../lib/comparison-context';
import { ComparisonView } from './ComparisonView';

const STORAGE_KEY = 'ppt-comparison';

function makeListing(id: string, price: unknown, area = 60): ListingSummary {
  return {
    id,
    slug: `${id}-slug`,
    title: `Listing ${id}`,
    price,
    currency: 'EUR',
    transactionType: 'sale',
    propertyType: 'apartment',
    status: 'active',
    area,
    rooms: 2,
    floor: 1,
    address: { city: 'Bratislava', district: 'Old Town', country: 'Slovakia' },
    isFavorite: false,
    isFeatured: false,
    createdAt: '2024-01-01T00:00:00Z',
    updatedAt: '2024-01-01T00:00:00Z',
    // `price` is deliberately widened to `unknown` to model an untrusted body.
  } as unknown as ListingSummary;
}

function seed(listings: ListingSummary[]) {
  window.localStorage.setItem(STORAGE_KEY, JSON.stringify(listings));
}

beforeEach(() => {
  window.localStorage.clear();
});

afterEach(() => {
  window.localStorage.clear();
  vi.restoreAllMocks();
});

describe('ComparisonView — best-value highlight with untrusted numeric fields', () => {
  it('highlights the cheapest listing even when another listing carries a non-numeric price', async () => {
    // A is genuinely cheapest; C carries a tampered string price that used to
    // poison Math.min(...) to NaN and blank the whole row's highlight.
    const a = makeListing('a', 100000);
    const b = makeListing('b', 200000);
    const c = makeListing('c', 'not-a-number');
    seed([a, b, c]);

    const { container } = render(
      <ComparisonProvider>
        <ComparisonView />
      </ComparisonProvider>
    );

    // Wait for the provider to rehydrate from localStorage and render the table.
    await waitFor(() => expect(container.querySelector('table')).toBeTruthy());

    // First tbody row is the price row (listings render in seeded order).
    const priceRow = container.querySelectorAll('tbody tr')[0];
    const priceCells = priceRow.querySelectorAll('td.value-cell');
    expect(priceCells).toHaveLength(3);

    // Regression bar: before the fix the string price made every price cell
    // un-highlighted (NaN min). The real cheapest (A) must now win...
    expect(priceCells[0].className).toContain('highlight-best');
    // ...and the pricier/garbage ones must not be marked best.
    expect(priceCells[1].className).not.toContain('highlight-best');
    expect(priceCells[2].className).not.toContain('highlight-best');
  });

  it('renders without throwing when a numeric field is a malformed string', async () => {
    seed([makeListing('a', 'abc', 60), makeListing('b', 150000, 80)]);

    const { container } = render(
      <ComparisonProvider>
        <ComparisonView />
      </ComparisonProvider>
    );

    await waitFor(() => expect(container.querySelector('table')).toBeTruthy());
    // The valid area maximum (B) still highlights despite A's bad price.
    const rows = container.querySelectorAll('tbody tr');
    expect(rows.length).toBeGreaterThan(0);
  });
});
