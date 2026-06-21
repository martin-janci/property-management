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
  insertCommentIntoTree,
  OPTIMISTIC_COMMENT_PREFIX,
  toPaginatedResponse,
} from './hooks';
import type { CommentWithAuthor } from './types';

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

describe('insertCommentIntoTree (Story 6.3 — optimistic add)', () => {
  const mkComment = (over: Partial<CommentWithAuthor>): CommentWithAuthor => ({
    id: 'c1',
    announcementId: 'a1',
    userId: 'u1',
    content: 'hi',
    authorName: 'Alice',
    isDeleted: false,
    createdAt: '2026-06-18T00:00:00Z',
    updatedAt: '2026-06-18T00:00:00Z',
    ...over,
  });

  it('appends a top-level comment when parentId is absent', () => {
    const existing = [mkComment({ id: 'c1' })];
    const added = mkComment({ id: 'c2', content: 'new top' });
    const result = insertCommentIntoTree(existing, added);
    expect(result).toHaveLength(2);
    expect(result[1].id).toBe('c2');
    // Original array is not mutated.
    expect(existing).toHaveLength(1);
  });

  it('nests a reply under the matching parent', () => {
    const existing = [mkComment({ id: 'c1', replies: [] })];
    const reply = mkComment({ id: 'r1', parentId: 'c1', content: 'a reply' });
    const result = insertCommentIntoTree(existing, reply);
    expect(result[0].replies).toHaveLength(1);
    expect(result[0].replies?.[0].id).toBe('r1');
  });

  it('nests a reply under a deeply-nested parent', () => {
    const existing = [
      mkComment({ id: 'c1', replies: [mkComment({ id: 'c1a', parentId: 'c1', replies: [] })] }),
    ];
    const reply = mkComment({ id: 'r2', parentId: 'c1a' });
    const result = insertCommentIntoTree(existing, reply);
    expect(result[0].replies?.[0].replies?.[0].id).toBe('r2');
  });

  it('leaves the tree unchanged when the parent is missing', () => {
    const existing = [mkComment({ id: 'c1' })];
    const orphan = mkComment({ id: 'r3', parentId: 'does-not-exist' });
    const result = insertCommentIntoTree(existing, orphan);
    expect(result).toHaveLength(1);
    expect(result[0].replies ?? []).toHaveLength(0);
  });

  it('exposes a stable optimistic-id prefix', () => {
    expect(OPTIMISTIC_COMMENT_PREFIX).toBe('optimistic-');
  });
});
