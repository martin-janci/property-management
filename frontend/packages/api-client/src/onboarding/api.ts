/**
 * Onboarding API client (Epic 10B, Story 10B.6).
 *
 * Functions for `/api/v1/onboarding/*` endpoints.
 * Auth via the shared `authenticatedFetchJson` helper.
 */

import { authenticatedFetchJson } from '../lib/fetch';
import type {
  OnboardingStatusResponse,
  OnboardingTour,
  TourProgressResponse,
  TourWithProgress,
} from './types';

const _win = typeof window !== 'undefined' ? (window as unknown as Record<string, unknown>) : {};
const ONBOARDING_BASE = `${_win.__API_BASE_URL__ ? String(_win.__API_BASE_URL__) : ''}/api/v1/onboarding`;

/**
 * GET /api/v1/onboarding/status
 */
export async function fetchOnboardingStatus(
  signal?: AbortSignal
): Promise<OnboardingStatusResponse> {
  return authenticatedFetchJson<OnboardingStatusResponse>(`${ONBOARDING_BASE}/status`, { signal });
}

/**
 * GET /api/v1/onboarding/tours
 */
export async function fetchOnboardingTours(signal?: AbortSignal): Promise<TourWithProgress[]> {
  return authenticatedFetchJson<TourWithProgress[]>(`${ONBOARDING_BASE}/tours`, { signal });
}

/**
 * GET /api/v1/onboarding/tours/{tour_id}
 */
export async function fetchOnboardingTour(
  tourId: string,
  signal?: AbortSignal
): Promise<OnboardingTour> {
  return authenticatedFetchJson<OnboardingTour>(
    `${ONBOARDING_BASE}/tours/${encodeURIComponent(tourId)}`,
    {
      signal,
    }
  );
}

/**
 * POST /api/v1/onboarding/tours/{tour_id}/start
 */
export async function startOnboardingTour(tourId: string): Promise<TourProgressResponse> {
  return authenticatedFetchJson<TourProgressResponse>(
    `${ONBOARDING_BASE}/tours/${encodeURIComponent(tourId)}/start`,
    { method: 'POST' }
  );
}

/**
 * POST /api/v1/onboarding/tours/{tour_id}/steps/{step_id}/complete
 */
export async function completeOnboardingStep(
  tourId: string,
  stepId: string
): Promise<TourProgressResponse> {
  return authenticatedFetchJson<TourProgressResponse>(
    `${ONBOARDING_BASE}/tours/${encodeURIComponent(tourId)}/steps/${encodeURIComponent(stepId)}/complete`,
    { method: 'POST' }
  );
}

/**
 * POST /api/v1/onboarding/tours/{tour_id}/complete
 */
export async function completeOnboardingTour(tourId: string): Promise<TourProgressResponse> {
  return authenticatedFetchJson<TourProgressResponse>(
    `${ONBOARDING_BASE}/tours/${encodeURIComponent(tourId)}/complete`,
    { method: 'POST' }
  );
}

/**
 * POST /api/v1/onboarding/tours/{tour_id}/skip
 */
export async function skipOnboardingTour(tourId: string): Promise<TourProgressResponse> {
  return authenticatedFetchJson<TourProgressResponse>(
    `${ONBOARDING_BASE}/tours/${encodeURIComponent(tourId)}/skip`,
    { method: 'POST' }
  );
}

/**
 * POST /api/v1/onboarding/tours/{tour_id}/reset
 */
export async function resetOnboardingTour(tourId: string): Promise<TourProgressResponse> {
  return authenticatedFetchJson<TourProgressResponse>(
    `${ONBOARDING_BASE}/tours/${encodeURIComponent(tourId)}/reset`,
    { method: 'POST' }
  );
}
