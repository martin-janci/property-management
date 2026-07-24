/**
 * Regression test for the onboarding/signup funnel instrumentation
 * (Epic 10B, Story 10B.6). Before this change the tour fired no analytics
 * events; these assertions lock in that every funnel transition emits its
 * canonical event with the expected tour context.
 */
/// <reference types="vitest/globals" />
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  __clearAnalyticsSinks,
  type AnalyticsEvent,
  registerAnalyticsSink,
} from '../../lib/analytics';
import {
  ONBOARDING_EVENTS,
  trackStepCompleted,
  trackTourCompleted,
  trackTourReset,
  trackTourSkipped,
  trackTourStarted,
} from './analytics';

const TOUR = { tourId: 'tour-welcome', tourName: 'Welcome tour' };

let captured: AnalyticsEvent[];

beforeEach(() => {
  captured = [];
  registerAnalyticsSink((e) => captured.push(e));
});

afterEach(() => {
  __clearAnalyticsSinks();
  vi.restoreAllMocks();
});

describe('onboarding funnel analytics', () => {
  it('fires tour_started with tour context', () => {
    trackTourStarted(TOUR);
    expect(captured).toHaveLength(1);
    expect(captured[0].event).toBe(ONBOARDING_EVENTS.tourStarted);
    expect(captured[0].properties).toMatchObject({
      tour_id: 'tour-welcome',
      tour_name: 'Welcome tour',
    });
    expect(captured[0].timestamp).toEqual(expect.any(String));
  });

  it('fires step_completed with step id and last-step flag', () => {
    trackStepCompleted(TOUR, 'step-2', false);
    trackStepCompleted(TOUR, 'step-3', true);
    expect(captured.map((e) => e.event)).toEqual([
      ONBOARDING_EVENTS.stepCompleted,
      ONBOARDING_EVENTS.stepCompleted,
    ]);
    expect(captured[0].properties).toMatchObject({ step_id: 'step-2', is_last_step: false });
    expect(captured[1].properties).toMatchObject({ step_id: 'step-3', is_last_step: true });
  });

  it('fires tour_completed, tour_skipped and tour_reset', () => {
    trackTourCompleted(TOUR);
    trackTourSkipped(TOUR);
    trackTourReset(TOUR);
    expect(captured.map((e) => e.event)).toEqual([
      ONBOARDING_EVENTS.tourCompleted,
      ONBOARDING_EVENTS.tourSkipped,
      ONBOARDING_EVENTS.tourReset,
    ]);
    for (const e of captured) {
      expect(e.properties).toMatchObject({ tour_id: 'tour-welcome', tour_name: 'Welcome tour' });
    }
  });

  it('mirrors events onto window.dataLayer when present', () => {
    (globalThis as { dataLayer?: unknown[] }).dataLayer = [];
    trackTourStarted(TOUR);
    const dl = (globalThis as { dataLayer?: Array<Record<string, unknown>> }).dataLayer;
    expect(dl).toHaveLength(1);
    expect(dl?.[0]).toMatchObject({
      event: ONBOARDING_EVENTS.tourStarted,
      tour_id: 'tour-welcome',
    });
    (globalThis as { dataLayer?: unknown[] }).dataLayer = undefined;
  });

  it('never throws when a sink misbehaves', () => {
    registerAnalyticsSink(() => {
      throw new Error('boom');
    });
    expect(() => trackTourCompleted(TOUR)).not.toThrow();
    // The well-behaved sink still received the event.
    expect(captured).toHaveLength(1);
  });
});
