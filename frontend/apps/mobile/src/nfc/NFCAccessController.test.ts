/**
 * Unit tests for `NFCAccessController` time-restriction evaluation.
 *
 * Regression guard for `code-review-mobile-rn-nfc-timerestrict-midnight-day`:
 * a midnight-crossing access-point window (e.g. Mon 23:00–01:00) must evaluate
 * the correct calendar day on BOTH sides of midnight. A restriction's `days`
 * list names the day the window OPENS, so:
 *
 *   - the Tue 00:00–01:00 tail of a Monday window must be GRANTED, and
 *   - Mon 00:00–01:00 (which belongs to a Sunday window, if any) must be DENIED.
 *
 * The buggy implementation matched the tap's current day against `days` for
 * both halves, which denied the legitimate post-midnight tail and spuriously
 * granted the pre-midnight-of-the-same-weekday slot.
 *
 * `validateAccess` is private; it is the pure, side-effect-free core of the
 * day/time boundary logic, so we exercise it directly through a typed cast
 * rather than driving the full native-module/haptics/logging `handleTap` path.
 */
import { NFCAccessController } from './NFCAccessController';
import type { AccessAttemptResult, NFCCredential, TimeRestriction } from './types';

// Sunday = 0 ... Saturday = 6, matching Date#getDay() and TimeRestriction.days.
const MON = 1;
const TUE = 2;

/** Build an active credential with a single access point + time restriction. */
function credentialWith(restriction: TimeRestriction): NFCCredential {
  return {
    id: 'cred-1',
    buildingId: 'bld-1',
    buildingName: 'Test Building',
    userId: 'user-1',
    accessLevel: 'resident',
    accessPoints: [
      {
        id: 'ap-1',
        name: 'Main Entrance',
        type: 'main_entrance',
        timeRestrictions: [restriction],
      },
    ],
    validFrom: '2020-01-01T00:00:00.000Z',
    validUntil: '2999-01-01T00:00:00.000Z',
    status: 'active',
    usageCount: 0,
    encryptedData: 'enc',
  };
}

/** Invoke the private validateAccess core with the system clock pinned. */
function validateAt(isoLocal: string, restriction: TimeRestriction): AccessAttemptResult {
  jest.useFakeTimers();
  // Construct a local-time Date so getDay()/getHours()/getMinutes() are stable
  // regardless of the runner's timezone.
  jest.setSystemTime(new Date(isoLocal));
  try {
    const controller = new NFCAccessController('http://localhost');
    const credential = credentialWith(restriction);
    // validateAccess is private but is the unit under test.
    return (
      controller as unknown as {
        validateAccess(c: NFCCredential, apId: string): AccessAttemptResult;
      }
    ).validateAccess(credential, 'ap-1');
  } finally {
    jest.useRealTimers();
  }
}

describe('NFCAccessController time restrictions — midnight-crossing window', () => {
  // Window opens Monday 23:00 and closes 01:00 the next calendar day (Tuesday).
  const monOvernight: TimeRestriction = {
    days: [MON],
    startTime: '23:00',
    endTime: '01:00',
  };

  it('grants the Tuesday 00:30 tail of a Monday 23:00–01:00 window', () => {
    // 2024-01-02 is a Tuesday.
    const result = validateAt('2024-01-02T00:30:00', monOvernight);
    expect(result.granted).toBe(true);
  });

  it('grants the Monday 23:30 evening half of the window', () => {
    // 2024-01-01 is a Monday.
    const result = validateAt('2024-01-01T23:30:00', monOvernight);
    expect(result.granted).toBe(true);
  });

  it('denies Monday 00:30 (that early-morning slot belongs to a Sunday window)', () => {
    const result = validateAt('2024-01-01T00:30:00', monOvernight);
    expect(result.granted).toBe(false);
    expect(result.denialReason).toBe('time_restriction');
  });

  it('denies Tuesday 02:00 (after the window has closed)', () => {
    const result = validateAt('2024-01-02T02:00:00', monOvernight);
    expect(result.granted).toBe(false);
    expect(result.denialReason).toBe('time_restriction');
  });

  it('still honours a plain same-day window (Mon 09:00–17:00)', () => {
    const daytime: TimeRestriction = { days: [MON], startTime: '09:00', endTime: '17:00' };
    expect(validateAt('2024-01-01T12:00:00', daytime).granted).toBe(true); // Mon noon
    expect(validateAt('2024-01-01T18:00:00', daytime).granted).toBe(false); // Mon evening
    expect(validateAt('2024-01-02T12:00:00', daytime).granted).toBe(false); // Tue noon
  });

  it('spans consecutive start days correctly (Mon & Tue 23:00–01:00)', () => {
    const monTueOvernight: TimeRestriction = {
      days: [MON, TUE],
      startTime: '23:00',
      endTime: '01:00',
    };
    // Tue 00:30 — tail of the Monday window.
    expect(validateAt('2024-01-02T00:30:00', monTueOvernight).granted).toBe(true);
    // Wed 00:30 (2024-01-03) — tail of the Tuesday window.
    expect(validateAt('2024-01-03T00:30:00', monTueOvernight).granted).toBe(true);
    // Mon 00:30 — would be a tail of a Sunday window, which is not configured.
    expect(validateAt('2024-01-01T00:30:00', monTueOvernight).granted).toBe(false);
  });
});
