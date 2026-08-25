/// <reference types="vitest/globals" />
/**
 * ContentModerationPage moderation-action / appeal-decision flow tests.
 *
 * History: the Phase-1 flow drove regulated moderation writes through
 * `window.prompt` / `window.alert` with hardcoded copy, and cast the raw prompt
 * string straight into the API unions
 * (`action_type as 'remove'|'restrict'|'warn'|'approve'`,
 * `decision as 'uphold'|'overturn'`) — a typo such as `aprove` or `Uphold` could
 * reach the moderation endpoint and land in the compliance audit trail. This
 * mirrors the AML fix (#2829): the flow is now in-app modal dialogs (localized,
 * Toast feedback), where:
 *   - the action type / appeal decision is constrained to the API union via a
 *     <select>, so an invalid free-text value is structurally impossible, and
 *   - the mutation fires only with a non-empty rationale, matching the old
 *     `if (!rationale) return;` guards.
 *
 * These tests lock in: no window.prompt/window.alert, required rationale gates
 * the mutation, a valid submission sends the typed union value, and re-opening a
 * dialog for a different case starts blank (the #2833-style key remount guard).
 */
import { fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ToastProvider } from '../../../components';
import { ContentModerationPage } from './ContentModerationPage';

const takeActionMutate = vi.fn();
const decideAppealMutate = vi.fn();

// Two pending cases (each renders a "Take Action" button) and two appealed cases
// (each renders a "Decide Appeal" button) so a dialog can be opened for one case
// and re-opened for another — the stale-state regression path.
const baseCase = {
  content_type: 'review',
  content_id: 'content-1',
  content_preview: 'preview',
  owner_id: 'owner-1',
  owner_name: 'Owner One',
  violation_type: 'spam',
  priority: 3,
  assigned_to_name: 'Moderator',
  reported_at: '2026-01-01T00:00:00Z',
};

const cases = [
  { ...baseCase, id: 'pending-1', status: 'pending', is_appeal: false },
  { ...baseCase, id: 'pending-2', status: 'pending', is_appeal: false },
  { ...baseCase, id: 'appealed-1', status: 'appealed', is_appeal: true },
  { ...baseCase, id: 'appealed-2', status: 'appealed', is_appeal: true },
];

vi.mock('@ppt/api-client', () => ({
  useModerationCases: vi.fn(() => ({
    data: { cases },
    isLoading: false,
    error: null,
  })),
  useModerationStats: vi.fn(() => ({ data: undefined })),
  useModerationTemplates: vi.fn(() => ({ data: undefined })),
  useAssignModerationCase: vi.fn(() => ({ mutate: vi.fn(), isPending: false })),
  useTakeModerationAction: vi.fn(() => ({ mutate: takeActionMutate, isPending: false })),
  useDecideModerationAppeal: vi.fn(() => ({ mutate: decideAppealMutate, isPending: false })),
}));

function renderPage() {
  return render(
    <ToastProvider>
      <ContentModerationPage />
    </ToastProvider>
  );
}

function openTakeActionDialog(cardIndex = 0) {
  fireEvent.click(screen.getAllByRole('button', { name: /take action/i })[cardIndex]);
}

function openDecideAppealDialog(cardIndex = 0) {
  fireEvent.click(screen.getAllByRole('button', { name: /decide appeal/i })[cardIndex]);
}

// The page also renders queue filter <select>s, so scope form queries to the
// open dialog rather than the whole document.
function dialog() {
  return within(screen.getByRole('dialog'));
}

describe('ContentModerationPage decision flow', () => {
  const promptSpy = vi.spyOn(window, 'prompt');
  const alertSpy = vi.spyOn(window, 'alert').mockImplementation(() => {});

  beforeEach(() => {
    takeActionMutate.mockClear();
    decideAppealMutate.mockClear();
    promptSpy.mockClear();
    alertSpy.mockClear();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('never uses window.prompt or window.alert for the take-action flow', () => {
    renderPage();
    openTakeActionDialog();

    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(promptSpy).not.toHaveBeenCalled();
    expect(alertSpy).not.toHaveBeenCalled();
  });

  it('exposes only the four typed action options and defaults notify-owner to true', () => {
    renderPage();
    openTakeActionDialog();

    const options = dialog()
      .getAllByRole('option')
      .map((o) => (o as HTMLOptionElement).value)
      .sort();
    expect(options).toEqual(['approve', 'remove', 'restrict', 'warn']);

    const notify = dialog().getByLabelText(/notify content owner/i) as HTMLInputElement;
    expect(notify.checked).toBe(true);
  });

  it('does not submit the action when the rationale is empty', () => {
    renderPage();
    openTakeActionDialog();

    fireEvent.click(screen.getByRole('button', { name: /submit action/i }));

    expect(takeActionMutate).not.toHaveBeenCalled();
    expect(screen.getByText(/a rationale is required/i)).toBeInTheDocument();
  });

  it('submits the selected action as the typed union value with rationale', () => {
    renderPage();
    openTakeActionDialog();

    fireEvent.change(dialog().getByRole('combobox'), { target: { value: 'remove' } });
    fireEvent.change(dialog().getByLabelText(/rationale/i), {
      target: { value: 'violates policy' },
    });
    fireEvent.click(screen.getByRole('button', { name: /submit action/i }));

    expect(takeActionMutate).toHaveBeenCalledTimes(1);
    const [payload] = takeActionMutate.mock.calls[0];
    expect(payload).toMatchObject({
      caseId: 'pending-1',
      request: { action_type: 'remove', rationale: 'violates policy', notify_owner: true },
    });
  });

  it('submits the selected appeal decision as the typed union value with rationale', () => {
    renderPage();
    openDecideAppealDialog();

    fireEvent.change(dialog().getByRole('combobox'), { target: { value: 'overturn' } });
    fireEvent.change(dialog().getByLabelText(/rationale/i), {
      target: { value: 'appeal is valid' },
    });
    fireEvent.click(screen.getByRole('button', { name: /submit decision/i }));

    expect(decideAppealMutate).toHaveBeenCalledTimes(1);
    const [payload] = decideAppealMutate.mock.calls[0];
    expect(payload).toMatchObject({
      caseId: 'appealed-1',
      request: { decision: 'overturn', rationale: 'appeal is valid' },
    });
  });

  // Regression: the take-action dialog must not carry rationale/action from a
  // previously-opened case into the next one opened (key remount guard).
  it('resets the action and rationale when re-opened for a different case', () => {
    renderPage();

    openTakeActionDialog(0);
    fireEvent.change(dialog().getByRole('combobox'), { target: { value: 'remove' } });
    fireEvent.change(dialog().getByLabelText(/rationale/i), {
      target: { value: 'stale rationale for pending-1' },
    });
    fireEvent.click(screen.getByRole('button', { name: /cancel/i }));

    openTakeActionDialog(1);
    expect((dialog().getByRole('combobox') as HTMLSelectElement).value).toBe('approve');
    expect((dialog().getByLabelText(/rationale/i) as HTMLTextAreaElement).value).toBe('');
  });
});
