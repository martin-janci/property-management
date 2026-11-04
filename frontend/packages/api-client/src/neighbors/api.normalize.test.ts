/**
 * Neighbor list normalization (round-3 audit #18). The api-server sends
 * `NeighborView` in snake_case; the messaging recipient picker filtered on
 * the camelCase `isVisible`, so it never had anyone to offer.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { clearTokenProvider, setTokenProvider } from '../auth/token-provider';
import { createNeighborsApi, normalizeNeighborsResponse } from './api';

const serverNeighbor = {
  user_id: 'u1',
  display_name: 'Jana Nováková',
  unit_label: 'A-12',
  is_visible: true,
  email: 'jana@example.com',
  phone: null,
  resident_type: 'owner',
};

describe('normalizeNeighborsResponse', () => {
  it('maps snake_case neighbors to the client shape', () => {
    expect(normalizeNeighborsResponse({ neighbors: [serverNeighbor], count: 1, total: 1 })).toEqual(
      {
        neighbors: [
          {
            userId: 'u1',
            displayName: 'Jana Nováková',
            unitLabel: 'A-12',
            isVisible: true,
            email: 'jana@example.com',
            phone: null,
            residentType: 'owner',
          },
        ],
        count: 1,
        total: 1,
      }
    );
  });

  it('accepts camelCase input and missing lists', () => {
    expect(
      normalizeNeighborsResponse({
        neighbors: [{ userId: 'u2', displayName: 'X', isVisible: false }],
      }).neighbors[0]
    ).toMatchObject({ userId: 'u2', displayName: 'X', isVisible: false });
    expect(normalizeNeighborsResponse(undefined)).toEqual({ neighbors: [], count: 0, total: 0 });
  });
});

describe('createNeighborsApi.listNeighbors', () => {
  beforeEach(() => {
    vi.stubGlobal(
      'fetch',
      vi.fn(
        async () =>
          new Response(JSON.stringify({ neighbors: [serverNeighbor], count: 1, total: 1 }), {
            status: 200,
          })
      )
    );
  });

  afterEach(() => {
    clearTokenProvider();
    vi.unstubAllGlobals();
  });

  it('normalizes the response and authenticates with the current token', async () => {
    const api = createNeighborsApi({ baseUrl: '' });
    setTokenProvider(() => 'tok');
    const res = await api.listNeighbors('b1');
    expect(res.neighbors[0].userId).toBe('u1');
    expect(res.neighbors[0].isVisible).toBe(true);
    const init = vi.mocked(fetch).mock.calls[0]?.[1] as RequestInit;
    expect(new Headers(init.headers).get('Authorization')).toBe('Bearer tok');
  });
});
