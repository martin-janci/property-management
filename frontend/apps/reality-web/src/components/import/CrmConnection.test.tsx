/**
 * CrmConnection Component Tests
 *
 * Tests for AddConnectionModal error handling (bug: silent create failure —
 * the create-connection wizard swallowed mutateAsync rejections and hung with
 * no user-facing feedback). Mirrors the RealtorManagement invite-modal fix.
 * Ensures the create mutation error is surfaced inline (role="alert") and the
 * wizard stays open on failure.
 */

import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

// Mock @ppt/reality-api-client before importing the component.
vi.mock('@ppt/reality-api-client', () => ({
  useCreateCrmConnection: vi.fn(),
  useTestCrmConnection: vi.fn(),
  useCrmConnections: vi.fn(() => ({ data: [], isLoading: false })),
  useDeleteCrmConnection: vi.fn(() => ({ mutateAsync: vi.fn() })),
  useMyAgency: vi.fn(() => ({ data: { id: 'agency-1' } })),
  useSyncCrmConnection: vi.fn(() => ({ mutate: vi.fn() })),
}));

import { useCreateCrmConnection, useTestCrmConnection } from '@ppt/reality-api-client';
import { AddConnectionModal } from './CrmConnection';

const mockUseCreateCrmConnection = vi.mocked(useCreateCrmConnection);
const mockUseTestCrmConnection = vi.mocked(useTestCrmConnection);

// Drive the wizard from the provider grid through to the mapping step, where
// the "Create Connection" button lives.
async function advanceToMappingStep() {
  // Step "select": pick the HubSpot provider card.
  fireEvent.click(screen.getByRole('button', { name: /HubSpot/i }));
  // Step "configure": fill the API key, then run the (successful) test.
  fireEvent.change(screen.getByLabelText(/apiKey/i), { target: { value: 'secret-key' } });
  fireEvent.click(screen.getByRole('button', { name: /testConnection/i }));
  // Successful test advances to the "mapping" step.
  await waitFor(() => {
    expect(screen.getByRole('button', { name: /createConnection/i })).toBeInTheDocument();
  });
}

describe('AddConnectionModal — create error handling', () => {
  const agencyId = 'agency-1';
  const onClose = vi.fn();
  const onSelectProvider = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    // A test connection always succeeds so the wizard can reach the mapping step.
    mockUseTestCrmConnection.mockReturnValue({
      mutateAsync: vi.fn().mockResolvedValue({ success: true, message: 'ok' }),
      isPending: false,
    } as unknown as ReturnType<typeof useTestCrmConnection>);
  });

  function renderModal() {
    render(
      <AddConnectionModal
        agencyId={agencyId}
        selectedProvider="hubspot"
        onSelectProvider={onSelectProvider}
        onClose={onClose}
      />
    );
  }

  it('shows an inline error when the create mutation fails', async () => {
    mockUseCreateCrmConnection.mockReturnValue({
      mutateAsync: vi.fn().mockRejectedValue(new Error('Network error')),
      isPending: false,
    } as unknown as ReturnType<typeof useCreateCrmConnection>);

    renderModal();
    await advanceToMappingStep();

    fireEvent.click(screen.getByRole('button', { name: /createConnection/i }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
    });
    expect(screen.getByRole('alert')).toHaveTextContent('createError');
  });

  it('keeps the wizard open when the create mutation fails', async () => {
    mockUseCreateCrmConnection.mockReturnValue({
      mutateAsync: vi.fn().mockRejectedValue(new Error('Server error')),
      isPending: false,
    } as unknown as ReturnType<typeof useCreateCrmConnection>);

    renderModal();
    await advanceToMappingStep();

    fireEvent.click(screen.getByRole('button', { name: /createConnection/i }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
    });
    expect(onClose).not.toHaveBeenCalled();
  });

  it('closes the wizard on a successful create', async () => {
    mockUseCreateCrmConnection.mockReturnValue({
      mutateAsync: vi.fn().mockResolvedValue({ id: 'conn-1' }),
      isPending: false,
    } as unknown as ReturnType<typeof useCreateCrmConnection>);

    renderModal();
    await advanceToMappingStep();

    fireEvent.click(screen.getByRole('button', { name: /createConnection/i }));

    await waitFor(() => {
      expect(onClose).toHaveBeenCalledOnce();
    });
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  // Credential-leak regression: an API-client rejection carries the request
  // body (incl. the CRM apiKey) on `config.data`. The catch handler must not
  // hand that carrier object to console.error, or the key ends up in logs.
  it('does not log the secret-bearing payload on a create failure', async () => {
    const SECRET = 'sk_live_SUPER_SECRET_API_KEY';
    const leakyError = Object.assign(new Error('Request failed with status code 400'), {
      config: { data: JSON.stringify({ credentials: { apiKey: SECRET } }) },
      request: { body: SECRET },
    });
    mockUseCreateCrmConnection.mockReturnValue({
      mutateAsync: vi.fn().mockRejectedValue(leakyError),
      isPending: false,
    } as unknown as ReturnType<typeof useCreateCrmConnection>);

    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    try {
      renderModal();
      await advanceToMappingStep();

      fireEvent.click(screen.getByRole('button', { name: /createConnection/i }));

      await waitFor(() => {
        expect(screen.getByRole('alert')).toBeInTheDocument();
      });

      // Whatever got logged, the secret must never appear in any argument.
      const loggedBlob = consoleErrorSpy.mock.calls
        .flat()
        .map((arg) => {
          try {
            return typeof arg === 'string' ? arg : JSON.stringify(arg);
          } catch {
            return String(arg);
          }
        })
        .join(' ');
      expect(loggedBlob).not.toContain(SECRET);
    } finally {
      consoleErrorSpy.mockRestore();
    }
  });
});
