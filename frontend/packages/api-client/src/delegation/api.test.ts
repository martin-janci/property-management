/**
 * Contract tests for the hand-rolled delegation API client (Epic 3, Story 3.4).
 *
 * `delegation/api.ts` is a hand-written transport layer the ppt-web delegations
 * feature calls directly. These tests pin the request wiring (URL template +
 * HTTP method + body) of every endpoint against
 * `backend/servers/api-server/src/routes/delegations.rs`, so the client can't
 * silently drift onto the wrong path/verb and 404 at runtime with no
 * compile-time signal.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  acceptDelegation,
  checkDelegation,
  createDelegation,
  declineDelegation,
  getDelegation,
  listMyDelegations,
  listReceivedDelegations,
  revokeDelegation,
} from './api';

const ID = '11111111-1111-1111-1111-111111111111';
const UNIT_ID = '22222222-2222-2222-2222-222222222222';
const DELEGATE_ID = '33333333-3333-3333-3333-333333333333';

/** Minimal ok JSON response stub. */
function okJson(body: unknown): Response {
  return {
    ok: true,
    status: 200,
    json: async () => body,
  } as Response;
}

/** Read the (url, method, body) the client passed to the last fetch call. */
function lastCall(): { url: string; method: string; body: string | undefined } {
  const calls = vi.mocked(fetch).mock.calls;
  const call = calls[calls.length - 1];
  if (!call) throw new Error('fetch was not called');
  const url = String(call[0]);
  const init = call[1] as RequestInit | undefined;
  const method = (init?.method ?? 'GET').toUpperCase();
  const body = init?.body == null ? undefined : String(init.body);
  return { url, method, body };
}

describe('delegation api client — request wiring contract', () => {
  beforeEach(() => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(okJson({}));
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('createDelegation → POST /api/v1/delegations with JSON body', async () => {
    await createDelegation({ delegate_user_id: DELEGATE_ID, scopes: ['voting'] });
    const { url, method, body } = lastCall();
    expect(method).toBe('POST');
    expect(url).toBe('/api/v1/delegations');
    expect(JSON.parse(body ?? '{}')).toEqual({
      delegate_user_id: DELEGATE_ID,
      scopes: ['voting'],
    });
  });

  it('listMyDelegations → GET /api/v1/delegations (bare path, no filters)', async () => {
    await listMyDelegations();
    const { url, method } = lastCall();
    expect(method).toBe('GET');
    expect(url).toBe('/api/v1/delegations');
  });

  it('listMyDelegations serializes filters as snake_case query params', async () => {
    await listMyDelegations({ unit_id: UNIT_ID, status: 'active' });
    const { url } = lastCall();
    const qs = new URLSearchParams(url.split('?')[1] ?? '');
    expect(qs.get('unit_id')).toBe(UNIT_ID);
    expect(qs.get('status')).toBe('active');
  });

  it('listReceivedDelegations → GET /api/v1/delegations/received', async () => {
    await listReceivedDelegations();
    const { url, method } = lastCall();
    expect(method).toBe('GET');
    expect(url).toBe('/api/v1/delegations/received');
  });

  it('getDelegation → GET /api/v1/delegations/{id}', async () => {
    await getDelegation(ID);
    const { url, method } = lastCall();
    expect(method).toBe('GET');
    expect(url).toBe(`/api/v1/delegations/${ID}`);
  });

  it('acceptDelegation → POST /api/v1/delegations/{id}/accept', async () => {
    await acceptDelegation(ID);
    const { url, method } = lastCall();
    expect(method).toBe('POST');
    expect(url).toBe(`/api/v1/delegations/${ID}/accept`);
  });

  it('declineDelegation → POST /api/v1/delegations/{id}/decline', async () => {
    await declineDelegation(ID);
    const { url, method } = lastCall();
    expect(method).toBe('POST');
    expect(url).toBe(`/api/v1/delegations/${ID}/decline`);
  });

  it('revokeDelegation → DELETE /api/v1/delegations/{id}/revoke', async () => {
    await revokeDelegation(ID);
    const { url, method } = lastCall();
    expect(method).toBe('DELETE');
    expect(url).toBe(`/api/v1/delegations/${ID}/revoke`);
  });

  it('checkDelegation → GET /api/v1/delegations/check/{unit_id}/{scope}', async () => {
    await checkDelegation(UNIT_ID, 'voting');
    const { url, method } = lastCall();
    expect(method).toBe('GET');
    expect(url).toBe(`/api/v1/delegations/check/${UNIT_ID}/voting`);
  });
});
