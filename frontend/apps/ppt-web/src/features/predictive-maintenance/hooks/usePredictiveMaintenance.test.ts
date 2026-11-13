/// <reference types="vitest/globals" />
/**
 * Regression tests for the `predictiveKeys` TanStack Query key factory.
 *
 * `useNeedingMaintenance(daysAhead)` varies its request by `days_ahead`
 * (`GET /ai/equipment/needing-maintenance?days_ahead=<n>`), so the query key
 * MUST include `daysAhead`. Previously `needingMaintenance()` ignored the
 * argument, so two different look-ahead windows collided on a single cache
 * entry (staleTime 5 min) — the second window silently served the first
 * window's data and never refetched.
 */
import { describe, expect, it } from 'vitest';
import { predictiveKeys } from './usePredictiveMaintenance';

describe('predictiveKeys.needingMaintenance', () => {
  it('includes daysAhead so distinct look-ahead windows get distinct keys', () => {
    const k7 = predictiveKeys.needingMaintenance(7);
    const k30 = predictiveKeys.needingMaintenance(30);

    expect(k7).not.toEqual(k30);
    expect(k7).toContain(7);
    expect(k30).toContain(30);
  });

  it('keeps the shared root so logout-eviction by prefix still matches', () => {
    expect(predictiveKeys.needingMaintenance(30)).toEqual([
      ...predictiveKeys.all,
      'needing-maintenance',
      30,
    ]);
  });
});
