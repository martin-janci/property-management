/**
 * Onboarding / signup funnel instrumentation (Epic 10B, Story 10B.6).
 *
 * The onboarding tour is the *last* leg of the signup funnel. Until now the
 * TourOverlay drove `start` / `complete-step` / `complete` / `skip` / `reset`
 * mutations but fired no analytics, so the funnel was invisible to the data
 * team. These helpers emit one canonical event per funnel transition, fired
 * from `OnboardingToursPage` only after the corresponding server mutation
 * succeeds (so events count real conversions, not attempts).
 *
 * The earlier funnel legs — registration submit, email verification and first
 * login — are captured by the `signup.*` events below (issue #2530). Their
 * call-sites live in the auth flows: the admin login page (`LoginPage`) and,
 * for the manager app, ppt-web's `RegisterPage` + `AuthContext.login` (which
 * emit the same canonical names through ppt-web's own copy of the analytics
 * bus, since Vite apps can't cross-import). Keeping every funnel name in one
 * schema block lets the data team stitch the stages together end-to-end.
 */

import { trackEvent } from '../../lib/analytics';

/** Canonical onboarding-tour funnel event names — keep in sync with the data-team schema. */
export const ONBOARDING_EVENTS = {
  tourStarted: 'onboarding.tour_started',
  stepCompleted: 'onboarding.step_completed',
  tourCompleted: 'onboarding.tour_completed',
  tourSkipped: 'onboarding.tour_skipped',
  tourReset: 'onboarding.tour_reset',
} as const;

export type OnboardingEventName = (typeof ONBOARDING_EVENTS)[keyof typeof ONBOARDING_EVENTS];

/**
 * Canonical *signup* funnel event names — the earlier legs that precede the
 * onboarding tour. Emitted after the corresponding auth mutation succeeds so
 * the funnel counts real conversions (registered → verified → logged in →
 * onboarding tour), not attempts. Keep in sync with the data-team schema.
 */
export const SIGNUP_EVENTS = {
  registrationSubmitted: 'signup.registration_submitted',
  emailVerified: 'signup.email_verified',
  loggedIn: 'signup.logged_in',
} as const;

export type SignupEventName = (typeof SIGNUP_EVENTS)[keyof typeof SIGNUP_EVENTS];

/** Signup methods a funnel event can be attributed to (no PII in properties). */
export type SignupMethod = 'email_password' | 'sso';

/** Registration form submitted and accepted by the server (UC-14.1). */
export function trackRegistrationSubmitted(method: SignupMethod = 'email_password'): void {
  trackEvent(SIGNUP_EVENTS.registrationSubmitted, { method });
}

/** Email verification link confirmed by the server. */
export function trackEmailVerified(): void {
  trackEvent(SIGNUP_EVENTS.emailVerified, {});
}

/** A successful login (the funnel counts the first, but the event is idempotent). */
export function trackSignupLoggedIn(method: SignupMethod = 'email_password'): void {
  trackEvent(SIGNUP_EVENTS.loggedIn, { method });
}

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
