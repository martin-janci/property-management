/**
 * Contract tests for the announcement list client (#751).
 *
 * The backend `ListAnnouncementsQuery` expects `limit` / `offset` /
 * `target_type` / `from_date` / `to_date` and returns
 * `{ announcements, count, total }` with snake_case summary fields. The client
 * exposes the UI-friendly `page` / `pageSize` / `targetType` shape. These tests
 * pin the request-param translation and the response normalisation so the two
 * contracts can't silently drift apart again (lists rendering empty).
 */

import { describe, expect, it } from 'vitest';
import {
  type BackendAnnouncementListResponse,
  buildAnnouncementQuery,
  toPaginatedResponse,
} from './hooks';

describe('buildAnnouncementQuery', () => {
  it('returns an empty string when no params are given', () => {
    expect(buildAnnouncementQuery()).toBe('');
  });

  it('translates page/pageSize to backend limit/offset', () => {
    const sp = new URLSearchParams(buildAnnouncementQuery({ page: 3, pageSize: 20 }).slice(1));
    expect(sp.get('limit')).toBe('20');
    // page 3 @ 20 per page → offset 40
    expect(sp.get('offset')).toBe('40');
  });

  it('maps camelCase filters to snake_case backend names', () => {
    const sp = new URLSearchParams(
      buildAnnouncementQuery({
        targetType: 'building',
        authorId: 'author-1',
        fromDate: '2026-01-01',
        toDate: '2026-02-01',
      }).slice(1)
    );
    expect(sp.get('target_type')).toBe('building');
    expect(sp.get('author_id')).toBe('author-1');
    expect(sp.get('from_date')).toBe('2026-01-01');
    expect(sp.get('to_date')).toBe('2026-02-01');
    // The old camelCase keys must NOT be sent.
    expect(sp.get('targetType')).toBeNull();
    expect(sp.get('fromDate')).toBeNull();
  });

  it('does not emit offset when only pageSize is given', () => {
    const sp = new URLSearchParams(buildAnnouncementQuery({ pageSize: 10 }).slice(1));
    expect(sp.get('limit')).toBe('10');
    expect(sp.get('offset')).toBeNull();
  });
});

describe('toPaginatedResponse', () => {
  const raw: BackendAnnouncementListResponse = {
    announcements: [
      {
        id: 'a1',
        title: 'Hello',
        status: 'published',
        target_type: 'all',
        published_at: '2026-01-01T00:00:00Z',
        pinned: true,
        comments_enabled: false,
        acknowledgment_required: true,
      },
    ],
    count: 1,
    total: 42,
  };

  it('maps snake_case summary fields to the camelCase UI shape', () => {
    const result = toPaginatedResponse(raw, { page: 2, pageSize: 10 });
    expect(result.items[0]).toEqual({
      id: 'a1',
      title: 'Hello',
      status: 'published',
      targetType: 'all',
      publishedAt: '2026-01-01T00:00:00Z',
      pinned: true,
      commentsEnabled: false,
      acknowledgmentRequired: true,
    });
  });

  it('derives total / page / pageSize / totalPages', () => {
    const result = toPaginatedResponse(raw, { page: 2, pageSize: 10 });
    expect(result.total).toBe(42);
    expect(result.page).toBe(2);
    expect(result.pageSize).toBe(10);
    expect(result.totalPages).toBe(5); // ceil(42 / 10)
  });

  it('tolerates a missing announcements array', () => {
    const result = toPaginatedResponse({ count: 0, total: 0 } as BackendAnnouncementListResponse);
    expect(result.items).toEqual([]);
    expect(result.total).toBe(0);
  });
});
