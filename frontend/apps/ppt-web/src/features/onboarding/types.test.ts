/// <reference types="vitest/globals" />
/**
 * Unit tests for the onboarding feature pure helpers.
 * Epic 10B: User Onboarding (Story 10B.6 / UC-42.1).
 *
 * The onboarding UI is an intentionally *unwired* feature (see
 * `features/unwired-features.ts` — board decision PAP-55: keep the code, do not
 * expose the routes). These helpers are the props-driving logic behind the
 * presentational `TourCard` / `TourStep` / `OnboardingProgress` components, so
 * they are worth locking down even while the surface stays behind the wire.
 */

import { describe, expect, it } from 'vitest';
import {
  calculateTourProgress,
  formatVideoDuration,
  getCurrentStepIndex,
  getTourDisplayStatus,
  type OnboardingTour,
  type UserOnboardingProgress,
} from './types';

const makeTour = (stepIds: string[]): OnboardingTour => ({
  id: 'tour-uuid',
  tourId: 'main-onboarding',
  name: 'Main onboarding',
  description: 'Getting started',
  steps: stepIds.map((id) => ({ id, title: id, content: `content-${id}` })),
  targetRoles: ['manager'],
  isActive: true,
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-01T00:00:00Z',
});

const makeProgress = (overrides: Partial<UserOnboardingProgress> = {}): UserOnboardingProgress => ({
  id: 'progress-uuid',
  userId: 'user-uuid',
  tourId: 'main-onboarding',
  completedSteps: [],
  currentStep: undefined,
  isCompleted: false,
  isSkipped: false,
  startedAt: '2026-01-01T00:00:00Z',
  completedAt: undefined,
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-01T00:00:00Z',
  ...overrides,
});

describe('calculateTourProgress', () => {
  const tour = makeTour(['s1', 's2', 's3', 's4']);

  it('is 0% when there is no progress yet', () => {
    expect(calculateTourProgress(tour, null)).toBe(0);
  });

  it('is 100% for a completed tour regardless of completed step count', () => {
    expect(calculateTourProgress(tour, makeProgress({ isCompleted: true }))).toBe(100);
  });

  it('is 0% for a skipped tour even with some completed steps', () => {
    expect(
      calculateTourProgress(tour, makeProgress({ isSkipped: true, completedSteps: ['s1', 's2'] }))
    ).toBe(0);
  });

  it('rounds the completed/total ratio to the nearest percent', () => {
    // 2 of 4 steps -> 50%
    expect(calculateTourProgress(tour, makeProgress({ completedSteps: ['s1', 's2'] }))).toBe(50);
    // 1 of 3 steps -> 33 (Math.round(33.33))
    expect(
      calculateTourProgress(makeTour(['a', 'b', 'c']), makeProgress({ completedSteps: ['a'] }))
    ).toBe(33);
  });

  it('treats a tour with zero steps as 100% complete', () => {
    expect(calculateTourProgress(makeTour([]), makeProgress())).toBe(100);
  });
});

describe('getCurrentStepIndex', () => {
  const tour = makeTour(['s1', 's2', 's3']);

  it('is 0 when there is no progress', () => {
    expect(getCurrentStepIndex(tour, null)).toBe(0);
  });

  it('is 0 when progress has no current step', () => {
    expect(getCurrentStepIndex(tour, makeProgress({ currentStep: undefined }))).toBe(0);
  });

  it('returns the index of the current step', () => {
    expect(getCurrentStepIndex(tour, makeProgress({ currentStep: 's2' }))).toBe(1);
    expect(getCurrentStepIndex(tour, makeProgress({ currentStep: 's3' }))).toBe(2);
  });

  it('falls back to 0 when the current step is not found in the tour', () => {
    expect(getCurrentStepIndex(tour, makeProgress({ currentStep: 'does-not-exist' }))).toBe(0);
  });
});

describe('getTourDisplayStatus', () => {
  it('is not_started when there is no progress', () => {
    expect(getTourDisplayStatus(null)).toBe('not_started');
  });

  it('is completed when the tour is completed (wins over other flags)', () => {
    expect(
      getTourDisplayStatus(
        makeProgress({ isCompleted: true, isSkipped: true, completedSteps: ['s1'] })
      )
    ).toBe('completed');
  });

  it('is skipped when skipped but not completed', () => {
    expect(getTourDisplayStatus(makeProgress({ isSkipped: true, completedSteps: ['s1'] }))).toBe(
      'skipped'
    );
  });

  it('is in_progress when some steps are done but not completed/skipped', () => {
    expect(getTourDisplayStatus(makeProgress({ completedSteps: ['s1'] }))).toBe('in_progress');
  });

  it('is not_started when progress exists but no steps are completed', () => {
    expect(getTourDisplayStatus(makeProgress({ completedSteps: [] }))).toBe('not_started');
  });
});

describe('formatVideoDuration', () => {
  it('formats sub-minute durations with a leading 0:', () => {
    expect(formatVideoDuration(5)).toBe('0:05');
    expect(formatVideoDuration(45)).toBe('0:45');
  });

  it('formats whole minutes', () => {
    expect(formatVideoDuration(60)).toBe('1:00');
    expect(formatVideoDuration(600)).toBe('10:00');
  });

  it('zero-pads the seconds component', () => {
    expect(formatVideoDuration(65)).toBe('1:05');
    expect(formatVideoDuration(129)).toBe('2:09');
  });

  it('handles zero', () => {
    expect(formatVideoDuration(0)).toBe('0:00');
  });
});
