/**
 * Regression tests for the granular-trigger optimistic patch (#2369).
 *
 * `useUpdateNotificationTrigger.onMutate` used to patch only `preferences[]`,
 * leaving each category's `enabledEvents` rollup — the source of the
 * "X of Y enabled" badge (NotificationTriggersPage) — stale until the
 * settle-invalidate refetch resolved. These tests pin that the pure patch
 * helper recomputes `categories[].enabledEvents` in lockstep with the toggled
 * channels, matching the backend predicate (at least one channel on).
 */

import { describe, expect, it } from 'vitest';
import { applyTriggerOptimisticPatch } from './hooks';
import type { EventTriggerPreference, EventTriggersResponse } from './types';

function pref(
  overrides: Partial<EventTriggerPreference> &
    Pick<EventTriggerPreference, 'eventType' | 'category'>
): EventTriggerPreference {
  return {
    displayName: overrides.eventType,
    isPriority: false,
    pushEnabled: false,
    emailEnabled: false,
    inAppEnabled: false,
    ...overrides,
  };
}

function fixture(): EventTriggersResponse {
  const preferences: EventTriggerPreference[] = [
    // fault: one trigger with its last-remaining channel (push) still on.
    pref({ eventType: 'fault.assigned', category: 'fault', pushEnabled: true }),
    // fault: a fully-off trigger.
    pref({ eventType: 'fault.resolved', category: 'fault' }),
    // vote: fully on.
    pref({
      eventType: 'vote.opened',
      category: 'vote',
      pushEnabled: true,
      emailEnabled: true,
      inAppEnabled: true,
    }),
  ];
  return {
    preferences,
    categories: [
      { category: 'fault', displayName: 'fault', totalEvents: 2, enabledEvents: 1 },
      { category: 'vote', displayName: 'vote', totalEvents: 1, enabledEvents: 1 },
    ],
  };
}

describe('applyTriggerOptimisticPatch', () => {
  it('applies the channel patch to the matching preference only', () => {
    const next = applyTriggerOptimisticPatch(fixture(), 'fault.assigned', { pushEnabled: false });
    expect(next.preferences.find((p) => p.eventType === 'fault.assigned')?.pushEnabled).toBe(false);
    // Untouched trigger is unchanged.
    expect(next.preferences.find((p) => p.eventType === 'vote.opened')?.pushEnabled).toBe(true);
  });

  it('decrements enabledEvents when the last channel of a trigger is turned off', () => {
    const next = applyTriggerOptimisticPatch(fixture(), 'fault.assigned', { pushEnabled: false });
    // fault.assigned had only push on; turning it off drops the category to 0.
    expect(next.categories.find((c) => c.category === 'fault')?.enabledEvents).toBe(0);
    // Other categories are untouched.
    expect(next.categories.find((c) => c.category === 'vote')?.enabledEvents).toBe(1);
  });

  it('increments enabledEvents when the first channel of a trigger is turned on', () => {
    const next = applyTriggerOptimisticPatch(fixture(), 'fault.resolved', { emailEnabled: true });
    // fault.resolved was fully off; enabling one channel adds it to the rollup.
    expect(next.categories.find((c) => c.category === 'fault')?.enabledEvents).toBe(2);
  });

  it('leaves enabledEvents unchanged when a still-enabled trigger keeps another channel on', () => {
    const next = applyTriggerOptimisticPatch(fixture(), 'vote.opened', { pushEnabled: false });
    // vote.opened still has email + in-app on, so it stays counted.
    expect(next.categories.find((c) => c.category === 'vote')?.enabledEvents).toBe(1);
  });

  it('does not mutate the previous response object', () => {
    const previous = fixture();
    const snapshot = JSON.parse(JSON.stringify(previous));
    applyTriggerOptimisticPatch(previous, 'fault.assigned', { pushEnabled: false });
    expect(previous).toEqual(snapshot);
  });
});
