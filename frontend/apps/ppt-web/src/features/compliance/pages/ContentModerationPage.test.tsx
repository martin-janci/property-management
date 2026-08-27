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
import { useModerationCases, useModerationStats } from '@ppt/api-client';
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

/**
 * Regression (#2853, follow-up to #2849): the overdue affordance must reflect
 * the true org-wide breached set, not a client-side narrowing of one fetched
 * page. #2849 filtered `visibleCases` over the already-fetched list, but that
 * list is capped server-side (`clamp_limit`, default 50). On an org with more
 * open cases than one page, the overdue rows can fall beyond the fetched page,
 * so the filtered view under-reported SLA breaches — showing far fewer (or
 * zero) overdue cases than the `overdue_count` badge the moderator clicked.
 *
 * The fix moves the predicate server-side: clicking the alert re-queries the
 * list API with `overdue: true`, and the page renders the server response
 * directly (no client narrowing). These tests model a truncated first page
 * that contains NO overdue rows, and assert the overdue view still shows the
 * full server-side breached set, matching `overdue_count`.
 */
describe('ContentModerationPage overdue affordance', () => {
  // The default (non-overdue) page the API returns: all still-open but FRESH
  // cases. Crucially, none of them are overdue — they stand in for a truncated
  // first page where the overdue rows live beyond the fetched window. Under the
  // old client-side filter this would have yielded zero overdue cases.
  const firstPageNoOverdue = [
    { ...baseCase, id: 'fresh-1', content_preview: 'fresh-one', status: 'pending' },
    { ...baseCase, id: 'fresh-2', content_preview: 'fresh-two', status: 'pending' },
  ];

  // The true org-wide overdue set the server returns for `overdue: true`. Its
  // size matches the stats `overdue_count` badge (2) even though NONE of these
  // rows appear on the default first page above.
  const serverOverdueSet = [
    { ...baseCase, id: 'overdue-p', content_preview: 'overdue-pending', status: 'pending' },
    { ...baseCase, id: 'overdue-r', content_preview: 'overdue-review', status: 'under_review' },
  ];

  beforeEach(() => {
    // Param-aware mock: the `overdue` query flag selects which server page the
    // client receives, exactly as the real endpoint would.
    vi.mocked(useModerationCases).mockImplementation(
      (params?: { overdue?: boolean }) =>
        ({
          data: { cases: params?.overdue ? serverOverdueSet : firstPageNoOverdue },
          isLoading: false,
          error: null,
        }) as unknown as ReturnType<typeof useModerationCases>
    );
    vi.mocked(useModerationStats).mockReturnValue({
      data: {
        stats: {
          pending_count: 2,
          under_review_count: 1,
          by_priority: [],
          by_violation_type: [],
          avg_resolution_time_hours: 0,
          overdue_count: 2,
        },
      },
    } as unknown as ReturnType<typeof useModerationStats>);
  });

  afterEach(() => {
    vi.mocked(useModerationCases).mockReturnValue({
      data: { cases },
      isLoading: false,
      error: null,
    } as unknown as ReturnType<typeof useModerationCases>);
    vi.mocked(useModerationStats).mockReturnValue({
      data: undefined,
    } as unknown as ReturnType<typeof useModerationStats>);
  });

  it('fetches the server-side overdue set (not a truncated client page) when clicked', () => {
    render(
      <ToastProvider>
        <ContentModerationPage />
      </ToastProvider>
    );

    // Before: only the fresh first page is shown; no overdue rows are present.
    expect(screen.getByText('fresh-one')).toBeInTheDocument();
    expect(screen.queryByText('overdue-pending')).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /overdue/i }));

    // The list re-queries with the server-side overdue flag...
    expect(useModerationCases).toHaveBeenCalledWith(expect.objectContaining({ overdue: true }));
    // ...and shows the full breached set (count == overdue_count = 2), even
    // though NONE of these rows were on the fetched first page. The old
    // client-side filter would have shown zero here.
    expect(screen.getByText('overdue-pending')).toBeInTheDocument();
    expect(screen.getByText('overdue-review')).toBeInTheDocument();
    expect(screen.queryByText('fresh-one')).not.toBeInTheDocument();
  });

  it('clears the overdue filter and restores the full list', () => {
    render(
      <ToastProvider>
        <ContentModerationPage />
      </ToastProvider>
    );

    fireEvent.click(screen.getByRole('button', { name: /overdue/i }));
    expect(screen.getByText('overdue-pending')).toBeInTheDocument();
    expect(screen.queryByText('fresh-one')).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /clear overdue filter/i }));

    // Back to the default (non-overdue) query and its first page.
    expect(useModerationCases).toHaveBeenLastCalledWith(
      expect.objectContaining({ overdue: undefined })
    );
    expect(screen.getByText('fresh-one')).toBeInTheDocument();
  });

  /**
   * Regression (#2859): the overdue view requests a single capped page
   * (OVERDUE_PAGE_LIMIT = 200). If an org has more overdue cases than the cap,
   * the list silently truncates below the unbounded `overdue_count` badge —
   * re-introducing the badge-vs-list mismatch #2853 fixed. When the returned
   * overdue page is exactly the cap and the badge is larger, the page must
   * surface the truncation explicitly instead of hiding it.
   */
  it('flags truncation when the overdue page is capped below overdue_count', () => {
    const OVERDUE_PAGE_LIMIT = 200;
    const cappedOverdueSet = Array.from({ length: OVERDUE_PAGE_LIMIT }, (_, i) => ({
      ...baseCase,
      id: `overdue-${i}`,
      content_preview: `overdue-${i}`,
      status: 'pending',
    }));

    vi.mocked(useModerationCases).mockImplementation(
      (params?: { overdue?: boolean }) =>
        ({
          data: { cases: params?.overdue ? cappedOverdueSet : firstPageNoOverdue },
          isLoading: false,
          error: null,
        }) as unknown as ReturnType<typeof useModerationCases>
    );
    vi.mocked(useModerationStats).mockReturnValue({
      data: {
        stats: {
          pending_count: 350,
          under_review_count: 0,
          by_priority: [],
          by_violation_type: [],
          avg_resolution_time_hours: 0,
          overdue_count: 350,
        },
      },
    } as unknown as ReturnType<typeof useModerationStats>);

    render(
      <ToastProvider>
        <ContentModerationPage />
      </ToastProvider>
    );

    // No truncation notice on the default (non-overdue) page.
    expect(screen.queryByText(/first 200 of 350/i)).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /overdue/i }));

    // The capped page (length === limit) with an unbounded badge of 350 must
    // surface the truncation explicitly rather than silently dropping rows.
    expect(screen.getByText(/first 200 of 350/i)).toBeInTheDocument();
  });

  /**
   * Boundary regression (#2862): the truncation notice keys off the badge
   * count strictly exceeding the cap, NOT merely the page filling to the cap.
   * When an org has *exactly* OVERDUE_PAGE_LIMIT (200) overdue cases, the page
   * fetches 200, `cases.length === 200`, but nothing is truncated — the list is
   * complete. The notice must NOT fire (it previously did, telling the user to
   * "refine the filters" for no reason — a false positive at the boundary).
   */
  it('does not flag truncation when overdue_count equals the cap exactly', () => {
    const OVERDUE_PAGE_LIMIT = 200;
    const exactCapOverdueSet = Array.from({ length: OVERDUE_PAGE_LIMIT }, (_, i) => ({
      ...baseCase,
      id: `overdue-${i}`,
      content_preview: `overdue-${i}`,
      status: 'pending',
    }));

    vi.mocked(useModerationCases).mockImplementation(
      (params?: { overdue?: boolean }) =>
        ({
          data: { cases: params?.overdue ? exactCapOverdueSet : firstPageNoOverdue },
          isLoading: false,
          error: null,
        }) as unknown as ReturnType<typeof useModerationCases>
    );
    vi.mocked(useModerationStats).mockReturnValue({
      data: {
        stats: {
          pending_count: 200,
          under_review_count: 0,
          by_priority: [],
          by_violation_type: [],
          avg_resolution_time_hours: 0,
          overdue_count: 200,
        },
      },
    } as unknown as ReturnType<typeof useModerationStats>);

    render(
      <ToastProvider>
        <ContentModerationPage />
      </ToastProvider>
    );

    fireEvent.click(screen.getByRole('button', { name: /overdue/i }));

    // 200 fetched, 200 total overdue — the list is complete, so no truncation
    // notice regardless of the page filling exactly to the cap.
    expect(screen.queryByText(/first 200 of 200/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/first 200 of/i)).not.toBeInTheDocument();
  });
});
