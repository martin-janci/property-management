/**
 * Wiring regression test for OnboardingToursPage (issue #2530, tests finding).
 *
 * PR #2515 unit-tested the `features/onboarding/analytics.ts` helpers but left
 * the actual `onSuccess` emitters in this page untested — a dropped or
 * misplaced `trackTour*` call (incl. the auto-start on open and the
 * last-step → complete chain) would not be caught. These tests mount the page
 * with mocked mutations that resolve synchronously and assert each canonical
 * event fires on the corresponding mutation success.
 */
/// <reference types="vitest/globals" />
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ONBOARDING_EVENTS } from '../features/onboarding/analytics';
import {
  __clearAnalyticsSinks,
  type AnalyticsEvent,
  registerAnalyticsSink,
} from '../lib/analytics';

// ── Mocks ────────────────────────────────────────────────────────────────────

// A mutation hook whose `.mutate(arg, opts)` resolves synchronously by invoking
// the caller's `onSuccess` — this is precisely the boundary the page wires its
// `trackTour*` emitters onto.
function okMutation() {
  return {
    mutate: (_arg: unknown, opts?: { onSuccess?: () => void }) => opts?.onSuccess?.(),
    isPending: false,
  };
}

const TOUR = {
  id: 'row-1',
  tour_id: 't1',
  name: 'Welcome',
  description: 'Getting started',
  steps: [
    { id: 's1', title: 'Step 1', content: 'a' },
    { id: 's2', title: 'Step 2', content: 'b' },
  ],
  target_roles: [],
  is_active: true,
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z',
};

vi.mock('@ppt/api-client', () => ({
  useOnboardingStatus: () => ({
    data: { needs_onboarding: true, tours: [{ tour: TOUR, progress: null }] },
    isLoading: false,
    isError: false,
    error: null,
    refetch: vi.fn(),
  }),
  useStartOnboardingTour: okMutation,
  useCompleteOnboardingStep: okMutation,
  useCompleteOnboardingTour: okMutation,
  useSkipOnboardingTour: okMutation,
  useResetOnboardingTour: okMutation,
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (k: string) => k, i18n: { language: 'en' } }),
}));

vi.mock('../components/Toast', () => ({
  useToast: () => ({ showToast: vi.fn() }),
}));

// The overlay is replaced with a thin harness exposing the page's handler
// props as buttons, so we can drive each funnel transition deterministically.
vi.mock('../features/onboarding/TourOverlay', () => ({
  TourOverlay: (props: {
    onCompleteStep: (stepId: string, isLast: boolean) => void;
    onSkipTour: () => void;
    onResetTour: () => void;
  }) => (
    <div>
      <button type="button" onClick={() => props.onCompleteStep('s1', false)}>
        complete-step
      </button>
      <button type="button" onClick={() => props.onCompleteStep('s2', true)}>
        complete-last
      </button>
      <button type="button" onClick={props.onSkipTour}>
        skip
      </button>
      <button type="button" onClick={props.onResetTour}>
        reset
      </button>
    </div>
  ),
}));

// Imported after the mocks are registered.
import OnboardingToursPage from './OnboardingToursPage';

// ── Helpers ──────────────────────────────────────────────────────────────────

let captured: AnalyticsEvent[];

function events() {
  return captured.map((e) => e.event);
}

/** Renders the page and opens the (auto-starting) tour overlay. */
async function renderAndOpen() {
  const user = userEvent.setup();
  render(<OnboardingToursPage />);
  // The single not-started tour card exposes a start button (auto-start on open).
  await user.click(screen.getByRole('button', { name: 'onboarding.startTour' }));
  return user;
}

beforeEach(() => {
  captured = [];
  registerAnalyticsSink((e) => captured.push(e));
});

afterEach(() => {
  __clearAnalyticsSinks();
  vi.clearAllMocks();
});

// ── Tests ────────────────────────────────────────────────────────────────────

describe('OnboardingToursPage analytics wiring', () => {
  it('emits tour_started when a tour is opened and auto-started', async () => {
    await renderAndOpen();
    expect(events()).toContain(ONBOARDING_EVENTS.tourStarted);
    const started = captured.find((e) => e.event === ONBOARDING_EVENTS.tourStarted);
    expect(started?.properties).toMatchObject({ tour_id: 't1', tour_name: 'Welcome' });
  });

  it('emits step_completed on a non-final step without completing the tour', async () => {
    const user = await renderAndOpen();
    captured = []; // drop the auto-start event
    await user.click(screen.getByRole('button', { name: 'complete-step' }));
    expect(events()).toEqual([ONBOARDING_EVENTS.stepCompleted]);
    expect(captured[0].properties).toMatchObject({
      tour_id: 't1',
      step_id: 's1',
      is_last_step: false,
    });
  });

  it('chains step_completed → tour_completed on the final step', async () => {
    const user = await renderAndOpen();
    captured = [];
    await user.click(screen.getByRole('button', { name: 'complete-last' }));
    // Order matters: the completion event must follow the final step event.
    expect(events()).toEqual([ONBOARDING_EVENTS.stepCompleted, ONBOARDING_EVENTS.tourCompleted]);
    expect(captured[0].properties).toMatchObject({ step_id: 's2', is_last_step: true });
    expect(captured[1].properties).toMatchObject({ tour_id: 't1', tour_name: 'Welcome' });
  });

  it('emits tour_skipped on skip', async () => {
    const user = await renderAndOpen();
    captured = [];
    await user.click(screen.getByRole('button', { name: 'skip' }));
    expect(events()).toEqual([ONBOARDING_EVENTS.tourSkipped]);
    expect(captured[0].properties).toMatchObject({ tour_id: 't1' });
  });

  it('emits tour_reset on reset', async () => {
    const user = await renderAndOpen();
    captured = [];
    await user.click(screen.getByRole('button', { name: 'reset' }));
    expect(events()).toEqual([ONBOARDING_EVENTS.tourReset]);
    expect(captured[0].properties).toMatchObject({ tour_id: 't1' });
  });
});
