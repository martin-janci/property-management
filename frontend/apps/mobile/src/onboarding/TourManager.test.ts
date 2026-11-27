/**
 * Coverage for the interactive onboarding tour engine (Epic 50 - Story 50.1).
 *
 * `TourManager` drives the whole onboarding flow — which step the user is on,
 * whether a tour was skipped or completed, whether onboarding should be shown
 * at all, and per-screen contextual-help dismissal — and persists every
 * transition into AsyncStorage under `@ppt/onboarding_state`. None of that was
 * exercised by a test, so regressions in step advancement, the
 * "all done" flag, or persistence/restore could ship silently.
 *
 * These tests use a stateful in-memory AsyncStorage double so the full
 * save -> reload round trip is verified, not just that `setItem` was called.
 */
import AsyncStorage from '@react-native-async-storage/async-storage';

import { TourManager } from './TourManager';

const ONBOARDING_STATE_KEY = '@ppt/onboarding_state';

const getItem = jest.mocked(AsyncStorage.getItem);
const setItem = jest.mocked(AsyncStorage.setItem);

// Stateful in-memory backing store so saveState()/loadState() round-trip for real.
let store: Record<string, string>;

beforeEach(() => {
  store = {};
  getItem.mockReset();
  setItem.mockReset();
  getItem.mockImplementation(async (key: string) => store[key] ?? null);
  setItem.mockImplementation(async (key: string, value: string) => {
    store[key] = value;
  });
});

function makeManager(): TourManager {
  return new TourManager();
}

describe('TourManager default tours', () => {
  it('registers the built-in tours on construction', () => {
    const manager = makeManager();

    const ids = manager.getAllTours().map((t) => t.id);
    expect(ids).toEqual(expect.arrayContaining(['main-onboarding', 'fault-reporting', 'voting']));
    expect(manager.getTour('main-onboarding')?.steps.length).toBeGreaterThan(0);
    expect(manager.getTour('does-not-exist')).toBeUndefined();
  });
});

describe('TourManager tour lifecycle', () => {
  it('startTour seeds progress, sets the current tour, and persists', async () => {
    const manager = makeManager();

    const progress = await manager.startTour('main-onboarding');

    expect(progress).not.toBeNull();
    expect(progress?.currentStepIndex).toBe(0);
    expect(progress?.skipped).toBe(false);
    expect(manager.getCurrentStep()?.id).toBe('welcome');
    // State was flushed to storage.
    expect(setItem).toHaveBeenCalledWith(ONBOARDING_STATE_KEY, expect.any(String));
    expect(JSON.parse(store[ONBOARDING_STATE_KEY]).currentTour).toBe('main-onboarding');
  });

  it('startTour returns null for an unknown tour and leaves state untouched', async () => {
    const manager = makeManager();

    await expect(manager.startTour('nope')).resolves.toBeNull();
    expect(manager.getState().currentTour).toBeNull();
    expect(setItem).not.toHaveBeenCalled();
  });

  it('nextStep advances the index and records the completed step id', async () => {
    const manager = makeManager();
    await manager.startTour('main-onboarding');

    const second = await manager.nextStep();

    expect(second?.id).toBe('dashboard');
    const progress = manager.getTourProgress('main-onboarding');
    expect(progress?.currentStepIndex).toBe(1);
    expect(progress?.stepsCompleted).toContain('welcome');
  });

  it('previousStep walks back a step but never past the first', async () => {
    const manager = makeManager();
    await manager.startTour('main-onboarding');
    await manager.nextStep();

    const back = await manager.previousStep();
    expect(back?.id).toBe('welcome');

    // Already at index 0 -> no-op, returns null.
    await expect(manager.previousStep()).resolves.toBeNull();
    expect(manager.getTourProgress('main-onboarding')?.currentStepIndex).toBe(0);
  });

  it('advancing past the last step completes the tour and clears currentTour', async () => {
    const manager = makeManager();
    const tour = manager.getTour('voting');
    await manager.startTour('voting');

    // Step through every remaining step.
    for (let i = 0; i < (tour?.steps.length ?? 0); i += 1) {
      await manager.nextStep();
    }

    expect(manager.getState().currentTour).toBeNull();
    expect(manager.isTourComplete('voting')).toBe(true);
    expect(manager.getTourProgress('voting')?.completedAt).toBeTruthy();
  });

  it('completing the main onboarding tour flips hasCompletedOnboarding', async () => {
    const manager = makeManager();
    const steps = manager.getTour('main-onboarding')?.steps.length ?? 0;
    await manager.startTour('main-onboarding');
    for (let i = 0; i < steps; i += 1) {
      await manager.nextStep();
    }

    expect(manager.getState().hasCompletedOnboarding).toBe(true);
    expect(manager.shouldShowOnboarding()).toBe(false);
  });

  it('skipTour marks the tour skipped without setting the completed-onboarding flag', async () => {
    const manager = makeManager();
    await manager.startTour('main-onboarding');

    await manager.skipTour();

    const progress = manager.getTourProgress('main-onboarding');
    expect(progress?.skipped).toBe(true);
    expect(progress?.completedAt).toBeTruthy();
    expect(manager.getState().currentTour).toBeNull();
    // A skipped main tour must NOT count as completed onboarding.
    expect(manager.getState().hasCompletedOnboarding).toBe(false);
  });
});

