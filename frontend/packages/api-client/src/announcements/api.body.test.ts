/**
 * Announcement write bodies match the api-server's snake_case request structs
 * (round-3 audit #8: create failed with 422 `missing field target_type`).
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  createAnnouncementsApi,
  toAnnouncementCommentBody,
  toAnnouncementScheduleBody,
  toAnnouncementWriteBody,
} from './api';

const ID = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa';

function okJson(body: unknown): Response {
  return { ok: true, status: 200, json: async () => body } as Response;
}

function lastBody(): Record<string, unknown> {
  const calls = vi.mocked(fetch).mock.calls;
  const init = calls[calls.length - 1]?.[1] as RequestInit | undefined;
  return JSON.parse(String(init?.body ?? '{}'));
}

describe('toAnnouncementWriteBody', () => {
  it('maps every create field to snake_case', () => {
    expect(
      toAnnouncementWriteBody({
        title: 'Water shutoff',
        content: 'Tomorrow 8-12',
        targetType: 'building',
        targetIds: ['b1'],
        scheduledAt: '2026-01-01T08:00:00Z',
        commentsEnabled: true,
        acknowledgmentRequired: false,
      })
    ).toEqual({
      title: 'Water shutoff',
      content: 'Tomorrow 8-12',
      target_type: 'building',
      target_ids: ['b1'],
      scheduled_at: '2026-01-01T08:00:00Z',
      comments_enabled: true,
      acknowledgment_required: false,
    });
  });

  it('drops fields that are not set (partial update)', () => {
    expect(toAnnouncementWriteBody({ title: 'New title' })).toEqual({ title: 'New title' });
  });
});

describe('toAnnouncementScheduleBody / toAnnouncementCommentBody', () => {
  it('maps scheduledAt', () => {
    expect(toAnnouncementScheduleBody({ scheduledAt: '2026-01-01T08:00:00Z' })).toEqual({
      scheduled_at: '2026-01-01T08:00:00Z',
    });
  });

  it('maps parentId and aiTrainingConsent', () => {
    expect(
      toAnnouncementCommentBody({ content: 'hi', parentId: 'c1', aiTrainingConsent: true })
    ).toEqual({ content: 'hi', parent_id: 'c1', ai_training_consent: true });
    expect(toAnnouncementCommentBody({ content: 'hi' })).toEqual({ content: 'hi' });
  });
});

describe('createAnnouncementsApi sends snake_case bodies', () => {
  const api = () => createAnnouncementsApi({ baseUrl: 'https://api.test', accessToken: 't' });

  beforeEach(() => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(okJson({}));
  });
  afterEach(() => vi.restoreAllMocks());

  it('create', async () => {
    await api().create({ title: 'T', content: 'C', targetType: 'all' });
    expect(lastBody()).toEqual({ title: 'T', content: 'C', target_type: 'all' });
  });

  it('update', async () => {
    await api().update(ID, { targetType: 'units', targetIds: ['u1'] });
    expect(lastBody()).toEqual({ target_type: 'units', target_ids: ['u1'] });
  });

  it('schedule', async () => {
    await api().schedule(ID, { scheduledAt: '2026-01-01T08:00:00Z' });
    expect(lastBody()).toEqual({ scheduled_at: '2026-01-01T08:00:00Z' });
  });

  it('createComment', async () => {
    await api().createComment(ID, { content: 'reply', parentId: 'c1' });
    expect(lastBody()).toEqual({ content: 'reply', parent_id: 'c1' });
  });
});
