/**
 * Contract tests for the hand-rolled fault API client (story 79-1).
 *
 * `faults/api.ts` is a hand-written transport layer that the ppt-web faults
 * feature calls directly. The generated `FaultsService` (from the backend
 * OpenAPI spec) is the contract source of truth for the endpoints that exist
 * in both. These tests pin the request wiring (URL template + HTTP method) of
 * the hand-rolled client against that contract so the two can't silently drift
 * apart — a drift that would route real fault calls to the wrong path/verb and
 * 404 at runtime with no compile-time signal.
 *
 * Known, intentional divergence: the backend registers `PUT /api/v1/faults/{id}`
 * (see `backend/servers/api-server/src/routes/faults.rs` `router()`), while the
 * *generated* `FaultsService.faultsApiUpdate` emits `PATCH`. The hand-rolled
 * client follows the live backend (`PUT`), which is what the running server
 * accepts. This test pins `PUT` on the hand-rolled side and documents the
 * generated-client mismatch so the regression is visible, not invisible.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  addComment,
  createFault,
  getFault,
  listFaultComments,
  listFaults,
  resolveFault,
  updateFault,
} from './api';

const FAULT_ID = '11111111-1111-1111-1111-111111111111';

/** Minimal ok JSON response stub. */
function okJson(body: unknown): Response {
  return {
    ok: true,
    status: 200,
    json: async () => body,
  } as Response;
}

/** Read the (url, method) pair the client passed to the last fetch call. */
function lastCall(): { url: string; method: string } {
  const calls = vi.mocked(fetch).mock.calls;
  const call = calls[calls.length - 1];
  if (!call) throw new Error('fetch was not called');
  const url = String(call[0]);
  const method = ((call[1] as RequestInit | undefined)?.method ?? 'GET').toUpperCase();
  return { url, method };
}

describe('faults api client — request wiring contract', () => {
  beforeEach(() => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(okJson({}));
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('listFaults → GET /api/v1/faults', async () => {
    await listFaults();
    const { url, method } = lastCall();
    expect(method).toBe('GET');
    // No filters → bare collection path (no trailing `?`).
    expect(url).toBe('/api/v1/faults');
  });

  it('listFaults serializes filters as snake_case query params', async () => {
    await listFaults({ building_id: 'b-1', status: 'new', page: 2, limit: 25 });
    const { url } = lastCall();
    const qs = new URLSearchParams(url.split('?')[1] ?? '');
    expect(url.split('?')[0]).toBe('/api/v1/faults');
    expect(qs.get('building_id')).toBe('b-1');
    expect(qs.get('status')).toBe('new');
    expect(qs.get('page')).toBe('2');
    expect(qs.get('limit')).toBe('25');
  });

  it('createFault → POST /api/v1/faults', async () => {
    await createFault({} as Parameters<typeof createFault>[0]);
    expect(lastCall()).toEqual({ url: '/api/v1/faults', method: 'POST' });
  });

  it('getFault → GET /api/v1/faults/{id}', async () => {
    await getFault(FAULT_ID);
    expect(lastCall()).toEqual({ url: `/api/v1/faults/${FAULT_ID}`, method: 'GET' });
  });

  it('updateFault → PUT /api/v1/faults/{id} (matches the live backend route)', async () => {
    await updateFault(FAULT_ID, {} as Parameters<typeof updateFault>[1]);
    const { url, method } = lastCall();
    expect(url).toBe(`/api/v1/faults/${FAULT_ID}`);
    // Backend `router()` registers `.route("/{id}", put(update_fault))`.
    // The generated FaultsService emits PATCH; the hand-rolled client must
    // stay on PUT to match the running server. Pinned to catch silent flips.
    expect(method).toBe('PUT');
  });

  it('listFaultComments → GET /api/v1/faults/{id}/comments and unwraps `comments`', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(okJson({ comments: [{ id: 'c-1' }] }));
    const comments = await listFaultComments(FAULT_ID);
    expect(lastCall()).toEqual({
      url: `/api/v1/faults/${FAULT_ID}/comments`,
      method: 'GET',
    });
    expect(comments).toEqual([{ id: 'c-1' }]);
  });

  it('addComment → POST /api/v1/faults/{id}/comments', async () => {
    await addComment(FAULT_ID, { content: 'hi' } as Parameters<typeof addComment>[1]);
    expect(lastCall()).toEqual({
      url: `/api/v1/faults/${FAULT_ID}/comments`,
      method: 'POST',
    });
  });

  it('resolveFault → POST /api/v1/faults/{id}/resolve', async () => {
    await resolveFault(FAULT_ID, {} as Parameters<typeof resolveFault>[1]);
    expect(lastCall()).toEqual({
      url: `/api/v1/faults/${FAULT_ID}/resolve`,
      method: 'POST',
    });
  });
});

/**
 * Cross-check the overlap explicitly against the generated `FaultsService`
 * contract. For every fault operation that exists in *both* the generated
 * client and the hand-rolled client, the URL template and method must agree
 * (with the one documented PUT/PATCH update divergence). This array is the
 * machine-checkable statement of "hand-rolled client ⊆ generated contract".
 */
describe('faults api client ↔ generated FaultsService contract', () => {
  beforeEach(() => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(okJson({ comments: [], attachments: [] }));
  });
  afterEach(() => vi.restoreAllMocks());

  // url template (with `{id}` placeholder) + method, as declared by the
  // generated FaultsService in src/generated/services.ts.
  const GENERATED_CONTRACT = {
    list: { url: '/api/v1/faults', method: 'GET' },
    create: { url: '/api/v1/faults', method: 'POST' },
    get: { url: '/api/v1/faults/{id}', method: 'GET' },
    listComments: { url: '/api/v1/faults/{id}/comments', method: 'GET' },
    addComment: { url: '/api/v1/faults/{id}/comments', method: 'POST' },
    resolve: { url: '/api/v1/faults/{id}/resolve', method: 'POST' },
  } as const;

  function normalize(url: string): string {
    return url.split('?')[0].replace(FAULT_ID, '{id}');
  }

  it('list matches generated contract', async () => {
    await listFaults();
    const c = lastCall();
    expect({ url: normalize(c.url), method: c.method }).toEqual(GENERATED_CONTRACT.list);
  });

  it('create matches generated contract', async () => {
    await createFault({} as Parameters<typeof createFault>[0]);
    const c = lastCall();
    expect({ url: normalize(c.url), method: c.method }).toEqual(GENERATED_CONTRACT.create);
  });

  it('get matches generated contract', async () => {
    await getFault(FAULT_ID);
    const c = lastCall();
    expect({ url: normalize(c.url), method: c.method }).toEqual(GENERATED_CONTRACT.get);
  });

  it('listComments matches generated contract', async () => {
    await listFaultComments(FAULT_ID);
    const c = lastCall();
    expect({ url: normalize(c.url), method: c.method }).toEqual(GENERATED_CONTRACT.listComments);
  });

  it('addComment matches generated contract', async () => {
    await addComment(FAULT_ID, { content: 'x' } as Parameters<typeof addComment>[1]);
    const c = lastCall();
    expect({ url: normalize(c.url), method: c.method }).toEqual(GENERATED_CONTRACT.addComment);
  });

  it('resolve matches generated contract', async () => {
    await resolveFault(FAULT_ID, {} as Parameters<typeof resolveFault>[1]);
    const c = lastCall();
    expect({ url: normalize(c.url), method: c.method }).toEqual(GENERATED_CONTRACT.resolve);
  });
});
