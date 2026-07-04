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

// Context for future maintainers (kept terse; the assertions, not the prose,
// are the contract):
//
// - The screen reads the RLS-scoped `GET /api/v1/announcements/published`
//   endpoint, whose `AnnouncementSummary` rows are lightweight: no
//   `content` body, no per-user read flag, no author/category. The list is
//   carried under the `announcements` key (NOT the legacy `items` key).
// - The pinned sticky band and the main feed are both derived client-side
//   from that single published list. Read state is overlaid from a
//   client-side `Set<id>`.

// --- Test fixtures -------------------------------------------------------

/** A representative published-summary row (pinned, no content body). */
const SUMMARY: ApiAnnouncementSummary = {
  id: 'a-1',
  title: 'Lift maintenance Friday',
  status: 'published',
  target_type: 'building',
  published_at: '2026-05-20T08:00:00Z',
  pinned: true,
  comments_enabled: true,
  acknowledgment_required: false,
};

/** Build a UI `Announcement` with sensible defaults; override per test. */
const makeAnnouncement = (
  over: Partial<Announcement> & Pick<Announcement, 'id'>
): Announcement => ({
  title: over.id,
  createdAt: '2026-05-01T00:00:00Z',
  author: 'Building Management',
  isRead: false,
  isPinned: false,
  attachments: [],
  commentsCount: 0,
  ...over,
});

// --- Mapping: summary row -> UI announcement -----------------------------

describe('toUiAnnouncement', () => {
  it('maps a lightweight summary row (no content body) to the UI shape', () => {
    expect(toUiAnnouncement(SUMMARY)).toMatchObject({
      id: 'a-1',
      title: 'Lift maintenance Friday',
      createdAt: '2026-05-20T08:00:00Z', // from published_at
      isPinned: true,
    });
  });

  it('does not invent a content body the summary never carried', () => {
    expect(toUiAnnouncement(SUMMARY)).not.toHaveProperty('content');
  });

  it('falls back to a non-empty createdAt when published_at is null', () => {
    const ui = toUiAnnouncement({ ...SUMMARY, published_at: null });
    expect(typeof ui.createdAt).toBe('string');
    expect(ui.createdAt.length).toBeGreaterThan(0);
  });
});

// --- List extraction -----------------------------------------------------

describe('extractItems', () => {
  it('returns the `announcements` array from the published-list response', () => {
    expect(extractItems({ announcements: [SUMMARY], count: 1 })).toEqual([SUMMARY]);
  });

  it('returns [] for undefined or empty responses', () => {
    expect(extractItems(undefined)).toEqual([]);
    expect(extractItems({})).toEqual([]);
  });

  it('ignores a stray legacy `items` key (not part of the contract)', () => {
    const legacy = { items: [SUMMARY] } as unknown as Parameters<typeof extractItems>[0];
    expect(extractItems(legacy)).toEqual([]);
  });
});

// --- Client-side pinned band ---------------------------------------------

describe('derivePinnedItems', () => {
  it('keeps only pinned rows, in order', () => {
    const items = [
      makeAnnouncement({ id: 'a', isPinned: true }),
      makeAnnouncement({ id: 'b', isPinned: false }),
      makeAnnouncement({ id: 'c', isPinned: true }),
    ];
    expect(derivePinnedItems(items).map((a) => a.id)).toEqual(['a', 'c']);
  });

  it('returns [] when nothing is pinned', () => {
    const items = [makeAnnouncement({ id: 'a' }), makeAnnouncement({ id: 'b' })];
    expect(derivePinnedItems(items)).toEqual([]);
  });
});

// --- Main feed partitioning ----------------------------------------------

describe('filterMainList', () => {
  const items = [
    makeAnnouncement({ id: 'pin', isPinned: true, createdAt: '2026-05-05T00:00:00Z' }),
    makeAnnouncement({ id: 'old', title: 'Lift maintenance', createdAt: '2026-05-01T00:00:00Z' }),
    makeAnnouncement({ id: 'new', title: 'Garden event', createdAt: '2026-05-03T00:00:00Z' }),
  ];

  it('excludes pinned items (they live in the sticky band)', () => {
    expect(filterMainList(items, '').map((a) => a.id)).not.toContain('pin');
  });

  it('sorts the remaining items newest-first', () => {
    expect(filterMainList(items, '').map((a) => a.id)).toEqual(['new', 'old']);
  });

  it('matches the search query against the title, case-insensitively', () => {
    expect(filterMainList(items, 'garden').map((a) => a.id)).toEqual(['new']);
    expect(filterMainList(items, 'LIFT').map((a) => a.id)).toEqual(['old']);
  });

  it('does not match terms absent from any title (no content-body search)', () => {
    expect(filterMainList(items, 'no-such-title')).toEqual([]);
  });
});

