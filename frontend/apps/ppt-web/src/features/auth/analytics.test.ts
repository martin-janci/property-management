/**
 * Unit tests for the ppt-web signup-funnel analytics helpers (issue #2530).
 * Locks in the canonical `signup.*` event names + payloads and the shared bus
 * safety guarantees (dataLayer mirroring, never-throws).
 */
/// <reference types="vitest/globals" />
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import {
  __clearAnalyticsSinks,
  type AnalyticsEvent,
  registerAnalyticsSink,
} from '../../lib/analytics';
import {
  SIGNUP_EVENTS,
  trackEmailVerified,
  trackRegistrationSubmitted,
  trackSignupLoggedIn,
} from './analytics';

let captured: AnalyticsEvent[];

beforeEach(() => {
  captured = [];
  registerAnalyticsSink((e) => captured.push(e));
});

afterEach(() => {
  __clearAnalyticsSinks();
  (globalThis as { dataLayer?: unknown[] }).dataLayer = undefined;
});

describe('ppt-web signup funnel analytics', () => {
  it('fires registration_submitted with the signup method', () => {
    trackRegistrationSubmitted();
    expect(captured).toHaveLength(1);
    expect(captured[0].event).toBe(SIGNUP_EVENTS.registrationSubmitted);
    expect(captured[0].properties).toMatchObject({ method: 'email_password' });
    expect(captured[0].timestamp).toEqual(expect.any(String));
  });

  it('fires email_verified and logged_in with an overridable method', () => {
    trackEmailVerified();
    trackSignupLoggedIn('sso');
    expect(captured.map((e) => e.event)).toEqual([
      SIGNUP_EVENTS.emailVerified,
      SIGNUP_EVENTS.loggedIn,
    ]);
    expect(captured[1].properties).toMatchObject({ method: 'sso' });
  });

  it('uses names identical to the admin-web schema so the funnel stitches', () => {
    expect(SIGNUP_EVENTS).toEqual({
      registrationSubmitted: 'signup.registration_submitted',
      emailVerified: 'signup.email_verified',
      loggedIn: 'signup.logged_in',
    });
  });

  it('mirrors events onto window.dataLayer when present', () => {
    (globalThis as { dataLayer?: unknown[] }).dataLayer = [];
    trackRegistrationSubmitted();
    const dl = (globalThis as { dataLayer?: Array<Record<string, unknown>> }).dataLayer;
    expect(dl).toHaveLength(1);
    expect(dl?.[0]).toMatchObject({
      event: SIGNUP_EVENTS.registrationSubmitted,
      method: 'email_password',
    });
  });

  it('never throws when a sink misbehaves', () => {
    registerAnalyticsSink(() => {
      throw new Error('boom');
    });
    expect(() => trackSignupLoggedIn()).not.toThrow();
    expect(captured).toHaveLength(1);
  });
});
