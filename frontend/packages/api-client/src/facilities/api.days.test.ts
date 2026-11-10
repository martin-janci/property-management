/**
 * Facility opening days: the form works with a Mon..Sun bitmask, the server
 * with an array of weekday numbers (0 = Sunday). The client used to send the
 * bitmask untouched, so creating or editing a facility always returned 422.
 */

import { afterEach, describe, expect, it, vi } from 'vitest';
import { createFacility, daysMaskToWire, daysWireToMask, getFacility } from './api';

function okJson(body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status: 200,
    headers: { 'Content-Type': 'application/json' },
  });
}

describe('facility available_days conversion', () => {
  afterEach(() => vi.restoreAllMocks());

  it('maps the bitmask to weekday numbers and back', () => {
    expect(daysMaskToWire(127)).toEqual([1, 2, 3, 4, 5, 6, 0]);
    expect(daysMaskToWire(1 | 16 | 64)).toEqual([1, 5, 0]);
    expect(daysMaskToWire(undefined)).toBeUndefined();
    expect(daysWireToMask([0, 1, 5])).toBe(1 | 16 | 64);
    expect(daysWireToMask([1, 2, 3, 4, 5, 6, 0])).toBe(127);
    expect(daysWireToMask(null)).toBeNull();
  });

  it('sends an array on create and returns a bitmask to the caller', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(okJson({ id: 'f1', available_days: [6, 0] }));
    const created = await createFacility('b-1', {
      name: 'Gym',
      facility_type: 'gym',
      available_days: 32 | 64,
    });
    const init = vi.mocked(fetch).mock.calls[0][1] as RequestInit;
    expect(JSON.parse(String(init.body)).available_days).toEqual([6, 0]);
    expect(created.available_days).toBe(32 | 64);
  });

  it('converts the stored array when loading a facility for editing', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(okJson({ id: 'f1', available_days: [1, 2] }));
    await expect(getFacility('b-1', 'f1')).resolves.toMatchObject({ available_days: 3 });
  });
});
