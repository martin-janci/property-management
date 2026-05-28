/**
 * Admin-web onboarding feature types (Epic 10B, Story 10B.6).
 *
 * Re-exports shared API types and adds admin-web specific helpers.
 */

export type {
  OnboardingStatusResponse,
  OnboardingTour,
  TourProgressResponse,
  TourStepData,
  TourWithProgress,
  UserOnboardingProgress,
} from '@ppt/api-client';

/**
 * Derived display status of a tour.
 */
export type TourDisplayStatus = 'not_started' | 'in_progress' | 'completed' | 'skipped';

export function getTourDisplayStatus(
  progress: { is_completed: boolean; is_skipped: boolean; completed_steps: string[] } | null
): TourDisplayStatus {
  if (!progress) return 'not_started';
  if (progress.is_completed) return 'completed';
  if (progress.is_skipped) return 'skipped';
  if (progress.completed_steps.length > 0) return 'in_progress';
  return 'not_started';
}

export function calculateTourProgress(
  stepsCount: number,
  progress: { is_completed: boolean; completed_steps: string[] } | null
): number {
  if (!progress) return 0;
  if (progress.is_completed) return 100;
  if (stepsCount === 0) return 100;
  return Math.round((progress.completed_steps.length / stepsCount) * 100);
}

export function getCurrentStepIndex(
  steps: { id: string }[],
  progress: { current_step?: string } | null
): number {
  if (!progress?.current_step) return 0;
  const idx = steps.findIndex((s) => s.id === progress.current_step);
  return idx >= 0 ? idx : 0;
}