// --- Client-side read overlay --------------------------------------------

describe('applyReadState', () => {
  const unread = makeAnnouncement({ id: 'a', isRead: false });

  it('marks an unread row read when its id is in the set (without mutating)', () => {
    const out = applyReadState(unread, new Set(['a']));
    expect(out.isRead).toBe(true);
    expect(out).not.toBe(unread);
  });

  it('returns the same reference when the id is absent', () => {
    expect(applyReadState(unread, new Set(['other']))).toBe(unread);
  });

  it('returns the same reference when the row is already read', () => {
    const read = makeAnnouncement({ id: 'a', isRead: true });
    expect(applyReadState(read, new Set(['a']))).toBe(read);
  });
});

describe('buildAnnouncements', () => {
  it('maps the published list and overlays read state', () => {
    const out = buildAnnouncements({ announcements: [SUMMARY] }, new Set(['a-1']));
    expect(out).toHaveLength(1);
    expect(out[0]).toMatchObject({ id: 'a-1', isRead: true, isPinned: true });
  });

  it('returns [] for an undefined response', () => {
    expect(buildAnnouncements(undefined, new Set())).toEqual([]);
  });

  it('leaves rows unread when their id is not in the set', () => {
    const out = buildAnnouncements({ announcements: [SUMMARY] }, new Set());
    expect(out[0].isRead).toBe(false);
  });
});

describe('countUnread', () => {
  it('counts rows whose isRead is false', () => {
    const items = [
      makeAnnouncement({ id: 'a', isRead: false }),
      makeAnnouncement({ id: 'b', isRead: true }),
      makeAnnouncement({ id: 'c', isRead: false }),
    ];
    expect(countUnread(items)).toBe(2);
  });

  it('is 0 for an empty list', () => {
    expect(countUnread([])).toBe(0);
  });
});

// --- Feed-card relative date label ---------------------------------------

describe('formatRelativeDate', () => {
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

  // Boundary transitions between the three branches. The interior cases above
  // ("Just now", "3h ago", "2d ago", absolute) never cross a `<` threshold, so
  // an off-by-one flip (`<` vs `<=`) on any boundary would slip past them. Pin
  // each transition explicitly -- this is the branchiest function in the file
  // and the documented relative-date churn source (see jest.config.js).

  it('flips from "Just now" to "Nh ago" at exactly one hour', () => {
    // diffHours === 1 is NOT `< 1`, so it must render as the (unpluralised)
    // "1h ago" label rather than staying "Just now".
    expect(formatRelativeDate('2026-05-20T11:00:00Z', now)).toBe('1h ago');
  });

  it('flips from "Nh ago" to "Nd ago" at exactly 24 hours', () => {
    // diffHours === 24 is NOT `< 24`, so it drops to the day branch: "1d ago"
    // (unpluralised), never "24h ago".
    expect(formatRelativeDate('2026-05-19T12:00:00Z', now)).toBe('1d ago');
  });

  it('flips from "Nd ago" to an absolute label at exactly seven days', () => {
    // diffDays === 7 is NOT `< 7`, so it must render the absolute "Mon D" date
    // rather than "7d ago". Derived under the pinned UTC zone (see jest.config.js).
    const sevenDays = '2026-05-13T12:00:00Z';
    const expected = new Date(sevenDays).toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      timeZone: 'UTC',
    });
    expect(expected).toMatch(/^[A-Z][a-z]{2} \d{1,2}$/); // guard the "Mon D" shape
    expect(formatRelativeDate(sevenDays, now)).toBe(expected);
  });

  it('returns an absolute month-day label beyond a week', () => {
    // Beyond a week the formatter switches from a relative ("Nd ago") label to
    // an absolute "Mon D" date produced by `toLocaleDateString('en-US', …)`.
    // The exact rendering is timezone-dependent, so the test worker pins TZ=UTC
    // (see jest.config.js); we derive the expected label from that same UTC
    // contract rather than hard-coding it, so the assertion can't drift if the
    // pinned zone ever changes -- while still proving an absolute (not relative)
    // label is returned via the `Mon D` shape check. Under the pinned UTC zone
    // this date renders as "May 1" (the value dev asserted literally).
    const old = '2026-05-01T12:00:00Z';
    const expected = new Date(old).toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      timeZone: 'UTC',
    });
    expect(expected).toMatch(/^[A-Z][a-z]{2} \d{1,2}$/); // guard the "Mon D" shape
    expect(formatRelativeDate(old, now)).toBe(expected);
  });
});
