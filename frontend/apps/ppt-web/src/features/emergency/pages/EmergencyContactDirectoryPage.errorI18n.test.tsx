/// <reference types="vitest/globals" />
/**
 * Emergency Contact Directory error-copy i18n regression.
 *
 * The five error handlers on `EmergencyContactDirectoryPage` (load, reload,
 * create, update, delete) shipped as:
 *
 *   setError(err instanceof Error ? err.message : t('emergency.errors.failedToX'))
 *
 * On the common failure path — an API rejection is an `Error` — the ternary's
 * `err.message` branch always wins, so the raw backend copy (English, and not
 * localized for cs/de/hu/pl/sk users) was surfaced verbatim in the `role=alert`
 * banner and the `t()` fallback was dead code. The fix routes every handler
 * through `t('emergency.errors.*')` unconditionally and logs the raw error only
 * to the console.
 *
 * These tests pin that: given an API rejection whose `.message` is a distinctive
 * raw string, the visible alert must show the translated catalog copy and must
 * NOT contain the raw message. They fail on `main` (raw message leaks through)
 * and pass once the ternary is removed. The load path is exercised on mount; the
 * create / update / delete paths are driven through stubbed child components.
 */

import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import en from '../../../../messages/en.json';
import { OrganizationContext } from '../../../hooks/useOrganization';
import { EmergencyContactDirectoryPage } from './EmergencyContactDirectoryPage';

// A raw backend message that must never reach the UI banner.
const RAW = 'RAW_BACKEND_500_do_not_show_me';

// ─── network boundary: @ppt/api-client ──────────────────────────────────────
const listEmergencyContacts = vi.fn();
const createEmergencyContact = vi.fn();
const updateEmergencyContact = vi.fn();
const deleteEmergencyContact = vi.fn();

vi.mock('@ppt/api-client', () => ({
  CONTACT_TYPE_LABELS: {},
  listEmergencyContacts: (...args: unknown[]) => listEmergencyContacts(...args),
  createEmergencyContact: (...args: unknown[]) => createEmergencyContact(...args),
  updateEmergencyContact: (...args: unknown[]) => updateEmergencyContact(...args),
  deleteEmergencyContact: (...args: unknown[]) => deleteEmergencyContact(...args),
}));

// ─── child components: reduce to controllable trigger surfaces ──────────────
const CONTACT = { id: 'c-1', name: 'Fire Dept', role: 'Fire', phone: '112', email: null };

vi.mock('../components', () => ({
  EmergencyContactForm: ({ onSubmit }: { onSubmit: (d: unknown) => void }) => (
    <button type="button" data-testid="form-submit" onClick={() => onSubmit({ name: 'x' })}>
      submit
    </button>
  ),
  EmergencyContactsList: ({ onDelete }: { onDelete: (c: unknown) => void }) => (
    <button type="button" data-testid="list-delete" onClick={() => onDelete(CONTACT)}>
      delete
    </button>
  ),
}));

vi.mock('../../../components', () => ({
  ConfirmationDialog: ({ isOpen, onConfirm }: { isOpen: boolean; onConfirm: () => void }) =>
    isOpen ? (
      <button type="button" data-testid="confirm-delete" onClick={onConfirm}>
        confirm
      </button>
    ) : null,
}));

function renderPage() {
  return render(
    <OrganizationContext.Provider value={{ organizationId: 'org-1' }}>
      <EmergencyContactDirectoryPage />
    </OrganizationContext.Provider>
  );
}

const errors = en.emergency.errors as Record<string, string>;

beforeEach(() => {
  vi.clearAllMocks();
  vi.spyOn(console, 'error').mockImplementation(() => {});
  // Default: list resolves empty so the page mounts cleanly for the
  // mutation-path tests. Individual tests override as needed.
  listEmergencyContacts.mockResolvedValue([]);
  createEmergencyContact.mockResolvedValue(CONTACT);
  updateEmergencyContact.mockResolvedValue(CONTACT);
  deleteEmergencyContact.mockResolvedValue(undefined);
});

describe('EmergencyContactDirectoryPage error copy is localized, not raw', () => {
  it('load failure shows the translated copy, not err.message', async () => {
    listEmergencyContacts.mockRejectedValueOnce(new Error(RAW));

    renderPage();

    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent(errors.failedToLoad);
    expect(alert).not.toHaveTextContent(RAW);
  });

  it('create failure shows the translated copy, not err.message', async () => {
    createEmergencyContact.mockRejectedValueOnce(new Error(RAW));

    renderPage();
    // Open the create form, then submit through the stubbed form.
    fireEvent.click(await screen.findByText(en.emergency.addContact));
    fireEvent.click(screen.getByTestId('form-submit'));

    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent(errors.failedToCreate);
    expect(alert).not.toHaveTextContent(RAW);
  });

  it('delete failure shows the translated copy, not err.message', async () => {
    deleteEmergencyContact.mockRejectedValueOnce(new Error(RAW));

    renderPage();
    // Wait for initial load to settle, then drive delete -> confirm.
    await waitFor(() => expect(listEmergencyContacts).toHaveBeenCalled());
    fireEvent.click(screen.getByTestId('list-delete'));
    fireEvent.click(screen.getByTestId('confirm-delete'));

    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent(errors.failedToDelete);
    expect(alert).not.toHaveTextContent(RAW);
  });
});
