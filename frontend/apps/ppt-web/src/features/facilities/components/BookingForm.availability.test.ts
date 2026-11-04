/**
 * Booking slot availability (round-3 audit #21).
 *
 * The availability endpoint returns the existing bookings as
 * `is_available: false` slots. A time must be free unless it is inside one of
 * them; previously every time was disabled because the form required an
 * `is_available: true` slot that the server never sends.
 */

import type { AvailableSlot } from '@ppt/api-client';
import { describe, expect, it } from 'vitest';
import { isTimeAvailable } from './BookingForm';

const booked = (start: string, end: string): AvailableSlot => ({
  start_time: new Date(start).toISOString(),
  end_time: new Date(end).toISOString(),
  is_available: false,
});

describe('isTimeAvailable', () => {
  it('treats every time as free when nothing is booked', () => {
    expect(isTimeAvailable('2026-03-10T09:00:00', [])).toBe(true);
    expect(isTimeAvailable('2026-03-10T09:00:00', undefined)).toBe(true);
  });

  it('blocks times inside an existing booking and frees the rest', () => {
    const slots = [booked('2026-03-10T10:00:00', '2026-03-10T12:00:00')];
    expect(isTimeAvailable('2026-03-10T10:00:00', slots)).toBe(false);
    expect(isTimeAvailable('2026-03-10T11:30:00', slots)).toBe(false);
    expect(isTimeAvailable('2026-03-10T12:00:00', slots)).toBe(true);
    expect(isTimeAvailable('2026-03-10T09:30:00', slots)).toBe(true);
  });

  it('requires an explicit free slot when the response lists them', () => {
    const slots: AvailableSlot[] = [
      {
        start_time: new Date('2026-03-10T08:00:00').toISOString(),
        end_time: new Date('2026-03-10T10:00:00').toISOString(),
        is_available: true,
      },
    ];
    expect(isTimeAvailable('2026-03-10T09:00:00', slots)).toBe(true);
    expect(isTimeAvailable('2026-03-10T11:00:00', slots)).toBe(false);
  });
});
