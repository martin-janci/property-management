/**
 * Contract tests for the hand-rolled voting API client (Epic 5, FR37-44).
 *
 * `voting/api.ts` is a hand-written transport layer that the ppt-web voting
 * feature calls directly (mirroring the `faults` module). These tests pin the
 * request wiring (URL template + HTTP method) of every endpoint the web UI
 * touches against `backend/servers/api-server/src/routes/voting.rs`, so the
 * client can't silently drift onto the wrong path/verb and 404 at runtime with
 * no compile-time signal.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  addComment,
  addQuestion,
  cancelVote,
  castVote,
  closeVote,
  createVote,
  deleteVote,
  getEligibility,
  getResults,
  getVote,
  listComments,
  listVotes,
  publishVote,
  updateVote,
} from './api';

const VOTE_ID = '11111111-1111-1111-1111-111111111111';

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

describe('voting api client — request wiring contract', () => {
  beforeEach(() => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(okJson({}));
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('listVotes → GET /api/v1/voting (bare path, no filters)', async () => {
    await listVotes();
    const { url, method } = lastCall();
    expect(method).toBe('GET');
    expect(url).toBe('/api/v1/voting');
  });

  it('listVotes serializes filters as snake_case query params', async () => {
    await listVotes({ building_id: 'b-1', status: 'active', limit: 25, offset: 50 });
    const { url } = lastCall();
    const qs = new URLSearchParams(url.split('?')[1] ?? '');
    expect(url.split('?')[0]).toBe('/api/v1/voting');
    expect(qs.get('building_id')).toBe('b-1');
    expect(qs.get('status')).toBe('active');
    expect(qs.get('limit')).toBe('25');
    expect(qs.get('offset')).toBe('50');
  });

  it('createVote → POST /api/v1/voting', async () => {
    await createVote({} as Parameters<typeof createVote>[0]);
    expect(lastCall()).toEqual({ url: '/api/v1/voting', method: 'POST' });
  });

  it('getVote → GET /api/v1/voting/{id}', async () => {
    await getVote(VOTE_ID);
    expect(lastCall()).toEqual({ url: `/api/v1/voting/${VOTE_ID}`, method: 'GET' });
  });

  it('updateVote → PUT /api/v1/voting/{id}', async () => {
    await updateVote(VOTE_ID, {} as Parameters<typeof updateVote>[1]);
    expect(lastCall()).toEqual({ url: `/api/v1/voting/${VOTE_ID}`, method: 'PUT' });
  });

  it('deleteVote → DELETE /api/v1/voting/{id}', async () => {
    await deleteVote(VOTE_ID);
    expect(lastCall()).toEqual({ url: `/api/v1/voting/${VOTE_ID}`, method: 'DELETE' });
  });

  it('publishVote → POST /api/v1/voting/{id}/publish', async () => {
    await publishVote(VOTE_ID);
    expect(lastCall()).toEqual({ url: `/api/v1/voting/${VOTE_ID}/publish`, method: 'POST' });
  });

  it('cancelVote → POST /api/v1/voting/{id}/cancel', async () => {
    await cancelVote(VOTE_ID, { reason: 'x' } as Parameters<typeof cancelVote>[1]);
    expect(lastCall()).toEqual({ url: `/api/v1/voting/${VOTE_ID}/cancel`, method: 'POST' });
  });

  it('closeVote → POST /api/v1/voting/{id}/close', async () => {
    await closeVote(VOTE_ID);
    expect(lastCall()).toEqual({ url: `/api/v1/voting/${VOTE_ID}/close`, method: 'POST' });
  });

  it('addQuestion → POST /api/v1/voting/{id}/questions', async () => {
    await addQuestion(VOTE_ID, {} as Parameters<typeof addQuestion>[1]);
    expect(lastCall()).toEqual({ url: `/api/v1/voting/${VOTE_ID}/questions`, method: 'POST' });
  });

  it('getEligibility → GET /api/v1/voting/{id}/eligibility', async () => {
    await getEligibility(VOTE_ID);
    expect(lastCall()).toEqual({ url: `/api/v1/voting/${VOTE_ID}/eligibility`, method: 'GET' });
  });

  it('castVote → POST /api/v1/voting/{id}/cast', async () => {
    await castVote(VOTE_ID, {} as Parameters<typeof castVote>[1]);
    expect(lastCall()).toEqual({ url: `/api/v1/voting/${VOTE_ID}/cast`, method: 'POST' });
  });

  it('getResults → GET /api/v1/voting/{id}/results', async () => {
    await getResults(VOTE_ID);
    expect(lastCall()).toEqual({ url: `/api/v1/voting/${VOTE_ID}/results`, method: 'GET' });
  });

  it('listComments → GET /api/v1/voting/{id}/comments', async () => {
    await listComments(VOTE_ID);
    expect(lastCall()).toEqual({ url: `/api/v1/voting/${VOTE_ID}/comments`, method: 'GET' });
  });

  it('addComment → POST /api/v1/voting/{id}/comments', async () => {
    await addComment(VOTE_ID, { content: 'hi' } as Parameters<typeof addComment>[1]);
    expect(lastCall()).toEqual({ url: `/api/v1/voting/${VOTE_ID}/comments`, method: 'POST' });
  });
});
