import {
  type Announcement,
  type ApiAnnouncementSummary,
  applyReadState,
  buildAnnouncements,
  countUnread,
  derivePinnedItems,
  extractItems,
  filterMainList,
  formatRelativeDate,
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
      createdAt: '2026-05-01T00:00:00Z',
    }),
    ui({
      id: 'new',
      title: 'Garden event',
      createdAt: '2026-05-03T00:00:00Z',
    }),
  ];

  it('excludes pinned items from the main feed', () => {
    expect(filterMainList(items, '').map((a) => a.id)).not.toContain('pin');
  });

  it('sorts the remaining items newest-first', () => {
    expect(filterMainList(items, '').map((a) => a.id)).toEqual(['new', 'old']);
  });

  it('matches the search query against the TITLE only (no content body since #943)', () => {
    expect(filterMainList(items, 'garden').map((a) => a.id)).toEqual(['new']);
    // Case-insensitive.
    expect(filterMainList(items, 'LIFT').map((a) => a.id)).toEqual(['old']);
    // A term that matched the old `content` body must NOT match now.
    expect(filterMainList(items, 'no-such-title')).toEqual([]);
  });
});

// Pure helpers extracted from the component body during the churn-hotspot
// refactor. These used to live inline (untested); pinning them here prevents
// silent regression of read-state overlay, unread counting, and the feed's
// relative-date label.

describe('applyReadState (client-side read overlay)', () => {
  const base = ui({ id: 'a', isRead: false });

  it('marks an unread row read when its id is in the set', () => {
    const out = applyReadState(base, new Set(['a']));
    expect(out.isRead).toBe(true);
    expect(out).not.toBe(base); // new object — does not mutate input
  });

  it('returns the same reference when the id is absent', () => {
    const out = applyReadState(base, new Set(['other']));
    expect(out).toBe(base);
  });

  it('returns the same reference when already read', () => {
    const read = ui({ id: 'a', isRead: true });
    expect(applyReadState(read, new Set(['a']))).toBe(read);
  });
});

describe('buildAnnouncements (response → read-aware UI list)', () => {
  it('maps the published list and overlays read state', () => {
    const resp = { announcements: [summary] }; // summary.id === 'a-1'
    const out = buildAnnouncements(resp, new Set(['a-1']));
    expect(out).toHaveLength(1);
    expect(out[0]).toMatchObject({ id: 'a-1', isRead: true, isPinned: true });
  });

  it('returns [] for an undefined response', () => {
    expect(buildAnnouncements(undefined, new Set())).toEqual([]);
  });

  it('leaves rows unread when their id is not in the set', () => {
    const out = buildAnnouncements({ announcements: [summary] }, new Set());
    expect(out[0].isRead).toBe(false);
  });
});

describe('countUnread', () => {
  it('counts rows whose isRead is false', () => {
    const items = [
      ui({ id: 'a', isRead: false }),
      ui({ id: 'b', isRead: true }),
      ui({ id: 'c', isRead: false }),
    ];
    expect(countUnread(items)).toBe(2);
  });

  it('is 0 for an empty list', () => {
    expect(countUnread([])).toBe(0);
  });
});

describe('formatRelativeDate (feed card label)', () => {
  const now = new Date('2026-05-20T12:00:00Z');

  it('returns "Just now" under one hour', () => {
    expect(formatRelativeDate('2026-05-20T11:30:00Z', now)).toBe('Just now');
  });

  it('returns "Nh ago" within the same day', () => {
    expect(formatRelativeDate('2026-05-20T09:00:00Z', now)).toBe('3h ago');
  });

  it('returns "Nd ago" within the week', () => {
    expect(formatRelativeDate('2026-05-18T12:00:00Z', now)).toBe('2d ago');
  });

  it('returns an absolute month-day label beyond a week', () => {
    // 2026-05-01 is >7d before now → absolute "May 1" style label.
    expect(formatRelativeDate('2026-05-01T12:00:00Z', now)).toBe('May 1');
  });
});
