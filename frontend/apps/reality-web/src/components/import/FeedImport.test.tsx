/**
 * FeedImport Component Tests
 *
 * Tests for AddFeedModal error handling (bug: silent create failure — the
 * add-feed wizard swallowed mutateAsync rejections and hung with no
 * user-facing feedback). Mirrors the RealtorManagement invite-modal fix.
 * Ensures the create mutation error is surfaced inline (role="alert") and the
 * wizard stays open on failure.
 */

import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

// Mock @ppt/reality-api-client before importing the component.
vi.mock('@ppt/reality-api-client', () => ({
  useCreateFeedSource: vi.fn(),
  useFeedPreview: vi.fn(),
  useDeleteFeedSource: vi.fn(() => ({ mutateAsync: vi.fn() })),
  useFeedSources: vi.fn(() => ({ data: [], isLoading: false })),
  useFeedSyncHistory: vi.fn(() => ({ data: [] })),
  useMyAgency: vi.fn(() => ({ data: { id: 'agency-1' } })),
  useSyncFeedSource: vi.fn(() => ({ mutate: vi.fn() })),
  useUpdateFeedSource: vi.fn(() => ({ mutateAsync: vi.fn() })),
}));

import { useCreateFeedSource, useFeedPreview } from '@ppt/reality-api-client';
import { AddFeedModal } from './FeedImport';

const mockUseCreateFeedSource = vi.mocked(useCreateFeedSource);
const mockUseFeedPreview = vi.mocked(useFeedPreview);

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
