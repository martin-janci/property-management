/**
 * Signup-funnel instrumentation for the ppt-web (manager) auth flows
 * (issue #2530).
 *
 * PR #2515 instrumented only the onboarding-tour leg (in admin-web). The
 * earlier funnel legs live in ppt-web: registration submit (`RegisterPage`)
 * and first login (`AuthContext.login`). These helpers emit one canonical
 * `signup.*` event per transition, fired only after the corresponding server
 * call succeeds (so events count real conversions, not attempts).
 *
 * The event names MUST stay identical to admin-web's
 * `features/onboarding/analytics.ts` `SIGNUP_EVENTS` so the data team can
 * stitch both apps into one funnel.
 */

import { trackEvent } from '../../lib/analytics';

/** Canonical signup funnel event names — keep in sync with admin-web + the data-team schema. */
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
