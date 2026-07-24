/**
 * Onboarding / signup funnel instrumentation (Epic 10B, Story 10B.6).
 *
 * The onboarding tour is the last leg of the signup funnel. Until now the
 * TourOverlay drove `start` / `complete-step` / `complete` / `skip` / `reset`
 * mutations but fired no analytics, so the funnel was invisible to the data
 * team. These helpers emit one canonical event per funnel transition, fired
 * from `OnboardingToursPage` only after the corresponding server mutation
 * succeeds (so events count real conversions, not attempts).
 */

import { trackEvent } from '../../lib/analytics';

/** Canonical funnel event names — keep in sync with the data-team schema. */
export const ONBOARDING_EVENTS = {
  tourStarted: 'onboarding.tour_started',
  stepCompleted: 'onboarding.step_completed',
  tourCompleted: 'onboarding.tour_completed',
  tourSkipped: 'onboarding.tour_skipped',
  tourReset: 'onboarding.tour_reset',
} as const;

export type OnboardingEventName = (typeof ONBOARDING_EVENTS)[keyof typeof ONBOARDING_EVENTS];

interface TourRef {
  tourId: string;
  tourName: string;
}

export function trackTourStarted({ tourId, tourName }: TourRef): void {
  trackEvent(ONBOARDING_EVENTS.tourStarted, { tour_id: tourId, tour_name: tourName });
}

export function trackStepCompleted(
  { tourId, tourName }: TourRef,
  stepId: string,
  isLast: boolean
): void {
  trackEvent(ONBOARDING_EVENTS.stepCompleted, {
    tour_id: tourId,
    tour_name: tourName,
    step_id: stepId,
    is_last_step: isLast,
  });
}

export function trackTourCompleted({ tourId, tourName }: TourRef): void {
  trackEvent(ONBOARDING_EVENTS.tourCompleted, { tour_id: tourId, tour_name: tourName });
}

export function trackTourSkipped({ tourId, tourName }: TourRef): void {
  trackEvent(ONBOARDING_EVENTS.tourSkipped, { tour_id: tourId, tour_name: tourName });
}

export function trackTourReset({ tourId, tourName }: TourRef): void {
  trackEvent(ONBOARDING_EVENTS.tourReset, { tour_id: tourId, tour_name: tourName });
}
