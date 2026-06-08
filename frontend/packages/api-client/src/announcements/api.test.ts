/**
 * Contract tests for the hand-rolled announcement API client (story 79-1).
 *
 * Announcements have no generated `AnnouncementsService` — the hand-rolled
 * `createAnnouncementsApi` transport in `announcements/api.ts` IS the contract
 * the ppt-web announcements feature depends on. Without coverage, a typo in a
 * path segment or a wrong HTTP verb ships silently and 404s at runtime. These
 * tests pin the request wiring (URL + method) for every endpoint the client
 * exposes, plus the auth/tenant header construction.
 *
 * The companion `hooks.test.ts` already pins the list-query translation and
 * response normalisation; this file covers the transport surface those hooks
 * delegate to.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createAnnouncementsApi } from './api';

const BASE = 'https://api.test';
const ROOT = `${BASE}/api/v1/announcements`;
const ID = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa';

function okJson(body: unknown): Response {
  return { ok: true, status: 200, json: async () => body } as Response;
}
function ok204(): Response {
  return {
    ok: true,
    status: 204,
    json: async () => {
      throw new Error('no body');
    },
  } as unknown as Response;
}

function lastCall(): { url: string; method: string; headers: Record<string, string> } {
  const calls = vi.mocked(fetch).mock.calls;
  const call = calls[calls.length - 1];
  if (!call) throw new Error('fetch was not called');
  const init = call[1] as RequestInit | undefined;
  return {
    url: String(call[0]),
    method: (init?.method ?? 'GET').toUpperCase(),
    headers: (init?.headers ?? {}) as Record<string, string>,
  };
}

const api = () =>
  createAnnouncementsApi({
    baseUrl: BASE,
    accessToken: 'tok-123',
    tenantId: 'tenant-9',
  });

describe('announcements api client — request wiring contract', () => {
  beforeEach(() => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(okJson({}));
  });
  afterEach(() => vi.restoreAllMocks());

  it('list → GET collection root', async () => {
    await api().list();
    const c = lastCall();
    expect(c.method).toBe('GET');
    expect(c.url).toBe(ROOT);
  });

  it('list serializes camelCase filters as query params', async () => {
    await api().list({ page: 2, pageSize: 20, status: 'published', targetType: 'building' });
    const c = lastCall();
    const qs = new URLSearchParams(c.url.split('?')[1] ?? '');
    expect(c.url.split('?')[0]).toBe(ROOT);
    expect(qs.get('page')).toBe('2');
    expect(qs.get('pageSize')).toBe('20');
    expect(qs.get('status')).toBe('published');
    expect(qs.get('targetType')).toBe('building');
  });

  it('listPublished → GET /published', async () => {
    await api().listPublished();
    expect(lastCall()).toMatchObject({ url: `${ROOT}/published`, method: 'GET' });
  });

  it('create → POST collection root', async () => {
    await api().create({} as Parameters<ReturnType<typeof createAnnouncementsApi>['create']>[0]);
    expect(lastCall()).toMatchObject({ url: ROOT, method: 'POST' });
  });

  it('get → GET /{id}', async () => {
    await api().get(ID);
    expect(lastCall()).toMatchObject({ url: `${ROOT}/${ID}`, method: 'GET' });
  });

  it('update → PUT /{id}', async () => {
    await api().update(
      ID,
      {} as Parameters<ReturnType<typeof createAnnouncementsApi>['update']>[1]
    );
    expect(lastCall()).toMatchObject({ url: `${ROOT}/${ID}`, method: 'PUT' });
  });

  it('delete → DELETE /{id} (tolerates empty body)', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(ok204());
    await expect(api().delete(ID)).resolves.toBeUndefined();
    expect(lastCall()).toMatchObject({ url: `${ROOT}/${ID}`, method: 'DELETE' });
  });

  it('publish → POST /{id}/publish', async () => {
    await api().publish(ID);
    expect(lastCall()).toMatchObject({ url: `${ROOT}/${ID}/publish`, method: 'POST' });
  });

  it('schedule → POST /{id}/schedule', async () => {
    await api().schedule(
      ID,
      {} as Parameters<ReturnType<typeof createAnnouncementsApi>['schedule']>[1]
    );
    expect(lastCall()).toMatchObject({ url: `${ROOT}/${ID}/schedule`, method: 'POST' });
  });

  it('archive → POST /{id}/archive', async () => {
    await api().archive(ID);
    expect(lastCall()).toMatchObject({ url: `${ROOT}/${ID}/archive`, method: 'POST' });
  });

  it('pin → POST /{id}/pin', async () => {
    await api().pin(ID, { pinned: true } as Parameters<
      ReturnType<typeof createAnnouncementsApi>['pin']
    >[1]);
    expect(lastCall()).toMatchObject({ url: `${ROOT}/${ID}/pin`, method: 'POST' });
  });

  it('markRead → POST /{id}/read', async () => {
    await api().markRead(ID);
    expect(lastCall()).toMatchObject({ url: `${ROOT}/${ID}/read`, method: 'POST' });
  });

  it('acknowledge → POST /{id}/acknowledge', async () => {
    await api().acknowledge(ID);
    expect(lastCall()).toMatchObject({ url: `${ROOT}/${ID}/acknowledge`, method: 'POST' });
  });

  it('getAcknowledgmentStats → GET /{id}/acknowledgments', async () => {
    await api().getAcknowledgmentStats(ID);
    expect(lastCall()).toMatchObject({ url: `${ROOT}/${ID}/acknowledgments`, method: 'GET' });
  });

  it('listComments → GET /{id}/comments', async () => {
    await api().listComments(ID);
    expect(lastCall()).toMatchObject({ url: `${ROOT}/${ID}/comments`, method: 'GET' });
  });

  it('createComment → POST /{id}/comments', async () => {
    await api().createComment(
      ID,
      {} as Parameters<ReturnType<typeof createAnnouncementsApi>['createComment']>[1]
    );
    expect(lastCall()).toMatchObject({ url: `${ROOT}/${ID}/comments`, method: 'POST' });
  });

  it('deleteComment → DELETE /{id}/comments/{commentId}', async () => {
    await api().deleteComment(ID, 'c-1');
    expect(lastCall()).toMatchObject({
      url: `${ROOT}/${ID}/comments/c-1`,
      method: 'DELETE',
    });
  });

  it('getStatistics / getUnreadCount → GET aggregate endpoints', async () => {
    await api().getStatistics();
    expect(lastCall()).toMatchObject({ url: `${ROOT}/statistics`, method: 'GET' });
    await api().getUnreadCount();
    expect(lastCall()).toMatchObject({ url: `${ROOT}/unread-count`, method: 'GET' });
  });
});

describe('announcements api client — auth/tenant headers', () => {
  beforeEach(() => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(okJson({}));
  });
  afterEach(() => vi.restoreAllMocks());

  it('attaches Authorization + X-Tenant-ID + JSON content type', async () => {
    await api().list();
    const { headers } = lastCall();
    expect(headers.Authorization).toBe('Bearer tok-123');
    expect(headers['X-Tenant-ID']).toBe('tenant-9');
    expect(headers['Content-Type']).toBe('application/json');
  });

  it('omits Authorization / tenant headers when not configured', async () => {
    const anon = createAnnouncementsApi({ baseUrl: BASE });
    await anon.list();
    const { headers } = lastCall();
    expect(headers.Authorization).toBeUndefined();
    expect(headers['X-Tenant-ID']).toBeUndefined();
    expect(headers['Content-Type']).toBe('application/json');
  });
});
