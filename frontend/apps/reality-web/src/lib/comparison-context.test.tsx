/**
 * Regression tests for localStorage rehydration in ComparisonProvider
 * (reality-web).
 *
 * Bug (`code-review-reality-web-comparison-parse-shape`): the mount effect did
 * `JSON.parse(localStorage)` inside a try/catch and fed the result straight
 * into state. `JSON.parse` only rejects malformed JSON — a tampered or corrupt
 * stored value that is still valid JSON (`{}`, `"x"`, `42`, `null`, `[{}]`)
 * parsed fine, became the `listings` state, and then the first consumer that
 * ran an array op (`isInComparison` → `listings.some(...)`) threw a TypeError
 * and crashed the whole provider subtree.
 *
 * These render the REAL `ComparisonProvider` with various tampered values
 * seeded into localStorage and assert the provider rehydrates gracefully
 * (usable subset, or empty) instead of throwing.
 */
/// <reference types="vitest/globals" />

import type { ListingSummary } from '@ppt/reality-api-client';
import { render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { ComparisonProvider, useComparison } from './comparison-context';

const STORAGE_KEY = 'ppt-comparison';

function makeListing(id: string): ListingSummary {
  // Only `id` is exercised by the context; the rest is filler to satisfy the type.
  return { id, title: `Listing ${id}`, slug: id } as unknown as ListingSummary;
}

function Probe() {
  const { listings, isInComparison } = useComparison();
  return (
    <div>
      <span data-testid="count">{listings.length}</span>
      {/* Exercises `listings.some(...)` — the op that threw before the fix. */}
      <span data-testid="has">{isInComparison('x') ? 'yes' : 'no'}</span>
    </div>
  );
}

beforeEach(() => {
  window.localStorage.clear();
});

afterEach(() => {
  window.localStorage.clear();
});

describe('ComparisonProvider — tampered localStorage rehydration', () => {
  // Valid JSON that is NOT an array of listings — the shapes that slipped past
  // the old try/catch and crashed the provider.
  it.each([
    ['an object', '{"foo":"bar"}'],
    ['a string', '"not-an-array"'],
    ['a number', '42'],
    ['null', 'null'],
    ['a boolean', 'true'],
  ])('rehydrates to empty and does not throw when the stored value is %s', async (_label, raw) => {
    window.localStorage.setItem(STORAGE_KEY, raw);

    render(
      <ComparisonProvider>
        <Probe />
      </ComparisonProvider>
    );

    await waitFor(() => expect(screen.getByTestId('has').textContent).toBe('no'));
    expect(screen.getByTestId('count').textContent).toBe('0');
    // The unusable value is dropped.
    expect(window.localStorage.getItem(STORAGE_KEY)).toBeNull();
  });

  it('drops entries without a string id but keeps the valid ones', async () => {
    const mixed = JSON.stringify([makeListing('a'), {}, { id: 123 }, null, makeListing('b')]);
    window.localStorage.setItem(STORAGE_KEY, mixed);

    render(
      <ComparisonProvider>
        <Probe />
      </ComparisonProvider>
    );

    await waitFor(() => expect(screen.getByTestId('count').textContent).toBe('2'));
    expect(screen.getByTestId('has').textContent).toBe('no');
  });

  it('rehydrates a well-formed array unchanged', async () => {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify([makeListing('x'), makeListing('y')]));

    render(
      <ComparisonProvider>
        <Probe />
      </ComparisonProvider>
    );

    await waitFor(() => expect(screen.getByTestId('count').textContent).toBe('2'));
    // `isInComparison('x')` now finds the seeded listing.
    expect(screen.getByTestId('has').textContent).toBe('yes');
  });

  it('clears malformed (non-JSON) stored data without throwing', async () => {
    window.localStorage.setItem(STORAGE_KEY, '{not valid json');

    render(
      <ComparisonProvider>
        <Probe />
      </ComparisonProvider>
    );

    await waitFor(() => expect(screen.getByTestId('count').textContent).toBe('0'));
    expect(window.localStorage.getItem(STORAGE_KEY)).toBeNull();
  });
});
