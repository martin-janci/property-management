import {
  type Announcement,
  type ApiAnnouncementSummary,
  derivePinnedItems,
  extractItems,
  filterMainList,
  toUiAnnouncement,
} from './AnnouncementsScreen';

// Regression for PR #918 (High #3 + Critical #1): the screen moved off the
// manager-only `/api/v1/announcements` endpoint (403 for residents) to the
// RLS-scoped `/api/v1/announcements/published`, whose `AnnouncementSummary`
// rows have NO `content`/`author`/`category`. The mapper must tolerate the
// lightweight summary shape and the list extractor must read `announcements`
// (the legacy `items` key no longer exists on this response).

const summary: ApiAnnouncementSummary = {
  id: 'a-1',
  title: 'Lift maintenance Friday',
  status: 'published',
  target_type: 'building',
  published_at: '2026-05-20T08:00:00Z',
  pinned: true,
  comments_enabled: true,
  acknowledgment_required: false,
};

describe('toUiAnnouncement (PR #918 published-summary mapping)', () => {
  it('maps a summary row without a content body', () => {
    const ui = toUiAnnouncement(summary);
    expect(ui).toMatchObject({
      id: 'a-1',
      title: 'Lift maintenance Friday',
      category: 'general',
      createdAt: '2026-05-20T08:00:00Z', // from published_at
      isPinned: true,
    });
    // The summary carries no content body; the UI shape must not expose one.
    expect((ui as unknown as Record<string, unknown>).content).toBeUndefined();
  });

  it('falls back to a synthetic createdAt when published_at is null', () => {
    const ui = toUiAnnouncement({ ...summary, published_at: null });
    expect(typeof ui.createdAt).toBe('string');
    expect(ui.createdAt.length).toBeGreaterThan(0);
  });
});

describe('extractItems (PR #918 reads `announcements`, not `items`)', () => {
  it('returns the announcements array from the published-list response', () => {
    expect(extractItems({ announcements: [summary], count: 1 })).toEqual([summary]);
  });

  it('returns [] for undefined / empty responses', () => {
    expect(extractItems(undefined)).toEqual([]);
    expect(extractItems({})).toEqual([]);
  });

  it('ignores a stray legacy `items` key (no longer part of the contract)', () => {
    // `items` is not in the response type; cast to confirm it is NOT read.
    const legacy = { items: [summary] } as unknown as Parameters<typeof extractItems>[0];
    expect(extractItems(legacy)).toEqual([]);
  });
});

// Regression for PR #943 (#767 dev-review tail): the screen dropped the
// dedicated `?pinned=true` query and now derives the sticky pinned band
// client-side from the single published list, and partitions the main feed
// from that same list. These behaviours used to be inlined in the component
// body (untested); PR #943 shipped without pinning them. The two helpers
// below pin the client-side derivation so it cannot silently regress.

const ui = (over: Partial<Announcement> & Pick<Announcement, 'id'>): Announcement => ({
  title: over.title ?? over.id,
  category: 'general',
  createdAt: '2026-05-01T00:00:00Z',
  author: 'Building Management',
  isRead: false,
  isPinned: false,
  attachments: [],
  commentsCount: 0,
  ...over,
});

describe('derivePinnedItems (PR #943 client-side pinned band)', () => {
  it('keeps only pinned rows', () => {
    const items = [
      ui({ id: 'a', isPinned: true }),
      ui({ id: 'b', isPinned: false }),
      ui({ id: 'c', isPinned: true }),
    ];
    expect(derivePinnedItems(items).map((a) => a.id)).toEqual(['a', 'c']);
  });

  it('returns [] when nothing is pinned', () => {
    expect(derivePinnedItems([ui({ id: 'a' }), ui({ id: 'b' })])).toEqual([]);
  });
});

describe('filterMainList (PR #943 main feed partitioning)', () => {
  const items = [
    ui({ id: 'pin', isPinned: true, createdAt: '2026-05-05T00:00:00Z' }),
    ui({
      id: 'old',
      title: 'Lift maintenance',
      category: 'maintenance',
      createdAt: '2026-05-01T00:00:00Z',
    }),
    ui({
      id: 'new',
      title: 'Garden event',
      category: 'event',
      createdAt: '2026-05-03T00:00:00Z',
    }),
  ];

  it('excludes pinned items from the main feed', () => {
    expect(filterMainList(items, 'all', '').map((a) => a.id)).not.toContain('pin');
  });

  it('sorts the remaining items newest-first', () => {
    expect(filterMainList(items, 'all', '').map((a) => a.id)).toEqual(['new', 'old']);
  });

  it('applies the active category filter', () => {
    expect(filterMainList(items, 'maintenance', '').map((a) => a.id)).toEqual(['old']);
  });

  it('matches the search query against the TITLE only (no content body since #943)', () => {
    expect(filterMainList(items, 'all', 'garden').map((a) => a.id)).toEqual(['new']);
    // Case-insensitive.
    expect(filterMainList(items, 'all', 'LIFT').map((a) => a.id)).toEqual(['old']);
    // A term that matched the old `content` body must NOT match now.
    expect(filterMainList(items, 'all', 'no-such-title')).toEqual([]);
  });
});
