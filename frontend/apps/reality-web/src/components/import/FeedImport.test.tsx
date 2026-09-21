/**
 * FeedImport / FeedCard Component Tests
 *
 * Regression tests for the FeedCard silent mutation-failure bug: a failed
 * pause-toggle or delete used to be indistinguishable from success. These
 * tests ensure the failure is surfaced inline (role="alert"), and that a
 * successful action shows no error.
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

import { useDeleteFeedSource, useFeedSources, useUpdateFeedSource } from '@ppt/reality-api-client';
import { FeedImport } from './FeedImport';

const mockUseFeedSources = vi.mocked(useFeedSources);
const mockUseUpdateFeedSource = vi.mocked(useUpdateFeedSource);
const mockUseDeleteFeedSource = vi.mocked(useDeleteFeedSource);

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
