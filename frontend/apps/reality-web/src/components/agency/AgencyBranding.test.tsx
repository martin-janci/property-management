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

import { useAgencyBranding, useUpdateBranding } from '@ppt/reality-api-client';
import { AgencyBranding } from './AgencyBranding';

const mockUseUpdateBranding = vi.mocked(useUpdateBranding);
const mockUseAgencyBranding = vi.mocked(useAgencyBranding);

// The default happy-path branding return re-applied in beforeEach so the
// branding-error suite below can override it without leaking into other tests.
const okBranding = {
  data: {
    primaryColor: '#111111',
    secondaryColor: '#222222',
    accentColor: '#333333',
  },
  isLoading: false,
  isError: false,
  error: null,
  refetch: vi.fn(),
} as unknown as ReturnType<typeof useAgencyBranding>;

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
    // clearAllMocks resets the vi.mock factory's default implementation, so
    // restore the happy-path branding return for this suite.
    mockUseAgencyBranding.mockReturnValue(okBranding);
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

describe('AgencyBranding — branding fetch error handling', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockUseUpdateBranding.mockReturnValue({
      mutateAsync: vi.fn(),
      isPending: false,
    } as unknown as ReturnType<typeof useUpdateBranding>);
  });

  function mockBrandingError(status: number) {
    mockUseAgencyBranding.mockReturnValue({
      data: undefined,
      isLoading: false,
      isError: true,
      error: { status },
      refetch: vi.fn(),
    } as unknown as ReturnType<typeof useAgencyBranding>);
  }

  it('shows the load-error state (not the form) when the branding query fails with a server error', () => {
    // Regression: before the fix the error path was unhandled — isLoading went
    // false, branding stayed undefined, and the form rendered against the
    // default colour state, so a save would clobber the stored branding with
    // defaults. The failure must surface a retry affordance instead.
    mockBrandingError(500);

    render(<AgencyBranding />);

    expect(screen.getByRole('alert')).toHaveTextContent('loadErrorTitle');
    expect(screen.getByRole('button', { name: /retry/i })).toBeInTheDocument();
    // The colour-editing form must NOT be rendered on a transient fetch error.
    expect(screen.queryByLabelText(/primaryColor/i)).not.toBeInTheDocument();
  });

  it('retrying a failed branding fetch calls refetch', () => {
    const refetch = vi.fn();
    mockUseAgencyBranding.mockReturnValue({
      data: undefined,
      isLoading: false,
      isError: true,
      error: { status: 503 },
      refetch,
    } as unknown as ReturnType<typeof useAgencyBranding>);

    render(<AgencyBranding />);
    fireEvent.click(screen.getByRole('button', { name: /retry/i }));

    expect(refetch).toHaveBeenCalledTimes(1);
  });

  it('treats a 404 branding response as the empty state and still renders the form', () => {
    // A freshly created agency has no branding row yet (404). That is the
    // legitimate "no branding configured" state, not a failure — the form
    // must render with the fallback palette so the user can set branding up.
    mockBrandingError(404);

    render(<AgencyBranding />);

    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    expect(screen.getByLabelText(/primaryColor/i)).toBeInTheDocument();
  });
});
