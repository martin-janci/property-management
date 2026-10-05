/**
 * SyncSchedule Component Tests
 *
 * Regression coverage for the silent save failure: handleSave used to await
 * updateMutation.mutateAsync without a catch, so a failed "Save Changes"
 * produced an unhandled rejection — no user feedback and the form stayed
 * open (setIsEditing(false) never reached) with no explanation. The fix
 * wraps the mutation in try/catch and surfaces updateMutation.isError inline.
 */

import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

// Mock @ppt/reality-api-client before importing the component.
vi.mock('@ppt/reality-api-client', () => ({
  useSyncSchedule: vi.fn(),
  useSyncHistory: vi.fn(() => ({ data: [] })),
  useUpdateSyncSchedule: vi.fn(),
}));

import { useSyncSchedule, useUpdateSyncSchedule } from '@ppt/reality-api-client';
import { SyncSchedule } from './SyncSchedule';

const mockUseSyncSchedule = vi.mocked(useSyncSchedule);
const mockUseUpdateSyncSchedule = vi.mocked(useUpdateSyncSchedule);

const baseProps = {
  agencyId: 'agency-1',
  connectionId: 'conn-1',
  connectionName: 'Acme CRM',
};

function mockSchedule() {
  mockUseSyncSchedule.mockReturnValue({
    data: { frequency: 'daily', preferredTime: '09:00', preferredDay: 1, enabled: true },
    isLoading: false,
  } as unknown as ReturnType<typeof useSyncSchedule>);
}

describe('SyncSchedule — save error handling', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockSchedule();
  });

  it('surfaces an inline error when the update mutation reports failure', () => {
    mockUseUpdateSyncSchedule.mockReturnValue({
      mutateAsync: vi.fn().mockRejectedValue(new Error('Network error')),
      isPending: false,
      isError: true,
    } as unknown as ReturnType<typeof useUpdateSyncSchedule>);

    render(<SyncSchedule {...baseProps} />);
    fireEvent.click(screen.getByRole('button', { name: /editSchedule/i }));

    // Mocked useTranslations returns the key verbatim.
    const alert = screen.getByRole('alert');
    expect(alert).toBeInTheDocument();
    expect(alert).toHaveTextContent('saveError');
  });

  it('keeps the edit form open when the save fails', async () => {
    const mutateAsync = vi.fn().mockRejectedValue(new Error('Server error'));
    mockUseUpdateSyncSchedule.mockReturnValue({
      mutateAsync,
      isPending: false,
      isError: false,
    } as unknown as ReturnType<typeof useUpdateSyncSchedule>);

    render(<SyncSchedule {...baseProps} />);
    fireEvent.click(screen.getByRole('button', { name: /editSchedule/i }));
    fireEvent.click(screen.getByRole('button', { name: /saveChanges/i }));

    await waitFor(() => {
      expect(mutateAsync).toHaveBeenCalledOnce();
    });

    // Still in edit mode: cancel/save present, the display-mode Edit button gone.
    expect(screen.getByRole('button', { name: /cancel/i })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /editSchedule/i })).not.toBeInTheDocument();
  });

  it('closes the edit form on a successful save', async () => {
    const mutateAsync = vi.fn().mockResolvedValue({});
    mockUseUpdateSyncSchedule.mockReturnValue({
      mutateAsync,
      isPending: false,
      isError: false,
    } as unknown as ReturnType<typeof useUpdateSyncSchedule>);

    render(<SyncSchedule {...baseProps} />);
    fireEvent.click(screen.getByRole('button', { name: /editSchedule/i }));
    fireEvent.click(screen.getByRole('button', { name: /saveChanges/i }));

    // Back to display mode: the Edit button returns, no error alert.
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /editSchedule/i })).toBeInTheDocument();
    });
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });
});
