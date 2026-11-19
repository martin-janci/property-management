/**
 * Regression tests for share-URL id encoding in ComparisonProvider
 * (reality-web).
 *
 * Bug (`code-review-reality-web-comparison-share-id-unencoded`):
 * `generateShareUrl` built `/compare?ids=${ids}` by joining the raw listing
 * ids with a comma and interpolating them straight into the query string
 * without URL-encoding. A listing id carrying a URL-reserved character
 * (`&`, `#`, `?`, `=`, space, …) therefore broke the link or injected extra
 * query parameters — e.g. an id `a&admin=1` turned `?ids=a&admin=1,b` into two
 * separate query params, so the `ids` value the `/compare` page read back was
 * just `a` and the injected `admin=1` rode along.
 *
 * The `/compare` page reads already-decoded `searchParams.ids` and does
 * `.split(',')`, so encoding each id on the producer side round-trips cleanly:
 * `new URL(url).searchParams.get('ids')?.split(',')` must recover the exact
 * original ids.
 */
/// <reference types="vitest/globals" />

import type { ListingSummary } from '@ppt/reality-api-client';
import { act, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { ComparisonProvider, useComparison } from './comparison-context';

const STORAGE_KEY = 'ppt-comparison';

function makeListing(id: string): ListingSummary {
  // Only `id` is exercised by the context; the rest is filler to satisfy the type.
  return { id, title: `Listing ${id}`, slug: id } as unknown as ListingSummary;
}

/**
 * Mirrors how the `/compare` page recovers the shared ids: Next.js decodes the
 * query value once (modelled here by `URL`/`URLSearchParams`), then the page
 * splits on the literal comma delimiter.
 */
function recoverIds(url: string): string[] {
  const idsParam = new URL(url, 'http://localhost').searchParams.get('ids');
  return idsParam ? idsParam.split(',').filter(Boolean) : [];
}

let shareFn: () => string;
let addFn: (listing: ListingSummary) => boolean;

function Probe() {
  const { generateShareUrl, addToComparison } = useComparison();
  shareFn = generateShareUrl;
  addFn = addToComparison;
  return <span data-testid="ready">ready</span>;
}

beforeEach(() => {
  window.localStorage.clear();
});

afterEach(() => {
  window.localStorage.clear();
});

describe('ComparisonProvider — share-URL id encoding', () => {
  it.each([
    ['ampersand / query injection', 'a&admin=1', 'plain'],
    ['hash fragment', 'a#frag', 'plain'],
    ['space', 'a b', 'plain'],
    ['equals + question mark', 'a=1?x', 'plain'],
  ])('round-trips an id containing %s through the shared URL', (_label, dirtyId, plainId) => {
    render(
      <ComparisonProvider>
        <Probe />
      </ComparisonProvider>
    );
    expect(screen.getByTestId('ready')).toBeTruthy();

    act(() => {
      addFn(makeListing(dirtyId));
      addFn(makeListing(plainId));
    });

    let url = '';
    act(() => {
      url = shareFn();
    });

    // The reserved characters must be percent-encoded, not left to break the
    // query string or inject params.
    expect(url).not.toContain(dirtyId);
    // The `/compare` page must recover the exact original ids.
    expect(recoverIds(url)).toEqual([dirtyId, plainId]);
  });

  it('encodes a clean comparison without altering simple ids', () => {
    render(
      <ComparisonProvider>
        <Probe />
      </ComparisonProvider>
    );

    act(() => {
      addFn(makeListing('abc-123'));
      addFn(makeListing('def-456'));
    });

    let url = '';
    act(() => {
      url = shareFn();
    });

    expect(url).toContain('ids=abc-123,def-456');
    expect(recoverIds(url)).toEqual(['abc-123', 'def-456']);
  });
});
