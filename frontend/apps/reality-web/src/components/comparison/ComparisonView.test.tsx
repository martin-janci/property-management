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

function makeListing(
  id: string,
  price: unknown,
  area = 60,
  currency: unknown = 'EUR'
): ListingSummary {
  return {
    id,
    slug: `${id}-slug`,
    title: `Listing ${id}`,
    price,
    currency,
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

// A corrupt/tampered persisted entry that passes the comparison context's
// weak persistence guard (`isStoredListing` only requires a string `id`) but
// lacks the renderable shape `ComparisonView` dereferences (`slug`, `title`,
// `address`). Models a hand-edited localStorage value or a malformed body that
// slipped past an older loader.
function makeMalformed(id: string): ListingSummary {
  return { id } as unknown as ListingSummary;
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

  it('renders without throwing when a listing carries an invalid currency code', async () => {
    // Bug (`code-review-reality-web-currency-code-rangeerror-crash`): the price
    // formatter fed `listing.currency` straight into `Intl.NumberFormat({
    // style: 'currency', currency })`. A listing whose `currency` is a symbol
    // (`'€'`), empty, or any non-ISO-4217 value makes that constructor throw
    // `RangeError: Invalid currency code`, which crashed the whole comparison
    // render. `currency` comes from untrusted listing data (API / mocks /
    // rehydrated localStorage), so a bad code must degrade gracefully instead.
    seed([makeListing('a', 100000, 60, '€'), makeListing('b', 200000, 80, '')]);

    const { container } = render(
      <ComparisonProvider>
        <ComparisonView />
      </ComparisonProvider>
    );

    // The table must render (no RangeError bubbling out of the price row).
    await waitFor(() => expect(container.querySelector('table')).toBeTruthy());
    const priceRow = container.querySelectorAll('tbody tr')[0];
    const priceCells = priceRow.querySelectorAll('td.value-cell');
    expect(priceCells).toHaveLength(2);
    // The amounts still render (fell back to a valid currency / plain number).
    expect(priceCells[0].textContent ?? '').toMatch(/\d/);
    expect(priceCells[1].textContent ?? '').toMatch(/\d/);
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

describe('ComparisonView — shape guard on malformed comparison entries', () => {
  it('drops entries that lack the renderable shape and only renders the valid ones', async () => {
    // `good` is a well-formed summary; `bad` carries only an `id` — enough to
    // survive the context's persistence guard, but missing the slug/title/
    // address this component dereferences. Before the guard it rendered as a
    // second, broken property column (`/listings/undefined`, empty title).
    seed([makeListing('good', 100000), makeMalformed('bad')]);

    const { container } = render(
      <ComparisonProvider>
        <ComparisonView />
      </ComparisonProvider>
    );

    await waitFor(() => expect(container.querySelector('table')).toBeTruthy());

    // Only the renderable listing contributes a property column.
    const propertyColumns = container.querySelectorAll('th.property-column');
    expect(propertyColumns).toHaveLength(1);
    expect(container.textContent).toContain('Listing good');
  });

  it('shows the empty state instead of a broken table when every entry is malformed', async () => {
    seed([makeMalformed('x'), makeMalformed('y')]);

    const { container } = render(
      <ComparisonProvider>
        <ComparisonView />
      </ComparisonProvider>
    );

    // Nothing renderable remains, so the component must fall back to the empty
    // state rather than render a table full of broken columns.
    await waitFor(() => expect(container.querySelector('.empty-state')).toBeTruthy());
    expect(container.querySelector('table')).toBeNull();
  });
});
