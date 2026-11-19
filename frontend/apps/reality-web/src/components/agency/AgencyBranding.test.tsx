/**
 * AgencyBranding Component Tests
 *
 * Tests for the branding save flow error handling (bug: silent save failure —
 * the rejected mutation promise was swallowed and nothing was surfaced to the
 * user). Ensures a failed save shows an inline alert and keeps `hasChanges`
 * true so the user can retry, and that a successful save clears the dirty state
 * without showing an alert.
 */

import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

// Mock @ppt/reality-api-client before importing the component.
vi.mock('@ppt/reality-api-client', () => ({
  useMyAgency: vi.fn(() => ({
    data: { id: 'agency-1', name: 'Test Agency' },
    isLoading: false,
    isError: false,
    error: null,
    refetch: vi.fn(),
  })),
  useAgencyBranding: vi.fn(() => ({
    data: {
      primaryColor: '#111111',
      secondaryColor: '#222222',
      accentColor: '#333333',
    },
    isLoading: false,
  })),
  useUpdateBranding: vi.fn(),
}));

import { useUpdateBranding } from '@ppt/reality-api-client';
import { AgencyBranding } from './AgencyBranding';

const mockUseUpdateBranding = vi.mocked(useUpdateBranding);

// The save button is disabled until the form is dirty; nudging a colour input
// flips `hasChanges` so the button becomes clickable.
function makeChange() {
  fireEvent.change(screen.getByLabelText(/primaryColor/i), {
    target: { value: '#abcdef' },
  });
}

describe('AgencyBranding — save error handling', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('shows an inline error alert when the save mutation fails', async () => {
    mockUseUpdateBranding.mockReturnValue({
      mutateAsync: vi.fn().mockRejectedValue(new Error('Network error')),
      isPending: false,
    } as unknown as ReturnType<typeof useUpdateBranding>);

    render(<AgencyBranding />);
    makeChange();
    fireEvent.click(screen.getByRole('button', { name: /saveChanges/i }));

    // Mock useTranslations returns the key, so the alert text is "saveError".
    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
    });
    expect(screen.getByRole('alert')).toHaveTextContent('saveError');
  });

  it('keeps the save button enabled after a failed save so the user can retry', async () => {
    mockUseUpdateBranding.mockReturnValue({
      mutateAsync: vi.fn().mockRejectedValue(new Error('Server error')),
      isPending: false,
    } as unknown as ReturnType<typeof useUpdateBranding>);

    render(<AgencyBranding />);
    makeChange();
    const saveButton = screen.getByRole('button', { name: /saveChanges/i });
    fireEvent.click(saveButton);

    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
    });
    // hasChanges must remain true on failure -> button stays enabled.
    expect(saveButton).not.toBeDisabled();
  });

  it('clears the dirty state and shows no alert on a successful save', async () => {
    mockUseUpdateBranding.mockReturnValue({
      mutateAsync: vi.fn().mockResolvedValue({}),
      isPending: false,
    } as unknown as ReturnType<typeof useUpdateBranding>);

    render(<AgencyBranding />);
    makeChange();
    const saveButton = screen.getByRole('button', { name: /saveChanges/i });
    fireEvent.click(saveButton);

    await waitFor(() => {
      expect(saveButton).toBeDisabled();
    });
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('clears a previous error before a subsequent save attempt', async () => {
    const mutateAsync = vi
      .fn()
      .mockRejectedValueOnce(new Error('First failure'))
      .mockResolvedValueOnce({});

    mockUseUpdateBranding.mockReturnValue({
      mutateAsync,
      isPending: false,
    } as unknown as ReturnType<typeof useUpdateBranding>);

    render(<AgencyBranding />);
    makeChange();
    const saveButton = screen.getByRole('button', { name: /saveChanges/i });

    // First attempt fails and surfaces the alert.
    fireEvent.click(saveButton);
    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
    });

    // Second attempt succeeds: the alert clears and the form is no longer dirty.
    fireEvent.click(saveButton);
    await waitFor(() => {
      expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    });
  });
});
