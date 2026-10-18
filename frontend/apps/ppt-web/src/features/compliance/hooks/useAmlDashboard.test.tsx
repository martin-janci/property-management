/// <reference types="vitest/globals" />
/**
 * useAmlDashboard — null-safe risk_factors transform regression.
 *
 * The API→display transform mapped `a.risk_factors.map(...)` with no guard. The
 * TypeScript type marks `risk_factors` as a non-nullable array, but the API can
 * return `null` (or omit the field) for an assessment with no computed factors,
 * so the unguarded `.map` threw `Cannot read properties of null (reading 'map')`
 * during render and crashed the whole AML dashboard.
 *
 * This test drives the hook with an assessment whose `risk_factors` is null and
 * asserts the hook renders and normalizes it to an empty array. It throws on the
 * pre-fix (unguarded) version and passes once the `?? []` guard is in place.
 */
import { renderHook } from '@testing-library/react';
import type { ReactNode } from 'react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { ToastProvider } from '../../../components';
import { useAmlDashboard } from './useAmlDashboard';

// An assessment with no computed risk factors — the API sends null here, which
// the non-nullable TS type does not reflect.
const assessmentWithNullFactors = {
  id: 'assess-null',
  subject_id: 'party-null',
  subject_type: 'user',
  risk_score: 42,
  risk_level: 'medium',
  status: 'completed',
  risk_factors: null,
  flagged_for_review: false,
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-02T00:00:00Z',
};

vi.mock('@ppt/api-client', () => ({
  useAmlAssessments: vi.fn(() => ({
    data: { assessments: [assessmentWithNullFactors] },
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

describe('useAmlDashboard risk_factors transform', () => {
  afterEach(() => {
    vi.clearAllMocks();
  });

  it('does not crash and normalizes null risk_factors to an empty array', () => {
    const { result } = renderHook(() => useAmlDashboard(), { wrapper });

    expect(result.current.assessments).toHaveLength(1);
    expect(result.current.assessments[0].risk_factors).toEqual([]);
  });
});
