/// <reference types="vitest/globals" />
/**
 * Regression tests for the document-detail share panel (Story 7A.5).
 *
 * These lock in the resolution of the UX nit tracked as sprint-status
 * issue #485 ("share panel uses window.confirm + no UUID validation on
 * user-ID input"):
 *
 *   1. revoke uses the accessible ConfirmationDialog, NOT window.confirm,
 *      and confirming fires the revoke mutation;
 *   2. a malformed user-ID is rejected with an inline error and the
 *      create mutation never fires;
 *   3. a well-formed UUID is accepted and the create mutation fires with
 *      the expected target_id.
 *
 * Only the @ppt/api-client share hooks (the network boundary) are mocked.
 */

import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ToastProvider } from '../../../components/Toast';
import { DocumentSharePanel } from './DocumentSharePanel';

// ─── network boundary: @ppt/api-client share hooks ──────────────────────────
const useDocumentSharesMock = vi.fn();
const createMutateAsync = vi.fn();
const revokeMutateAsync = vi.fn();
const refetchMock = vi.fn();

const SHARE_TYPE = {
  USER: 'user',
  ROLE: 'role',
  BUILDING: 'building',
  LINK: 'link',
} as const;

vi.mock('@ppt/api-client', () => ({
  SHARE_TYPE: {
    USER: 'user',
    ROLE: 'role',
    BUILDING: 'building',
    LINK: 'link',
  },
  useDocumentShares: (documentId: string) => useDocumentSharesMock(documentId),
  useCreateDocumentShare: () => ({
    mutateAsync: createMutateAsync,
    isPending: false,
  }),
  useRevokeDocumentShare: () => ({
    mutateAsync: revokeMutateAsync,
    isPending: false,
    variables: undefined,
  }),
}));

function makeShare(over: Record<string, unknown> = {}) {
  return {
    id: 'share-1',
    document_id: 'doc-1',
    share_type: SHARE_TYPE.USER,
    target_id: '11111111-2222-3333-4444-555555555555',
    target_role: null,
    share_token: null,
    expires_at: null,
    revoked_at: null,
    ...over,
  };
}

function renderPanel() {
  return render(
    <ToastProvider>
      <DocumentSharePanel documentId="doc-1" documentTitle="Lease.pdf" />
    </ToastProvider>
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  useDocumentSharesMock.mockReturnValue({
    data: { shares: [] },
    isLoading: false,
    error: null,
    refetch: refetchMock,
  });
});

describe('DocumentSharePanel — UUID validation (issue #485)', () => {
  it('rejects a malformed user-ID and does not call the create mutation', async () => {
    renderPanel();

    // Switch to the "Specific user" share type.
    fireEvent.click(screen.getByRole('button', { name: 'Specific user' }));

    const input = screen.getByLabelText('User ID');
    fireEvent.change(input, { target: { value: 'not-a-uuid' } });

    const createBtn = screen.getByRole('button', { name: 'Create share' });
    // Submit is disabled for an invalid UUID …
    expect(createBtn).toBeDisabled();

    // … and even if forced, the mutation must not fire.
    fireEvent.click(createBtn);
    await waitFor(() => {
      expect(createMutateAsync).not.toHaveBeenCalled();
    });

    // An accessible inline error is shown.
    fireEvent.blur(input);
    expect(screen.getByRole('alert')).toHaveTextContent('Must be a valid UUID.');
    expect(input).toHaveAttribute('aria-invalid', 'true');
  });

  it('accepts a well-formed UUID and submits it as target_id', async () => {
    createMutateAsync.mockResolvedValue({ share_url: null });
    renderPanel();

    fireEvent.click(screen.getByRole('button', { name: 'Specific user' }));
    const uuid = 'abcdefab-1234-5678-9abc-def012345678';
    fireEvent.change(screen.getByLabelText('User ID'), { target: { value: uuid } });

    const createBtn = screen.getByRole('button', { name: 'Create share' });
    expect(createBtn).not.toBeDisabled();
    fireEvent.click(createBtn);

    await waitFor(() => {
      expect(createMutateAsync).toHaveBeenCalledWith(
        expect.objectContaining({ share_type: SHARE_TYPE.USER, target_id: uuid })
      );
    });
  });
});

describe('DocumentSharePanel — revoke uses ConfirmationDialog (issue #485)', () => {
  it('opens an accessible dialog instead of window.confirm, and confirming revokes', async () => {
    const confirmSpy = vi.spyOn(window, 'confirm');
    revokeMutateAsync.mockResolvedValue(undefined);
    useDocumentSharesMock.mockReturnValue({
      data: { shares: [makeShare()] },
      isLoading: false,
      error: null,
      refetch: refetchMock,
    });

    renderPanel();

    // No dialog yet.
    expect(screen.queryByRole('alertdialog')).toBeNull();

    // The row-level "Revoke" button opens the dialog.
    fireEvent.click(screen.getByRole('button', { name: 'Revoke' }));

    // The accessible ConfirmationDialog appears — window.confirm is never used.
    const dialog = screen.getByRole('alertdialog');
    expect(dialog).toBeInTheDocument();
    expect(confirmSpy).not.toHaveBeenCalled();

    // The dialog's confirm button (also labelled "Revoke") fires the mutation.
    const dialogConfirm = within(dialog).getByRole('button', { name: 'Revoke' });
    fireEvent.click(dialogConfirm);

    await waitFor(() => {
      expect(revokeMutateAsync).toHaveBeenCalledWith('share-1');
    });

    confirmSpy.mockRestore();
  });
});
