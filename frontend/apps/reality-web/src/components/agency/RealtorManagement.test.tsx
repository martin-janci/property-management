/**
 * RealtorManagement Component Tests
 *
 * Tests for InviteRealtorModal error handling (bug: silent invite failure).
 * Ensures the mutation error is surfaced inline and the modal stays open on failure.
 */

import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

// Mock @ppt/reality-api-client before importing the component
vi.mock('@ppt/reality-api-client', () => ({
  useInviteRealtor: vi.fn(),
  useMyAgency: vi.fn(() => ({ data: { id: 'agency-1' } })),
  useRealtors: vi.fn(() => ({ data: [], isLoading: false })),
  useRemoveRealtor: vi.fn(() => ({ mutateAsync: vi.fn() })),
  useResendInvitation: vi.fn(() => ({ mutateAsync: vi.fn() })),
  useUpdateRealtor: vi.fn(() => ({ mutateAsync: vi.fn() })),
}));

import type { Realtor } from '@ppt/reality-api-client';
import { useInviteRealtor, useRemoveRealtor, useUpdateRealtor } from '@ppt/reality-api-client';
import { InviteRealtorModal, RealtorDetailModal } from './RealtorManagement';

const mockUseInviteRealtor = vi.mocked(useInviteRealtor);
const mockUseUpdateRealtor = vi.mocked(useUpdateRealtor);
const mockUseRemoveRealtor = vi.mocked(useRemoveRealtor);

describe('InviteRealtorModal — error handling', () => {
  const agencyId = 'agency-1';
  const onClose = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('shows inline error message when invite mutation fails', async () => {
    mockUseInviteRealtor.mockReturnValue({
      mutateAsync: vi.fn().mockRejectedValue(new Error('Network error')),
      isPending: false,
    } as unknown as ReturnType<typeof useInviteRealtor>);

    render(<InviteRealtorModal agencyId={agencyId} onClose={onClose} />);

    fireEvent.change(screen.getByLabelText(/formLabelEmail/i), {
      target: { value: 'test@example.com' },
    });
    fireEvent.change(screen.getByLabelText(/formLabelName/i), {
      target: { value: 'Jane Doe' },
    });

    fireEvent.click(screen.getByRole('button', { name: /buttonSendInvitation/i }));

    // Error alert should appear (mock useTranslations returns the key: "inviteError")
    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
    });
    expect(screen.getByRole('alert')).toHaveTextContent('inviteError');
  });

  it('keeps the modal open when invite mutation fails', async () => {
    mockUseInviteRealtor.mockReturnValue({
      mutateAsync: vi.fn().mockRejectedValue(new Error('Server error')),
      isPending: false,
    } as unknown as ReturnType<typeof useInviteRealtor>);

    render(<InviteRealtorModal agencyId={agencyId} onClose={onClose} />);

    fireEvent.change(screen.getByLabelText(/formLabelEmail/i), {
      target: { value: 'test@example.com' },
    });
    fireEvent.change(screen.getByLabelText(/formLabelName/i), {
      target: { value: 'Jane Doe' },
    });

    fireEvent.click(screen.getByRole('button', { name: /buttonSendInvitation/i }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
    });

    // onClose must NOT have been called — modal stays open on failure
    expect(onClose).not.toHaveBeenCalled();
  });

  it('closes the modal on successful invite', async () => {
    mockUseInviteRealtor.mockReturnValue({
      mutateAsync: vi.fn().mockResolvedValue({}),
      isPending: false,
    } as unknown as ReturnType<typeof useInviteRealtor>);

    render(<InviteRealtorModal agencyId={agencyId} onClose={onClose} />);

    fireEvent.change(screen.getByLabelText(/formLabelEmail/i), {
      target: { value: 'test@example.com' },
    });
    fireEvent.change(screen.getByLabelText(/formLabelName/i), {
      target: { value: 'Jane Doe' },
    });

    fireEvent.click(screen.getByRole('button', { name: /buttonSendInvitation/i }));

    await waitFor(() => {
      expect(onClose).toHaveBeenCalledOnce();
    });
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('clears the error message before a subsequent submission attempt', async () => {
    const mutateAsync = vi
      .fn()
      .mockRejectedValueOnce(new Error('First failure'))
      .mockResolvedValueOnce({});

    mockUseInviteRealtor.mockReturnValue({
      mutateAsync,
      isPending: false,
    } as unknown as ReturnType<typeof useInviteRealtor>);

    render(<InviteRealtorModal agencyId={agencyId} onClose={onClose} />);

    fireEvent.change(screen.getByLabelText(/formLabelEmail/i), {
      target: { value: 'test@example.com' },
    });
    fireEvent.change(screen.getByLabelText(/formLabelName/i), {
      target: { value: 'Jane Doe' },
    });

    // First submit — should fail and show error
    fireEvent.click(screen.getByRole('button', { name: /buttonSendInvitation/i }));
    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
    });

    // Second submit — succeeds, modal closes
    fireEvent.click(screen.getByRole('button', { name: /buttonSendInvitation/i }));
    await waitFor(() => {
      expect(onClose).toHaveBeenCalledOnce();
    });
  });
});

