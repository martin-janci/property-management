import { describe, expect, it } from 'vitest';
import { mapAcknowledgmentStatsResponse, mapCommentsResponse } from './hooks';

describe('announcement engagement wire mapping', () => {
  it('maps snake_case comments (and nested replies) to the camelCase UI shape', () => {
    const mapped = mapCommentsResponse({
      comments: [
        {
          id: 'c-1',
          announcement_id: 'a-1',
          user_id: 'u-1',
          parent_id: null,
          content: 'Hello',
          author_name: 'Jana Nováková',
          is_deleted: false,
          created_at: '2026-11-01T10:00:00Z',
          updated_at: '2026-11-01T10:00:00Z',
          replies: [
            {
              id: 'c-2',
              announcement_id: 'a-1',
              user_id: 'u-2',
              parent_id: 'c-1',
              content: 'Hi',
              author_name: 'Peter',
              is_deleted: true,
              created_at: '2026-11-01T11:00:00Z',
              updated_at: '2026-11-01T11:00:00Z',
            },
          ],
        },
      ],
      count: 1,
      total: 2,
    });

    expect(mapped.total).toBe(2);
    expect(mapped.comments[0]).toMatchObject({
      announcementId: 'a-1',
      userId: 'u-1',
      authorName: 'Jana Nováková',
      isDeleted: false,
      createdAt: '2026-11-01T10:00:00Z',
    });
    expect(mapped.comments[0].replies?.[0]).toMatchObject({
      parentId: 'c-1',
      authorName: 'Peter',
      isDeleted: true,
    });
  });

  it('maps snake_case acknowledgment stats', () => {
    expect(
      mapAcknowledgmentStatsResponse({
        stats: {
          announcement_id: 'a-1',
          total_targeted: 40,
          read_count: 25,
          acknowledged_count: 10,
          pending_count: 30,
        },
      })
    ).toEqual({
      stats: {
        announcementId: 'a-1',
        totalTargeted: 40,
        readCount: 25,
        acknowledgedCount: 10,
        pendingCount: 30,
      },
    });
  });
});
