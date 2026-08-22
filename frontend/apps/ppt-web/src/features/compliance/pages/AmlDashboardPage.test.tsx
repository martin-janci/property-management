/// <reference types="vitest/globals" />
/**
 * AmlDashboardPage EDD / review decision-flow tests.
 *
 * History: the Phase-1 EDD + review flow drove regulated decisions through
 * `window.prompt` / `window.alert` with hardcoded English copy, and cast the raw
 * prompt string straight into the `approve | reject | escalate` union (a typo
 * such as "aprove" could reach the API). Epic 90 replaced that flow with in-app
 * modal dialogs (localized, using the shared Toast for feedback):
 *   - the decision is now constrained to the union via a <select>, so an invalid
 *     free-text decision is structurally impossible, and
 *   - the mutation still fires only with a non-empty reason (EDD) / non-empty
 *     notes (review), matching the old `if (!x) return;` guards.
 *
 * These tests lock in: no window.prompt/window.alert is used, required fields
 * gate the mutation, and a valid submission sends the typed union value.
 */
import { fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ToastProvider } from '../../../components';
import { AmlDashboardPage } from './AmlDashboardPage';

const reviewMutate = vi.fn();
const eddMutate = vi.fn();

const assessment = {
  id: 'assess-1',
  subject_id: 'party-1',
  subject_type: 'tenant',
  risk_score: 80,
  risk_level: 'high',
  status: 'requires_review',
  risk_factors: [],
  flagged_for_review: true,
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-02T00:00:00Z',
};

vi.mock('@ppt/api-client', () => ({
  useAmlAssessments: vi.fn(() => ({
    data: { assessments: [assessment] },
    isLoading: false,
    error: null,
  })),
  useAmlThresholds: vi.fn(() => ({ data: undefined })),
  useCountryRisks: vi.fn(() => ({ data: undefined })),
  useInitiateEdd: vi.fn(() => ({ mutate: eddMutate, isPending: false })),
  useReviewAmlAssessment: vi.fn(() => ({ mutate: reviewMutate, isPending: false })),
}));

function renderPage() {
  return render(
    <ToastProvider>
      <AmlDashboardPage />
    </ToastProvider>
  );
}

function openReviewDialog() {
  fireEvent.click(screen.getByRole('button', { name: /review assessment/i }));
}

function openEddDialog() {
  fireEvent.click(screen.getByRole('button', { name: /initiate edd/i }));
}

describe('AmlDashboardPage decision flow', () => {
  const promptSpy = vi.spyOn(window, 'prompt');
  const alertSpy = vi.spyOn(window, 'alert').mockImplementation(() => {});

  beforeEach(() => {
    reviewMutate.mockClear();
    eddMutate.mockClear();
    promptSpy.mockClear();
    alertSpy.mockClear();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('never uses window.prompt or window.alert for the review flow', () => {
    renderPage();
    openReviewDialog();

    // The decision dialog is rendered in-app.
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(promptSpy).not.toHaveBeenCalled();
    expect(alertSpy).not.toHaveBeenCalled();
  });

  it('does not submit the review when notes are empty', () => {
    renderPage();
    openReviewDialog();

    // Submit with the default decision but no notes.
    fireEvent.click(screen.getByRole('button', { name: /submit decision/i }));

    expect(reviewMutate).not.toHaveBeenCalled();
    // A localized inline validation message is shown instead of an alert.
    expect(screen.getByText(/review notes are required/i)).toBeInTheDocument();
  });

  it('submits the selected decision as the typed union value with notes', () => {
    renderPage();
    openReviewDialog();

    const select = screen.getByLabelText(/decision/i);
    fireEvent.change(select, { target: { value: 'escalate' } });

    const notes = screen.getByLabelText(/review notes/i);
    fireEvent.change(notes, { target: { value: 'needs more docs' } });

    fireEvent.click(screen.getByRole('button', { name: /submit decision/i }));

    expect(reviewMutate).toHaveBeenCalledTimes(1);
    const [payload] = reviewMutate.mock.calls[0];
    expect(payload).toMatchObject({
      assessmentId: 'assess-1',
      request: { decision: 'escalate', notes: 'needs more docs' },
    });
  });

  it('does not submit EDD when the reason is empty, and submits it when provided', () => {
    renderPage();
    openEddDialog();

    // Two buttons match /initiate edd/i once the dialog is open (the card action
    // + the dialog submit); the dialog submit is the last one in the DOM.
    const submitButtons = screen.getAllByRole('button', { name: /initiate edd/i });
    const dialogSubmit = submitButtons[submitButtons.length - 1];

    // Empty reason: no mutation, inline validation shown.
    fireEvent.click(dialogSubmit);
    expect(eddMutate).not.toHaveBeenCalled();
    expect(screen.getByText(/a reason is required/i)).toBeInTheDocument();

    // Provide a reason and submit.
    fireEvent.change(screen.getByLabelText(/reason/i), {
      target: { value: 'high risk score' },
    });
    fireEvent.click(dialogSubmit);

    expect(eddMutate).toHaveBeenCalledTimes(1);
    const [payload] = eddMutate.mock.calls[0];
    expect(payload).toMatchObject({
      assessment_id: 'assess-1',
      reason: 'high risk score',
      documents_requested: [],
    });
  });
});