describe('RealtorDetailModal — error handling', () => {
  const agencyId = 'agency-1';
  const onClose = vi.fn();

  const realtor: Realtor = {
    id: 'realtor-1',
    name: 'Jane Doe',
    email: 'jane@example.com',
    status: 'active',
    title: 'Senior Agent',
    bio: 'Bio',
    activeListings: 3,
    totalSales: 10,
  } as unknown as Realtor;

  beforeEach(() => {
    vi.clearAllMocks();
    // Default happy-path mocks — individual tests override as needed.
    mockUseUpdateRealtor.mockReturnValue({
      mutateAsync: vi.fn().mockResolvedValue({}),
    } as unknown as ReturnType<typeof useUpdateRealtor>);
    mockUseRemoveRealtor.mockReturnValue({
      mutateAsync: vi.fn().mockResolvedValue({}),
    } as unknown as ReturnType<typeof useRemoveRealtor>);
  });

  it('surfaces an inline error when saving edits fails', async () => {
    mockUseUpdateRealtor.mockReturnValue({
      mutateAsync: vi.fn().mockRejectedValue(new Error('Network error')),
    } as unknown as ReturnType<typeof useUpdateRealtor>);

    render(<RealtorDetailModal realtor={realtor} agencyId={agencyId} onClose={onClose} />);

    fireEvent.click(screen.getByRole('button', { name: /buttonEdit/i }));
    fireEvent.click(screen.getByRole('button', { name: /buttonSaveChanges/i }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
    });
    expect(screen.getByRole('alert')).toHaveTextContent('detailUpdateError');
  });

  it('surfaces an inline error and keeps the modal open when removal fails', async () => {
    mockUseRemoveRealtor.mockReturnValue({
      mutateAsync: vi.fn().mockRejectedValue(new Error('Server error')),
    } as unknown as ReturnType<typeof useRemoveRealtor>);

    render(<RealtorDetailModal realtor={realtor} agencyId={agencyId} onClose={onClose} />);

    fireEvent.click(screen.getByRole('button', { name: /buttonRemove/i }));
    fireEvent.click(screen.getByRole('button', { name: /confirmRemoveYes/i }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
    });
    expect(screen.getByRole('alert')).toHaveTextContent('detailRemoveError');
    // Modal must stay open on failure so the user can retry.
    expect(onClose).not.toHaveBeenCalled();
  });

  it('surfaces an inline error when a status change fails', async () => {
    mockUseUpdateRealtor.mockReturnValue({
      mutateAsync: vi.fn().mockRejectedValue(new Error('Network error')),
    } as unknown as ReturnType<typeof useUpdateRealtor>);

    render(<RealtorDetailModal realtor={realtor} agencyId={agencyId} onClose={onClose} />);

    fireEvent.click(screen.getByRole('button', { name: /statusInactive/i }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
    });
    expect(screen.getByRole('alert')).toHaveTextContent('detailStatusError');
  });

  it('closes the modal on a successful removal', async () => {
    render(<RealtorDetailModal realtor={realtor} agencyId={agencyId} onClose={onClose} />);

    fireEvent.click(screen.getByRole('button', { name: /buttonRemove/i }));
    fireEvent.click(screen.getByRole('button', { name: /confirmRemoveYes/i }));

    await waitFor(() => {
      expect(onClose).toHaveBeenCalledOnce();
    });
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });
});
