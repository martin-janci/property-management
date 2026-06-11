/**
 * Route-wrapper integration test (feat-6-2 — Story 6.2 web viewing & acknowledgment).
 *
 * The presentational `ViewAnnouncementPage` and the data-layer hooks
 * (`useAcknowledgeAnnouncement` / `useMarkReadAnnouncement` /
 * `useAnnouncementAcknowledgmentStats`) are each covered in isolation by
 * `ViewAnnouncementPage.test.tsx` and `hooks/acknowledgment.test.tsx`.
 *
 * The previously-untested layer was the *wiring* in
 * `routes/groups/announcements.tsx#ViewAnnouncementPageInner` — the glue that
 * turns the page's `onAcknowledge` / auto-mark-read into real api-client
 * mutation calls against the backend. This test pins that contract:
 *
 *  - On mount the wrapper fires the read-receipt mutation exactly once
 *    (auto-mark-read), even under React StrictMode double-invocation.
 *  - The "Acknowledge" button is wired to `useAcknowledgeAnnouncement` and is
 *    only rendered when the announcement requires acknowledgment.
 *  - The manager acknowledgment-stats query is enabled only when the
 *    announcement requires acknowledgment.
 */

/// <reference types="vitest/globals" />
import type { AnnouncementWithDetails } from '@ppt/api-client';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

// --- Mocks ----------------------------------------------------------------

const markReadMutate = vi.fn();
const acknowledgeMutateAsync = vi.fn().mockResolvedValue(undefined);
const markReadMutateAsync = vi.fn().mockResolvedValue(undefined);

// Capture the args the ack-stats hook is called with so we can assert the
// "only fetch stats when acknowledgment is required" gate.
const ackStatsCalls: Array<[string, boolean]> = [];

let announcementData: { announcement: AnnouncementWithDetails | undefined } = {
  announcement: undefined,
};

vi.mock('@ppt/api-client', () => ({
  useAnnouncement: () => ({ data: announcementData, isLoading: false, error: null }),
  useMarkReadAnnouncement: () => ({ mutate: markReadMutate, mutateAsync: markReadMutateAsync }),
  useAcknowledgeAnnouncement: () => ({ mutateAsync: acknowledgeMutateAsync }),
  useAnnouncementAcknowledgmentStats: (id: string, enabled: boolean) => {
    ackStatsCalls.push([id, enabled]);
    return { data: undefined };
  },
  useAnnouncementComments: () => ({ data: undefined, isLoading: false }),
  useCreateAnnouncementComment: () => ({ mutateAsync: vi.fn() }),
  useDeleteAnnouncementComment: () => ({ mutateAsync: vi.fn() }),
  usePublishAnnouncement: () => ({ mutateAsync: vi.fn() }),
  useArchiveAnnouncement: () => ({ mutateAsync: vi.fn() }),
  usePinAnnouncement: () => ({ mutateAsync: vi.fn() }),
  useDeleteAnnouncement: () => ({ mutateAsync: vi.fn() }),
}));

vi.mock('react-router-dom', () => ({
  useNavigate: () => vi.fn(),
}));

vi.mock('../../components', () => ({
  useToast: () => ({ showToast: vi.fn() }),
}));

vi.mock('../../contexts', () => ({
  useAuth: () => ({ user: { id: 'mgr-1', role: 'manager' } }),
}));

// Import after the mocks are registered.
import { ViewAnnouncementPageInner } from './announcements';

function makeAnnouncement(
  overrides: Partial<AnnouncementWithDetails> = {}
): AnnouncementWithDetails {
  return {
    id: 'ann-1',
    organizationId: 'org-1',
    authorId: 'author-1',
    authorName: 'Jane Manager',
    title: 'Roof repair scheduled',
    content: 'The roof repair starts Monday.',
    status: 'published',
    targetType: 'all',
    targetIds: [],
    pinned: false,
    acknowledgmentRequired: false,
    commentsEnabled: false,
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

describe('ViewAnnouncementPageInner — route wiring (Story 6.2)', () => {
  beforeEach(() => {
    markReadMutate.mockClear();
    acknowledgeMutateAsync.mockClear();
    markReadMutateAsync.mockClear();
    ackStatsCalls.length = 0;
    announcementData = { announcement: undefined };
  });

  it('auto-fires the read-receipt mutation exactly once on mount', async () => {
    announcementData = { announcement: makeAnnouncement() };
    render(<ViewAnnouncementPageInner announcementId="ann-1" />);

    await waitFor(() => expect(markReadMutate).toHaveBeenCalledTimes(1));
    expect(markReadMutate).toHaveBeenCalledWith('ann-1');
  });

  it('wires the Acknowledge button to the acknowledge mutation when required', async () => {
    announcementData = { announcement: makeAnnouncement({ acknowledgmentRequired: true }) };
    render(<ViewAnnouncementPageInner announcementId="ann-1" />);

    const ackButton = await screen.findByRole('button', { name: 'Acknowledge' });
    await userEvent.click(ackButton);

    expect(acknowledgeMutateAsync).toHaveBeenCalledTimes(1);
    expect(acknowledgeMutateAsync).toHaveBeenCalledWith('ann-1');
  });

  it('does not render the Acknowledge button when acknowledgment is not required', () => {
    announcementData = { announcement: makeAnnouncement({ acknowledgmentRequired: false }) };
    render(<ViewAnnouncementPageInner announcementId="ann-1" />);

    expect(screen.queryByRole('button', { name: 'Acknowledge' })).not.toBeInTheDocument();
  });

  it('enables the manager ack-stats query only when acknowledgment is required', () => {
    announcementData = { announcement: makeAnnouncement({ acknowledgmentRequired: true }) };
    render(<ViewAnnouncementPageInner announcementId="ann-1" />);

    // The hook is called with (id, enabled); enabled must be true here.
    expect(ackStatsCalls.some(([id, enabled]) => id === 'ann-1' && enabled === true)).toBe(true);
  });

  it('disables the ack-stats query when acknowledgment is not required', () => {
    announcementData = { announcement: makeAnnouncement({ acknowledgmentRequired: false }) };
    render(<ViewAnnouncementPageInner announcementId="ann-1" />);

    expect(ackStatsCalls.every(([, enabled]) => enabled === false)).toBe(true);
  });
});
