import { type ApiAnnouncementSummary, extractItems, toUiAnnouncement } from './AnnouncementsScreen';

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
