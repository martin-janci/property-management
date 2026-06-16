/**
 * AnnouncementList pin/unpin UI tests (feat-6-4 — Story 6.4 pin/unpin web UI).
 *
 * The presentational `PinnedAnnouncementsBand` is covered in isolation by
 * `PinnedAnnouncementsBand.test.tsx`. The previously-untested layer was the
 * *integration* in `AnnouncementList` — the glue that:
 *
 *  - threads `pinnedAnnouncements` into the sticky band above the list (and
 *    keeps that band immune to the main list, which carries every status);
 *  - renders one pin/unpin toggle per card and wires it to `onPin(id, next)`
 *    where `next` is the negation of the card's current `pinned` flag;
 *  - shows the "Unpin" affordance on already-pinned rows and "Pin" otherwise.
 *
 * These pin the Story 6.4 pin/unpin contract at the list level.
 */

/// <reference types="vitest/globals" />

import type { AnnouncementSummary } from '@ppt/api-client';
import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { AnnouncementList } from './AnnouncementList';

function ann(overrides: Partial<AnnouncementSummary> = {}): AnnouncementSummary {
  return {
    id: 'ann-1',
    title: 'Untitled',
    status: 'published',
    targetType: 'all',
    pinned: false,
    commentsEnabled: false,
    acknowledgmentRequired: false,
    publishedAt: '2026-05-26T00:00:00Z',
    ...overrides,
  } as AnnouncementSummary;
}

const baseProps = {
  total: 0,
  page: 1,
  pageSize: 10,
  onPageChange: vi.fn(),
  onStatusFilter: vi.fn(),
  onTargetTypeFilter: vi.fn(),
  onView: vi.fn(),
  onEdit: vi.fn(),
  onDelete: vi.fn(),
  onPublish: vi.fn(),
  onArchive: vi.fn(),
  onPin: vi.fn(),
  onCreate: vi.fn(),
};

describe('AnnouncementList — pin/unpin UI (Story 6.4)', () => {
  it('renders the pinned band from pinnedAnnouncements, separate from the main list', () => {
    render(
      <AnnouncementList
        {...baseProps}
        announcements={[ann({ id: 'list-1', title: 'In the list' })]}
        total={1}
        pinnedAnnouncements={[ann({ id: 'pin-1', title: 'Pinned chip', pinned: true })]}
      />
    );

    const band = screen.getByRole('region', { name: 'Pinned announcements' });
    expect(within(band).getByText('Pinned chip')).toBeInTheDocument();
    // The list card title lives outside the band.
    expect(within(band).queryByText('In the list')).not.toBeInTheDocument();
    expect(screen.getByText('In the list')).toBeInTheDocument();
  });

  it('does not render the pinned band when there are no pinned announcements', () => {
    render(
      <AnnouncementList
        {...baseProps}
        announcements={[ann({ id: 'list-1', title: 'Only item' })]}
        total={1}
      />
    );

    expect(screen.queryByRole('region', { name: 'Pinned announcements' })).not.toBeInTheDocument();
  });

  it('shows "Pin" on an unpinned card and pins it via onPin(id, true)', async () => {
    const onPin = vi.fn();
    render(
      <AnnouncementList
        {...baseProps}
        onPin={onPin}
        announcements={[ann({ id: 'unpinned-1', title: 'Not pinned', pinned: false })]}
        total={1}
      />
    );

    await userEvent.click(screen.getByRole('button', { name: 'Pin' }));
    expect(onPin).toHaveBeenCalledTimes(1);
    expect(onPin).toHaveBeenCalledWith('unpinned-1', true);
  });

  it('shows "Unpin" on a pinned card and unpins it via onPin(id, false)', async () => {
    const onPin = vi.fn();
    render(
      <AnnouncementList
        {...baseProps}
        onPin={onPin}
        announcements={[ann({ id: 'pinned-1', title: 'Already pinned', pinned: true })]}
        total={1}
      />
    );

    await userEvent.click(screen.getByRole('button', { name: 'Unpin' }));
    expect(onPin).toHaveBeenCalledTimes(1);
    expect(onPin).toHaveBeenCalledWith('pinned-1', false);
  });

  it('renders an independent pin toggle per card, each carrying its own id and next-state', async () => {
    const onPin = vi.fn();
    render(
      <AnnouncementList
        {...baseProps}
        onPin={onPin}
        announcements={[
          ann({ id: 'a', title: 'Alpha', pinned: false }),
          ann({ id: 'b', title: 'Bravo', pinned: true }),
        ]}
        total={2}
      />
    );

    // One toggle reads "Pin" (for the unpinned row) and one reads "Unpin".
    await userEvent.click(screen.getByRole('button', { name: 'Pin' }));
    await userEvent.click(screen.getByRole('button', { name: 'Unpin' }));

    expect(onPin).toHaveBeenNthCalledWith(1, 'a', true);
    expect(onPin).toHaveBeenNthCalledWith(2, 'b', false);
  });
});

describe('AnnouncementList — pinned ordering (Story 6.4)', () => {
  /** Titles of the rendered cards, in document order. */
  function renderedTitlesInOrder(): string[] {
    return screen.getAllByRole('heading', { level: 3 }).map((h) => h.textContent?.trim() ?? '');
  }

  it('floats pinned announcements to the top of the list, regardless of input order', () => {
    render(
      <AnnouncementList
        {...baseProps}
        announcements={[
          ann({ id: 'u1', title: 'Unpinned A', pinned: false }),
          ann({ id: 'p1', title: 'Pinned A', pinned: true }),
          ann({ id: 'u2', title: 'Unpinned B', pinned: false }),
          ann({ id: 'p2', title: 'Pinned B', pinned: true }),
        ]}
        total={4}
      />
    );

    const titles = renderedTitlesInOrder();
    // Both pinned rows precede both unpinned rows.
    expect(titles.indexOf('Pinned A')).toBeLessThan(titles.indexOf('Unpinned A'));
    expect(titles.indexOf('Pinned A')).toBeLessThan(titles.indexOf('Unpinned B'));
    expect(titles.indexOf('Pinned B')).toBeLessThan(titles.indexOf('Unpinned A'));
    expect(titles.indexOf('Pinned B')).toBeLessThan(titles.indexOf('Unpinned B'));
  });

  it('keeps the server-provided order stable among rows sharing a pinned state', () => {
    render(
      <AnnouncementList
        {...baseProps}
        announcements={[
          ann({ id: 'p1', title: 'Pinned first', pinned: true }),
          ann({ id: 'p2', title: 'Pinned second', pinned: true }),
          ann({ id: 'u1', title: 'Unpinned first', pinned: false }),
          ann({ id: 'u2', title: 'Unpinned second', pinned: false }),
        ]}
        total={4}
      />
    );

    expect(renderedTitlesInOrder()).toEqual([
      'Pinned first',
      'Pinned second',
      'Unpinned first',
      'Unpinned second',
    ]);
  });
});
