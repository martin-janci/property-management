/**
 * PinnedAnnouncementsBand tests (#516 — IG3 follow-up for PR #511).
 *
 * Verifies: render-when-empty short-circuit, chip count, click handler.
 */
/// <reference types="vitest/globals" />

import type { AnnouncementSummary } from '@ppt/api-client';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { PinnedAnnouncementsBand } from './PinnedAnnouncementsBand';

function ann(id: string, title: string, ack = false): AnnouncementSummary {
  return {
    id,
    title,
    summary: '',
    severity: 'info',
    status: 'published',
    targetType: 'building',
    authorId: 'a1',
    isPinned: true,
    acknowledgmentRequired: ack,
    createdAt: '2026-05-26T00:00:00Z',
    updatedAt: '2026-05-26T00:00:00Z',
  } as unknown as AnnouncementSummary;
}

describe('PinnedAnnouncementsBand', () => {
  it('renders nothing when there are no items', () => {
    const { container } = render(<PinnedAnnouncementsBand announcements={[]} onView={vi.fn()} />);
    expect(container).toBeEmptyDOMElement();
  });

  it('renders one chip per item', () => {
    render(
      <PinnedAnnouncementsBand
        announcements={[ann('1', 'Fire alarm test'), ann('2', 'Elevator out')]}
        onView={vi.fn()}
      />
    );
    expect(screen.getByText('Fire alarm test')).toBeInTheDocument();
    expect(screen.getByText('Elevator out')).toBeInTheDocument();
  });

  it('shows the ack indicator only on items requiring acknowledgment', () => {
    render(
      <PinnedAnnouncementsBand
        announcements={[ann('1', 'A', true), ann('2', 'B', false)]}
        onView={vi.fn()}
      />
    );
    const acks = screen.getAllByLabelText('Acknowledgment required');
    expect(acks).toHaveLength(1);
  });

  it('calls onView with the announcement id when a chip is clicked', async () => {
    const onView = vi.fn();
    render(
      <PinnedAnnouncementsBand announcements={[ann('abc-123', 'Click me')]} onView={onView} />
    );
    await userEvent.click(screen.getByText('Click me'));
    expect(onView).toHaveBeenCalledWith('abc-123');
  });
});
