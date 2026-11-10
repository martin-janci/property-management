/// <reference types="vitest/globals" />
/**
 * useAmlDashboard — AML-check flags are wired from the backend row.
 *
 * Regression for the code-review finding that the hook hardcoded
 * `id_verified` / `source_of_funds_documented` / `pep_check_completed` /
 * `sanctions_check_completed` to `false` regardless of the API response. The
 * backend list endpoint serializes these booleans from the assessment row, so
 * the hook must surface the real values.
 *
 * This test drives the hook with an assessment whose check flags are all
 * `true` and asserts the display assessment reflects them. It fails on the
 * pre-fix (hardcoded-`false`) hook and passes once the flags are read from the
 * response.
 */
import { renderHook } from '@testing-library/react';
import type { ReactNode } from 'react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { ToastProvider } from '../../../components';
import { useAmlDashboard } from './useAmlDashboard';

// A fully-screened assessment: every AML check completed on the backend row.
const screenedAssessment = {
  id: 'assess-screened',
  subject_id: 'party-screened',
  subject_type: 'user',
  risk_score: 80,
  risk_level: 'high',
  status: 'completed',
  risk_factors: [],
  flagged_for_review: true,
  id_verified: true,
  source_of_funds_documented: true,
  pep_check_completed: true,
  sanctions_check_completed: true,
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-02T00:00:00Z',
};

// An assessment with the check flags omitted — the untyped list payload can do
// this for rows predating the columns; the hook must default to `false`.
const unscreenedAssessment = {
  id: 'assess-unscreened',
  subject_id: 'party-unscreened',
  subject_type: 'user',
  risk_score: 10,
  risk_level: 'low',
  status: 'completed',
  risk_factors: [],
  flagged_for_review: false,
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-02T00:00:00Z',
};

vi.mock('@ppt/api-client', () => ({
  useAmlAssessments: vi.fn(() => ({
    data: { assessments: [screenedAssessment, unscreenedAssessment] },
    isLoading: false,
    error: null,
  })),
  useAmlThresholds: vi.fn(() => ({ data: undefined })),
  useCountryRisks: vi.fn(() => ({ data: undefined })),
  useInitiateEdd: vi.fn(() => ({ mutate: vi.fn(), isPending: false })),
  useReviewAmlAssessment: vi.fn(() => ({ mutate: vi.fn(), isPending: false })),
}));

function wrapper({ children }: { children: ReactNode }) {
  return <ToastProvider>{children}</ToastProvider>;
}

describe('useAmlDashboard AML-check flags', () => {
  afterEach(() => {
    vi.clearAllMocks();
  });

  it('surfaces the backend check flags instead of hardcoding false', () => {
    const { result } = renderHook(() => useAmlDashboard(), { wrapper });

    const screened = result.current.assessments[0];
    expect(screened.id_verified).toBe(true);
    expect(screened.source_of_funds_documented).toBe(true);
    expect(screened.pep_check_completed).toBe(true);
    expect(screened.sanctions_check_completed).toBe(true);
  });

  it('defaults missing check flags to false', () => {
    const { result } = renderHook(() => useAmlDashboard(), { wrapper });

    const unscreened = result.current.assessments[1];
    expect(unscreened.id_verified).toBe(false);
    expect(unscreened.source_of_funds_documented).toBe(false);
    expect(unscreened.pep_check_completed).toBe(false);
    expect(unscreened.sanctions_check_completed).toBe(false);
  });
});
