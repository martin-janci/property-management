/**
 * FeedImport Component Tests
 *
 * - FeedCard: a failed pause-toggle or delete used to be indistinguishable
 *   from success. These tests ensure the failure is surfaced inline
 *   (role="alert"), and that a successful action shows no error.
 * - AddFeedModal: the add-feed wizard swallowed create mutateAsync rejections
 *   and hung with no user-facing feedback (mirrors the RealtorManagement
 *   invite-modal fix). These tests ensure the error is surfaced inline and the
 *   wizard stays open on failure.
 */

import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

// Mock @ppt/reality-api-client before importing the component.
vi.mock('@ppt/reality-api-client', () => ({
  useMyAgency: vi.fn(() => ({ data: { id: 'agency-1' } })),
  useFeedSources: vi.fn(),
  useDeleteFeedSource: vi.fn(),
  useSyncFeedSource: vi.fn(() => ({ mutate: vi.fn(), isPending: false })),
  useUpdateFeedSource: vi.fn(),
  useFeedSyncHistory: vi.fn(() => ({ data: undefined })),
  useCreateFeedSource: vi.fn(() => ({ mutateAsync: vi.fn(), isPending: false })),
  useFeedPreview: vi.fn(() => ({ mutateAsync: vi.fn(), isPending: false, error: null })),
}));

import {
  useCreateFeedSource,
  useDeleteFeedSource,
  useFeedPreview,
  useFeedSources,
  useUpdateFeedSource,
} from '@ppt/reality-api-client';
import { AddFeedModal, FeedImport } from './FeedImport';

const mockUseFeedSources = vi.mocked(useFeedSources);
const mockUseUpdateFeedSource = vi.mocked(useUpdateFeedSource);
const mockUseDeleteFeedSource = vi.mocked(useDeleteFeedSource);
const mockUseCreateFeedSource = vi.mocked(useCreateFeedSource);
const mockUseFeedPreview = vi.mocked(useFeedPreview);

const activeFeed = {
  id: 'feed-1',
  name: 'Test Feed',
  url: 'https://example.com/feed.xml',
  format: 'xml',
  status: 'active',
  totalListings: 10,
  syncFrequency: 'daily',
  lastFetchAt: null,
};

function setMutations({
  updateReject = false,
  deleteReject = false,
}: {
  updateReject?: boolean;
  deleteReject?: boolean;
} = {}) {
  mockUseFeedSources.mockReturnValue({
    data: [activeFeed],
    isLoading: false,
  } as unknown as ReturnType<typeof useFeedSources>);

  mockUseUpdateFeedSource.mockReturnValue({
    mutateAsync: updateReject
      ? vi.fn().mockRejectedValue(new Error('Network error'))
      : vi.fn().mockResolvedValue({}),
    isPending: false,
  } as unknown as ReturnType<typeof useUpdateFeedSource>);

  mockUseDeleteFeedSource.mockReturnValue({
    mutateAsync: deleteReject
      ? vi.fn().mockRejectedValue(new Error('Network error'))
      : vi.fn().mockResolvedValue({}),
    isPending: false,
  } as unknown as ReturnType<typeof useDeleteFeedSource>);
}

describe('FeedCard — mutation error handling', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('surfaces an inline error when the pause toggle fails', async () => {
    setMutations({ updateReject: true });
    render(<FeedImport />);

    fireEvent.click(screen.getByRole('button', { name: 'pause' }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
    });
    expect(screen.getByRole('alert')).toHaveTextContent('updateError');
  });

  it('shows no error when the pause toggle succeeds', async () => {
    setMutations({ updateReject: false });
    render(<FeedImport />);

    fireEvent.click(screen.getByRole('button', { name: 'pause' }));

    // Give any rejected promise a chance to settle before asserting absence.
    await Promise.resolve();
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('surfaces an inline error when the delete fails (after confirm)', async () => {
    setMutations({ deleteReject: true });
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);
    render(<FeedImport />);

    fireEvent.click(screen.getByRole('button', { name: 'remove' }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
    });
    expect(screen.getByRole('alert')).toHaveTextContent('removeError');
    confirmSpy.mockRestore();
  });

  it('does not attempt delete when the confirm is declined', () => {
    setMutations({ deleteReject: true });
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(false);
    render(<FeedImport />);

    fireEvent.click(screen.getByRole('button', { name: 'remove' }));

    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    confirmSpy.mockRestore();
  });
});

// Drive the wizard from the URL step through to the mapping step, where the
// "Create Feed" button lives.
async function advanceToMappingStep() {
  // Step "url": provide a feed URL and fetch a preview.
  fireEvent.change(screen.getByLabelText(/feedUrl/i), {
    target: { value: 'https://example.com/feed.xml' },
  });
  fireEvent.click(screen.getByRole('button', { name: /fetchFeed/i }));
  // A successful preview advances to the "preview" step.
  await waitFor(() => {
    expect(screen.getByRole('button', { name: /configureMapping/i })).toBeInTheDocument();
  });
  // Step "preview" -> "mapping".
  fireEvent.click(screen.getByRole('button', { name: /configureMapping/i }));
  await waitFor(() => {
    expect(screen.getByRole('button', { name: /createFeed/i })).toBeInTheDocument();
  });
}

describe('AddFeedModal — create error handling', () => {
  const agencyId = 'agency-1';
  const onClose = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    // A feed preview always succeeds so the wizard can reach the mapping step.
    mockUseFeedPreview.mockReturnValue({
      mutateAsync: vi.fn().mockResolvedValue({
        success: true,
        format: 'xml',
        totalItems: 2,
        availableFields: ['title', 'price'],
        sampleItems: [{ title: 'Sample' }],
      }),
      isPending: false,
      error: null,
    } as unknown as ReturnType<typeof useFeedPreview>);
  });

  function renderModal() {
    render(<AddFeedModal agencyId={agencyId} onClose={onClose} />);
  }

  it('shows an inline error when the create mutation fails', async () => {
    mockUseCreateFeedSource.mockReturnValue({
      mutateAsync: vi.fn().mockRejectedValue(new Error('Network error')),
      isPending: false,
    } as unknown as ReturnType<typeof useCreateFeedSource>);

    renderModal();
    await advanceToMappingStep();

    fireEvent.click(screen.getByRole('button', { name: /createFeed/i }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
    });
    expect(screen.getByRole('alert')).toHaveTextContent('createError');
  });

  it('keeps the wizard open when the create mutation fails', async () => {
    mockUseCreateFeedSource.mockReturnValue({
      mutateAsync: vi.fn().mockRejectedValue(new Error('Server error')),
      isPending: false,
    } as unknown as ReturnType<typeof useCreateFeedSource>);

    renderModal();
    await advanceToMappingStep();

    fireEvent.click(screen.getByRole('button', { name: /createFeed/i }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
    });
    expect(onClose).not.toHaveBeenCalled();
  });

  it('closes the wizard on a successful create', async () => {
    mockUseCreateFeedSource.mockReturnValue({
      mutateAsync: vi.fn().mockResolvedValue({ id: 'feed-1' }),
      isPending: false,
    } as unknown as ReturnType<typeof useCreateFeedSource>);

    renderModal();
    await advanceToMappingStep();

    fireEvent.click(screen.getByRole('button', { name: /createFeed/i }));

    await waitFor(() => {
      expect(onClose).toHaveBeenCalledOnce();
    });
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });
});
