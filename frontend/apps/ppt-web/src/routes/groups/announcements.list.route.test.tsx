/**
 * Announcements route group — AnnouncementsPageRoute API wiring tests (Story 79.1).
 *
 * Sibling of `faults.route.test.tsx`: the faults *list* route already had a
 * wired-data-path test, but the announcements *list* route did not — only the
 * announcement-detail wiring was covered (`announcements.route.test.tsx`). This
 * file closes that coverage gap by pinning the contract between the live
 * `@ppt/api-client` hooks and the `AnnouncementsPage` list component:
 *
 *  (a) loading state propagates (useAnnouncements.isLoading) → empty state absent
 *  (b) error state propagates → inline alert rendered + retry button calls refetch
 *  (c) successful data renders an announcement title (item → AnnouncementCard)
 *  (d) empty list (no error) → "No announcements found." empty state
 *  (e) pinned-band query (usePinnedAnnouncements) feeds the sticky band
 *
 * Renders the real route via MemoryRouter + Routes + TanStack
 * QueryClientProvider, exactly as in production. A dedicated test file (rather
 * than extending `announcements.route.test.tsx`) is required because that file's
 * module-level `vi.mock('@ppt/api-client')` is shaped for the detail view.
 */

/// <reference types="vitest/globals" />
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Suspense } from 'react';
import { MemoryRouter, Routes } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';

// ── i18n ──
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: { defaultValue?: string }) => opts?.defaultValue ?? key,
  }),
}));

// ── toast ──
const mockShowToast = vi.fn();
vi.mock('../../components', () => ({
  useToast: () => ({ showToast: mockShowToast }),
}));

// ── auth ──
vi.mock('../../contexts', () => ({
  useAuth: () => ({ user: { role: 'manager' } }),
}));

// ── api-client stubs ──
const mockUseAnnouncements = vi.fn();
const mockUsePinnedAnnouncements = vi.fn();
vi.mock('@ppt/api-client', () => ({
  useAnnouncements: (...args: unknown[]) => mockUseAnnouncements(...args),
  usePinnedAnnouncements: (...args: unknown[]) => mockUsePinnedAnnouncements(...args),
  useDeleteAnnouncement: () => ({ mutateAsync: vi.fn() }),
  usePublishAnnouncement: () => ({ mutateAsync: vi.fn() }),
  useArchiveAnnouncement: () => ({ mutateAsync: vi.fn() }),
  usePinAnnouncement: () => ({ mutateAsync: vi.fn() }),
  // The other announcement routes (create/view/edit) import these at module load.
  useAnnouncement: () => ({ data: undefined, isLoading: true, error: null }),
  useCreateAnnouncement: () => ({ mutateAsync: vi.fn(), isPending: false }),
  useUpdateAnnouncement: () => ({ mutateAsync: vi.fn(), isPending: false }),
  useMarkReadAnnouncement: () => ({ mutate: vi.fn(), mutateAsync: vi.fn() }),
  useAcknowledgeAnnouncement: () => ({ mutateAsync: vi.fn() }),
  useAnnouncementAcknowledgmentStats: () => ({ data: undefined }),
  useAnnouncementComments: () => ({ data: undefined, isLoading: false }),
  useCreateAnnouncementComment: () => ({ mutateAsync: vi.fn() }),
  useDeleteAnnouncementComment: () => ({ mutateAsync: vi.fn() }),
}));

// Import the route group AFTER mocks are in place.
import { announcementRoutes } from './announcements';

const noopRefetch = vi.fn().mockResolvedValue(undefined);

function makeSummary(overrides: Record<string, unknown> = {}) {
  return {
    id: 'ann-1',
    title: 'Roof repair scheduled',
    status: 'published',
    targetType: 'all',
    pinned: false,
    authorName: 'Jane Manager',
    createdAt: '2026-06-01T00:00:00Z',
    publishedAt: '2026-06-01T00:00:00Z',
    readCount: 0,
    commentCount: 0,
    ...overrides,
  };
}

function renderAnnouncementsRoute() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <MemoryRouter initialEntries={['/announcements']}>
        <Suspense fallback={<div>loading…</div>}>
          <Routes>{announcementRoutes()}</Routes>
        </Suspense>
      </MemoryRouter>
    </QueryClientProvider>
  );
}

describe('AnnouncementsPageRoute API wiring (Story 79.1)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Default: pinned band empty unless a test overrides it.
    mockUsePinnedAnnouncements.mockReturnValue({ data: { items: [] } });
  });

  it('(a) does not show empty state while loading', async () => {
    mockUseAnnouncements.mockReturnValue({
      data: undefined,
      isLoading: true,
      error: null,
      refetch: noopRefetch,
    });

    renderAnnouncementsRoute();
    // The "Create Announcement" button is rendered by AnnouncementList regardless
    // of loading/error state; wait for the lazy chunk to resolve via it.
    await screen.findByRole('button', { name: 'Create Announcement' }, { timeout: 3000 });

    expect(screen.queryByText('No announcements found.')).not.toBeInTheDocument();
  });

  it('(b) shows error alert when useAnnouncements errors, retry calls refetch', async () => {
    mockUseAnnouncements.mockReturnValue({
      data: undefined,
      isLoading: false,
      error: new Error('network error'),
      refetch: noopRefetch,
    });

    renderAnnouncementsRoute();

    const alert = await screen.findByRole('alert', {}, { timeout: 5000 });
    expect(alert).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Retry' }));
    expect(noopRefetch).toHaveBeenCalledTimes(1);
  });

  it('(c) renders the announcement title when useAnnouncements returns data', async () => {
    mockUseAnnouncements.mockReturnValue({
      data: { items: [makeSummary({ title: 'Lift maintenance Friday' })], total: 1 },
      isLoading: false,
      error: null,
      refetch: noopRefetch,
    });

    renderAnnouncementsRoute();

    expect(
      await screen.findByText('Lift maintenance Friday', {}, { timeout: 5000 })
    ).toBeInTheDocument();
  });

  it('(d) shows empty state when list is empty and no error', async () => {
    mockUseAnnouncements.mockReturnValue({
      data: { items: [], total: 0 },
      isLoading: false,
      error: null,
      refetch: noopRefetch,
    });

    renderAnnouncementsRoute();

    expect(
      await screen.findByText('No announcements found.', {}, { timeout: 5000 })
    ).toBeInTheDocument();
  });

  it('(e) feeds the pinned-band query result into the sticky band', async () => {
    mockUseAnnouncements.mockReturnValue({
      data: { items: [], total: 0 },
      isLoading: false,
      error: null,
      refetch: noopRefetch,
    });
    mockUsePinnedAnnouncements.mockReturnValue({
      data: { items: [makeSummary({ id: 'pin-1', title: 'Water shutoff notice', pinned: true })] },
    });

    renderAnnouncementsRoute();

    expect(
      await screen.findByText('Water shutoff notice', {}, { timeout: 5000 })
    ).toBeInTheDocument();
    // The list query result is independent of the pinned-band query.
    expect(mockUsePinnedAnnouncements).toHaveBeenCalled();
  });
});
