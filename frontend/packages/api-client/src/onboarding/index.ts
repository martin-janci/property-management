/**
 * Onboarding Module (Epic 10B, Story 10B.6).
 *
 * API client + hooks for `/api/v1/onboarding/*` endpoints.
 */

export {
  completeOnboardingStep,
  completeOnboardingTour,
  fetchOnboardingStatus,
  fetchOnboardingTour,
  fetchOnboardingTours,
  resetOnboardingTour,
  skipOnboardingTour,
  startOnboardingTour,
} from './api';
export {
  onboardingKeys,
  useCompleteOnboardingStep,
  useCompleteOnboardingTour,
  useOnboardingStatus,
  useOnboardingTour,
  useOnboardingTours,
  useResetOnboardingTour,
  useSkipOnboardingTour,
  useStartOnboardingTour,
} from './hooks';
export type {
  OnboardingStatusResponse,
  OnboardingTour,
  TourProgressResponse,
  TourStepData,
  TourWithProgress,
  UserOnboardingProgress,
} from './types';
