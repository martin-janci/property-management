/**
 * ViewAnnouncementPage tests (gap-6-2 — Story 6.2 web viewing & acknowledgment).
 *
 * Mirrors the shipped mobile AnnouncementDetailScreen behaviour on web:
 *  - "Mark as Read" wires to onMarkRead.
 *  - "Acknowledge" appears only when acknowledgmentRequired AND a handler is
 *    supplied, and wires to onAcknowledge.
 *  - The manager AcknowledgmentStats panel renders only when acknowledgment is
 *    required and stats are provided.
 */
/// <reference types="vitest/globals" />

import type { AcknowledgmentStats, AnnouncementWithDetails } from '@ppt/api-client';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { ViewAnnouncementPage } from './ViewAnnouncementPage';

function announcement(overrides: Partial<AnnouncementWithDetails> = {}): AnnouncementWithDetails {
  return {
    id: 'ann-1',
    organizationId: 'org-1',
    authorId: 'author-1',
    authorName: 'Jane Manager',
    title: 'Roof repair scheduled',
    content: 'The roof repair starts Monday.',
    targetType: 'all',
    targetIds: [],
    status: 'published',
    pinned: false,
    commentsEnabled: false,
    acknowledgmentRequired: false,
    readCount: 4,
    acknowledgedCount: 0,
    commentCount: 0,
    attachmentCount: 0,
    createdAt: '2026-06-01T00:00:00Z',
    updatedAt: '2026-06-01T00:00:00Z',
    publishedAt: '2026-06-01T00:00:00Z',
    ...overrides,
  };
}

const ackStats: AcknowledgmentStats = {
  announcementId: 'ann-1',
  totalTargeted: 10,
  readCount: 4,
  acknowledgedCount: 2,
  pendingCount: 6,
};

function noopHandlers() {
  return {
    onEdit: vi.fn(),
    onPublish: vi.fn(),
    onArchive: vi.fn(),
    onPin: vi.fn(),
    onDelete: vi.fn(),
    onBack: vi.fn(),
  };
}

describe('ViewAnnouncementPage', () => {
  it('renders the announcement title, author and read count', () => {
    render(
      <ViewAnnouncementPage announcement={announcement()} attachments={[]} {...noopHandlers()} />
    );

    expect(screen.getByText('Roof repair scheduled')).toBeInTheDocument();
    expect(screen.getByText(/Jane Manager/)).toBeInTheDocument();
    expect(screen.getByText('4')).toBeInTheDocument();
  });

  it('fires onMarkRead when the "Mark as Read" button is clicked', async () => {
    const onMarkRead = vi.fn();
    render(
      <ViewAnnouncementPage
        announcement={announcement()}
        attachments={[]}
        onMarkRead={onMarkRead}
        {...noopHandlers()}
      />
    );

    await userEvent.click(screen.getByRole('button', { name: 'Mark as Read' }));
    expect(onMarkRead).toHaveBeenCalledTimes(1);
  });

  it('hides the "Mark as Read" button when no handler is supplied', () => {
    render(
      <ViewAnnouncementPage announcement={announcement()} attachments={[]} {...noopHandlers()} />
    );

    expect(screen.queryByRole('button', { name: 'Mark as Read' })).not.toBeInTheDocument();
  });

  it('shows and wires the Acknowledge button when acknowledgment is required', async () => {
    const onAcknowledge = vi.fn();
    render(
      <ViewAnnouncementPage
        announcement={announcement({ acknowledgmentRequired: true })}
        attachments={[]}
        onAcknowledge={onAcknowledge}
        {...noopHandlers()}
      />
    );

    const btn = screen.getByRole('button', { name: 'Acknowledge' });
    await userEvent.click(btn);
    expect(onAcknowledge).toHaveBeenCalledTimes(1);
  });

  it('hides the Acknowledge button when acknowledgment is not required', () => {
    render(
      <ViewAnnouncementPage
        announcement={announcement({ acknowledgmentRequired: false })}
        attachments={[]}
        onAcknowledge={vi.fn()}
        {...noopHandlers()}
      />
    );

    expect(screen.queryByRole('button', { name: 'Acknowledge' })).not.toBeInTheDocument();
  });

  it('renders the manager AcknowledgmentStats panel only when required and stats are present', () => {
    const { rerender } = render(
      <ViewAnnouncementPage
        announcement={announcement({ acknowledgmentRequired: true })}
        attachments={[]}
        acknowledgmentStats={ackStats}
        {...noopHandlers()}
      />
    );

    expect(screen.getByText('Acknowledgment Status')).toBeInTheDocument();

    // No stats provided -> panel hidden even when ack is required.
    rerender(
      <ViewAnnouncementPage
        announcement={announcement({ acknowledgmentRequired: true })}
        attachments={[]}
        {...noopHandlers()}
      />
    );
    expect(screen.queryByText('Acknowledgment Status')).not.toBeInTheDocument();
  });

  it('shows a loading spinner instead of content while loading', () => {
    render(
      <ViewAnnouncementPage
        announcement={announcement()}
        attachments={[]}
        isLoading
        {...noopHandlers()}
      />
    );

    expect(screen.getByLabelText('Loading')).toBeInTheDocument();
    expect(screen.queryByText('Roof repair scheduled')).not.toBeInTheDocument();
  });
  it('renders the markdown body as formatted, sanitized HTML', () => {
    const { container } = render(
      <ViewAnnouncementPage
        announcement={announcement({
          content: '## Water outage\n\nPlease **store water**.\n\n- Item one\n\n<script>x</script>',
        })}
        attachments={[]}
        {...noopHandlers()}
      />
    );
    expect(screen.getByRole('heading', { level: 2, name: 'Water outage' })).toBeInTheDocument();
    expect(screen.getByText('store water').tagName).toBe('STRONG');
    expect(screen.getByText('Item one').tagName).toBe('LI');
    expect(container.querySelector('script')).toBeNull();
    expect(screen.queryByText(/## Water outage/)).not.toBeInTheDocument();
  });

  it('hides the author line when the author name is missing', () => {
    render(
      <ViewAnnouncementPage
        announcement={announcement({ authorName: '' })}
        attachments={[]}
        {...noopHandlers()}
      />
    );
    expect(screen.queryByText(/^By\b/)).not.toBeInTheDocument();
  });

  it('hides manager actions and read statistics from residents (canManage=false)', () => {
    render(
      <ViewAnnouncementPage
        announcement={announcement({ pinned: true, acknowledgmentRequired: true })}
        attachments={[]}
        onMarkRead={vi.fn()}
        onAcknowledge={vi.fn()}
        acknowledgmentStats={ackStats}
        canManage={false}
        {...noopHandlers()}
      />
    );
    for (const name of [/^Archive$/, /^Unpin$/, /^Pin$/, /^Edit$/, /^Delete$/, /^Publish Now$/]) {
      expect(screen.queryByRole('button', { name })).not.toBeInTheDocument();
    }
    expect(screen.queryByText('Acknowledgment Status')).not.toBeInTheDocument();
    expect(screen.queryByText('4')).not.toBeInTheDocument();
    // Resident-facing actions remain.
    expect(screen.getByRole('button', { name: 'Acknowledge' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Mark as Read' })).toBeInTheDocument();
  });

  it('shows manager actions by default', () => {
    render(
      <ViewAnnouncementPage
        announcement={announcement({ pinned: true })}
        attachments={[]}
        {...noopHandlers()}
      />
    );
    expect(screen.getByRole('button', { name: 'Archive' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Unpin' })).toBeInTheDocument();
  });
});
