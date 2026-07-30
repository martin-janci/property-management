/// <reference types="vitest/globals" />
/**
 * AmlDashboardPage review-decision validation tests.
 *
 * Regression: the Phase-1 review flow reads the decision from window.prompt and
 * previously cast the raw string straight into the `approve | reject | escalate`
 * union (`decision as ...`). A typo ("aprove") therefore reached the API as an
 * invalid AML decision. The page now validates the free-text input against the
 * allowed union before calling the mutation.
 */
import { render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { AmlDashboardPage } from './AmlDashboardPage';

const reviewMutate = vi.fn();

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
  useInitiateEdd: vi.fn(() => ({ mutate: vi.fn() })),
  useReviewAmlAssessment: vi.fn(() => ({ mutate: reviewMutate })),
}));

function clickReview() {
  const button = screen.getByRole('button', { name: /review assessment/i });
  button.click();
}

describe('AmlDashboardPage review-decision validation', () => {
  const promptSpy = vi.spyOn(window, 'prompt');
  const alertSpy = vi.spyOn(window, 'alert').mockImplementation(() => {});

  beforeEach(() => {
    reviewMutate.mockClear();
    promptSpy.mockReset();
    alertSpy.mockClear();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('does not submit when the decision prompt contains an invalid value', () => {
    // First prompt = decision (typo). Notes are also supplied so that the old
    // `decision as ...` cast path would have reached the mutation — the guard is
    // what keeps it from being called.
    promptSpy.mockReturnValueOnce('aprove').mockReturnValueOnce('some notes');
    render(<AmlDashboardPage />);
    clickReview();

    expect(reviewMutate).not.toHaveBeenCalled();
    expect(alertSpy).toHaveBeenCalledWith(expect.stringContaining('Invalid decision'));
  });

  it('submits a valid decision (case/whitespace tolerant) as the typed union value', () => {
    promptSpy
      .mockReturnValueOnce('  Escalate ') // decision — normalised to 'escalate'
      .mockReturnValueOnce('needs more docs'); // notes
    render(<AmlDashboardPage />);
    clickReview();

    expect(reviewMutate).toHaveBeenCalledTimes(1);
    const [payload] = reviewMutate.mock.calls[0];
    expect(payload).toMatchObject({
      assessmentId: 'assess-1',
      request: { decision: 'escalate', notes: 'needs more docs' },
    });
  });
});