describe('TourManager gating and help dismissal', () => {
  it('shouldShowOnboarding is true for a fresh user and false while a tour runs', async () => {
    const manager = makeManager();
    expect(manager.shouldShowOnboarding()).toBe(true);

    await manager.startTour('voting');
    expect(manager.shouldShowOnboarding()).toBe(false);
  });

  it('dismissHelp persists per-screen and is reflected by isHelpDismissed', async () => {
    const manager = makeManager();
    expect(manager.isHelpDismissed('Dashboard')).toBe(false);

    await manager.dismissHelp('Dashboard');

    expect(manager.isHelpDismissed('Dashboard')).toBe(true);
    expect(manager.isHelpDismissed('Voting')).toBe(false);
    expect(JSON.parse(store[ONBOARDING_STATE_KEY]).helpDismissed.Dashboard).toBe(true);
  });

  it('reset restores default state and clears progress', async () => {
    const manager = makeManager();
    await manager.startTour('voting');
    await manager.dismissHelp('Dashboard');

    await manager.reset();

    expect(manager.getState()).toEqual({
      hasCompletedOnboarding: false,
      tours: {},
      currentTour: null,
      helpDismissed: {},
    });
    expect(manager.shouldShowOnboarding()).toBe(true);
  });
});

describe('TourManager persistence and subscribers', () => {
  it('initialize restores a previously saved state across instances', async () => {
    const first = makeManager();
    await first.startTour('main-onboarding');
    await first.nextStep();
    await first.dismissHelp('Voting');

    // A brand new manager sharing the same backing store reloads the progress.
    const second = makeManager();
    await second.initialize();

    expect(second.getState().currentTour).toBe('main-onboarding');
    expect(second.getTourProgress('main-onboarding')?.currentStepIndex).toBe(1);
    expect(second.isHelpDismissed('Voting')).toBe(true);
  });

  it('initialize degrades to defaults on a corrupt stored blob instead of throwing', async () => {
    store[ONBOARDING_STATE_KEY] = '{ not valid json';

    const manager = makeManager();
    await expect(manager.initialize()).resolves.toBeUndefined();

    expect(manager.getState().hasCompletedOnboarding).toBe(false);
    expect(manager.getState().currentTour).toBeNull();
  });

  it('notifies subscribers on every transition and stops after unsubscribe', async () => {
    const manager = makeManager();
    const listener = jest.fn();

    const unsubscribe = manager.subscribe(listener);
    await manager.startTour('voting');
    await manager.nextStep();
    const callsWhileSubscribed = listener.mock.calls.length;
    expect(callsWhileSubscribed).toBeGreaterThanOrEqual(2);

    unsubscribe();
    await manager.nextStep();
    expect(listener).toHaveBeenCalledTimes(callsWhileSubscribed);
  });
});
