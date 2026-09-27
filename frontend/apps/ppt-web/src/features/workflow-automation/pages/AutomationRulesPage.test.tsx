/// <reference types="vitest/globals" />
/**
 * AutomationRulesPage mutation-feedback regression tests.
 *
 * History: the automation-rules page fired its delete / toggle / run mutations
 * with a bare `await mutation.mutateAsync(...)` and no try/catch. A rejected
 * mutation therefore surfaced nothing to the user (no toast, no inline error)
 * and produced an unhandled promise rejection — the failure was swallowed
 * silently. Success was equally silent.
 *
 * These tests lock in the ppt-web error-handling convention (useToast): every
 * automation-rules mutation now shows an error Toast on failure and a success
 * Toast on success. The success path also relies on the mutation hooks'
 * onSuccess query invalidation to reload the list, so on delete the confirm
 * modal only closes once the mutation resolves.
 */
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ToastProvider } from '../../../components';
import { AutomationRulesPage } from './AutomationRulesPage';

const deleteMutate = vi.fn();
const updateMutate = vi.fn();
const runMutate = vi.fn();

const rules = [
  {
    id: 'rule-manual',
    name: 'Manual rule',
    isEnabled: true,
    trigger: { type: 'manual', name: 'Manual' },
    actions: [],
  },
  {
    id: 'rule-timed',
    name: 'Timed rule',
    isEnabled: false,
    trigger: { type: 'time_based', name: 'Daily' },
    actions: [],
  },
];

vi.mock('@ppt/api-client', () => ({
  useAutomationRules: vi.fn(() => ({
    data: { data: rules, total: rules.length },
    isLoading: false,
    error: null,
  })),
  useDeleteAutomationRule: vi.fn(() => ({ mutateAsync: deleteMutate, isPending: false })),
  useUpdateAutomationRule: vi.fn(() => ({ mutateAsync: updateMutate, isPending: false })),
  useRunAutomationRule: vi.fn(() => ({ mutateAsync: runMutate, isPending: false })),
}));

function renderPage() {
  return render(
    <MemoryRouter>
      <ToastProvider>
        <AutomationRulesPage />
      </ToastProvider>
    </MemoryRouter>
  );
}

function alerts() {
  return screen.queryAllByRole('alert');
}

function hasToast(pattern: RegExp) {
  return alerts().some((el) => pattern.test(el.textContent ?? ''));
}

// Open the delete-confirmation modal for the first rule and return its confirm
// button (both the RuleCard and the modal expose a "Delete" button, so scope to
// the modal panel via its unique heading).
function openDeleteModalAndGetConfirm() {
  fireEvent.click(screen.getAllByRole('button', { name: /^delete$/i })[0]);
  const heading = screen.getByRole('heading', { name: /delete automation rule/i });
  const panel = heading.closest('div.bg-white') as HTMLElement;
  return within(panel).getByRole('button', { name: /^delete$/i });
}

describe('AutomationRulesPage mutation feedback', () => {
  beforeEach(() => {
    deleteMutate.mockReset();
    updateMutate.mockReset();
    runMutate.mockReset();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('surfaces an error toast when delete fails and keeps the confirm modal open', async () => {
    deleteMutate.mockRejectedValueOnce(new Error('boom'));
    renderPage();

    fireEvent.click(openDeleteModalAndGetConfirm());

    await waitFor(() => expect(hasToast(/delete failed/i)).toBe(true));
    // The failure message is propagated, not swallowed.
    expect(hasToast(/boom/i)).toBe(true);
    // Modal stays open so the user can retry.
    expect(screen.getByText(/this action cannot be undone/i)).toBeInTheDocument();
  });

  it('shows a success toast and closes the modal when delete succeeds', async () => {
    deleteMutate.mockResolvedValueOnce(undefined);
    renderPage();

    fireEvent.click(openDeleteModalAndGetConfirm());

    await waitFor(() => expect(hasToast(/rule deleted/i)).toBe(true));
    expect(deleteMutate).toHaveBeenCalledWith('rule-manual');
    await waitFor(() =>
      expect(screen.queryByText(/this action cannot be undone/i)).not.toBeInTheDocument()
    );
  });

  it('surfaces an error toast when toggling a rule fails', async () => {
    updateMutate.mockRejectedValueOnce(new Error('nope'));
    renderPage();

    fireEvent.click(screen.getByRole('switch', { name: /toggle manual rule/i }));

    await waitFor(() => expect(hasToast(/update failed/i)).toBe(true));
  });

  it('surfaces an error toast when running a rule fails', async () => {
    runMutate.mockRejectedValueOnce(new Error('run-error'));
    renderPage();

    fireEvent.click(screen.getByRole('button', { name: /run now/i }));

    await waitFor(() => expect(hasToast(/run failed/i)).toBe(true));
  });

  it('shows a success toast when running a rule succeeds', async () => {
    runMutate.mockResolvedValueOnce({ id: 'exec-1' });
    renderPage();

    fireEvent.click(screen.getByRole('button', { name: /run now/i }));

    await waitFor(() => expect(hasToast(/rule triggered/i)).toBe(true));
    expect(runMutate).toHaveBeenCalledWith('rule-manual');
  });
});
