/**
 * SyncSchedule Component Tests
 *
 * Regression coverage for the save-error banner behaviour.
 *
 * Original bug (PR #2967): handleSave awaited updateMutation.mutateAsync
 * without a catch, so a failed "Save Changes" produced an unhandled rejection
 * — no user feedback and the form stayed open with no explanation.
 *
 * Follow-up bug (issue #3029): the fix surfaced the banner via the persistent
 * React Query flag updateMutation.isError, which stays true until the next
 * mutate/reset. Cancel → re-open Edit therefore re-rendered the stale banner
 * before the user had attempted anything. The banner is now driven by local
 * saveError state that is cleared whenever an edit session starts, so a prior
 * failure never bleeds into a fresh one.
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

  it('surfaces an inline error only after a save fails (not on edit open)', async () => {
    const mutateAsync = vi.fn().mockRejectedValue(new Error('Network error'));
    mockUseUpdateSyncSchedule.mockReturnValue({
      mutateAsync,
      isPending: false,
      isError: false,
    } as unknown as ReturnType<typeof useUpdateSyncSchedule>);

    render(<SyncSchedule {...baseProps} />);
    fireEvent.click(screen.getByRole('button', { name: /editSchedule/i }));

    // No banner before the user attempts to save.
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /saveChanges/i }));

    // Mocked useTranslations returns the key verbatim.
    const alert = await screen.findByRole('alert');
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

  it('clears the stale error banner when re-entering edit after a failed save (#3029)', async () => {
    const mutateAsync = vi.fn().mockRejectedValue(new Error('Server error'));
    mockUseUpdateSyncSchedule.mockReturnValue({
      mutateAsync,
      isPending: false,
      // Simulate the persistent mutation flag staying true after a rejection —
      // the old render guard would re-show the banner on re-edit because of it.
      isError: true,
    } as unknown as ReturnType<typeof useUpdateSyncSchedule>);

    render(<SyncSchedule {...baseProps} />);

    // Fail a save to raise the banner.
    fireEvent.click(screen.getByRole('button', { name: /editSchedule/i }));
    fireEvent.click(screen.getByRole('button', { name: /saveChanges/i }));
    await screen.findByRole('alert');

    // Cancel out of the failed edit session…
    fireEvent.click(screen.getByRole('button', { name: /cancel/i }));
    // …and re-open a fresh edit session.
    fireEvent.click(screen.getByRole('button', { name: /editSchedule/i }));

    // The prior failure must not bleed into the new session.
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });
});
