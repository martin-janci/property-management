/**
 * Onboarding API types (Epic 10B, Story 10B.6).
 *
 * Mirrors the JSON shapes returned by `/api/v1/onboarding/*`.
 */

export interface TourStepData {
  id: string;
  title: string;
  content: string;
  target?: string;
}

export interface OnboardingTour {
  id: string;
  tour_id: string;
  name: string;
  description?: string;
  steps: TourStepData[];
  target_roles?: string[];
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface UserOnboardingProgress {
  id: string;
  user_id: string;
  tour_id: string;
  completed_steps: string[];
  current_step?: string;
  is_completed: boolean;
  is_skipped: boolean;
  started_at: string;
  completed_at?: string;
  created_at: string;
  updated_at: string;
}

export interface TourWithProgress {
  tour: OnboardingTour;
  progress: UserOnboardingProgress | null;
}

export interface OnboardingStatusResponse {
  needs_onboarding: boolean;
  tours: TourWithProgress[];
}

export interface TourProgressResponse {
  progress: UserOnboardingProgress;
}
