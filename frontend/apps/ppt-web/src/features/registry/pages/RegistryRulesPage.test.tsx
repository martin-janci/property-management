/// <reference types="vitest/globals" />
/**
 * RegistryRulesPage save-feedback regression tests.
 *
 * History: `handleSubmit` caught a failed `updateRulesMutation.mutateAsync` and
 * only called `console.error` — no user-visible side effect — and the success
 * branch carried a `// Success - could show a toast notification here` TODO, so
 * it was silent too. A failed save therefore looked identical to a successful
 * one (the earlier inline `isError`/`isSuccess` banners were hardcoded English
 * and duplicated the shared-toast UX the rest of ppt-web uses).
 *
 * These tests lock in the ppt-web convention (useToast): the save shows a
 * success Toast on resolve and an error Toast (surfacing the server's message)
 * on reject. They fail on the pre-fix version because no toast ever appears.
 */
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ToastProvider } from '../../../components';
import { RegistryRulesPage } from './RegistryRulesPage';

const mutateAsync = vi.fn();

vi.mock('@ppt/api-client', () => ({
  createRegistryApi: vi.fn(() => ({})),
  createRegistryHooks: vi.fn(() => ({
    useRegistryRules: vi.fn(() => ({
      data: {
        petsAllowed: true,
        petsRequireApproval: true,
        allowedPetTypes: [],
        bannedPetBreeds: [],
        vehiclesRequireApproval: false,
        notes: '',
      },
      isLoading: false,
      error: null,
    })),
    useUpdateRegistryRules: vi.fn(() => ({ mutateAsync, isPending: false })),
  })),
}));

function renderPage() {
  return render(
    <MemoryRouter initialEntries={['/buildings/bld-1/registry/rules']}>
      <ToastProvider>
        <Routes>
          <Route path="/buildings/:buildingId/registry/rules" element={<RegistryRulesPage />} />
        </Routes>
      </ToastProvider>
    </MemoryRouter>
  );
}

function save() {
  fireEvent.click(screen.getByRole('button', { name: /save rules/i }));
}

function alerts() {
  return screen.queryAllByRole('alert');
}

function hasToast(pattern: RegExp) {
  return alerts().some((el) => pattern.test(el.textContent ?? ''));
}

describe('RegistryRulesPage save feedback', () => {
  beforeEach(() => {
    mutateAsync.mockReset();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('surfaces an error toast (with the server message) when the save fails', async () => {
    mutateAsync.mockRejectedValueOnce(new Error('Internal server error'));

    renderPage();
    save();

    await waitFor(() => expect(hasToast(/failed to save registry rules/i)).toBe(true));
    // The server's error message is surfaced, not a generic fallback.
    expect(hasToast(/internal server error/i)).toBe(true);
    // No success toast leaked through on the failure path.
    expect(hasToast(/registry rules saved/i)).toBe(false);
  });

  it('shows a success toast when the save succeeds', async () => {
    mutateAsync.mockResolvedValueOnce(undefined);

    renderPage();
    save();

    await waitFor(() => expect(hasToast(/registry rules saved/i)).toBe(true));
    expect(hasToast(/failed to save registry rules/i)).toBe(false);
    expect(mutateAsync).toHaveBeenCalledWith(expect.objectContaining({ buildingId: 'bld-1' }));
  });
});
