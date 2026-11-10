/**
 * Characterization tests for `NFCAccessController` — the physical door-access
 * authorization engine.
 *
 * Scope (matches the untested-code finding
 * `code-review-mobile-rn-nfc-accesscontroller-untested`): the controller's
 * access-decision logic — credential status (suspended / revoked / expired),
 * validity-window expiry, access-point authorization, and per-access-point
 * time restrictions. These are CHARACTERIZATION tests: they pin the engine's
 * CURRENT behavior so future refactors are safe; they do not assert what the
 * logic "should" do.
 *
 * The decision logic lives in the private `validateAccess()`, which is reached
 * through the public `handleTap()` surface. In the jest environment the native
 * `PPTNFC` / `PPTWallet` modules are absent (`NativeModules.PPTNFC` is
 * undefined), so `handleTap()` never mutates a granted result via
 * `transmitCredential` — it passes `validateAccess()`'s verdict through
 * unchanged. That makes `handleTap()` a faithful window onto the authorization
 * decision.
 *
 * Time-window tests use jest fake timers to pin "now" deterministically. The
 * process TZ is UTC (pinned in jest.config.js), so `Date#getDay/getHours`
 * align with their UTC counterparts; restriction days are derived from the
 * mocked instant rather than hard-coded so the suite is robust to the chosen
 * date. The exact midnight-boundary edge is deliberately NOT exercised here —
 * that quirk is owned by a separate bug task.
 */
import { NFCAccessController } from './NFCAccessController';
import type { NFCCredentialManager } from './NFCCredentialManager';
import type { AccessPoint, CredentialStatus, NFCCredential, TimeRestriction } from './types';

const API_BASE_URL = 'http://localhost:8080';

/** A single main-entrance access point, optionally time-restricted. */
function makeAccessPoint(
  id: string,
  name: string,
  timeRestrictions?: TimeRestriction[]
): AccessPoint {
  return {
    id,
    name,
    type: 'main_entrance',
    ...(timeRestrictions ? { timeRestrictions } : {}),
  };
}

/**
 * Build a credential. Defaults describe an active credential with a wide
 * validity window and a single unrestricted access point `ap-1`, so each test
 * overrides only the field it characterizes.
 */
function makeCredential(overrides: Partial<NFCCredential> = {}): NFCCredential {
  return {
    id: 'cred-1',
    buildingId: 'building-1',
    buildingName: 'Building 1',
    userId: 'user-1',
    accessLevel: 'resident',
    accessPoints: [makeAccessPoint('ap-1', 'Main Entrance')],
    validFrom: '2020-01-01T00:00:00.000Z',
    validUntil: '2999-01-01T00:00:00.000Z',
    status: 'active',
    usageCount: 0,
    encryptedData: 'ZW5jcnlwdGVk',
    ...overrides,
  };
}

/**
 * Drive `handleTap()` for a given credential at a given access point.
 *
 * Spies the controller's internal credential manager so the decision is
 * isolated from storage/network: `getCredentialById` returns `credential`
 * (pass `null` to simulate a credential that vanished from the cache), and the
 * post-decision side effects (`logAccessAttempt`, `updateCredentialUsage`) are
 * stubbed to no-op resolved promises. The active credential id is seeded
 * directly — the precondition `handleTap()` documents — to avoid standing up
 * the native-hardware session state machine.
 */
async function tap(options: {
  credential: NFCCredential | null;
  accessPointId?: string;
  accessPointName?: string;
  activeCredentialId?: string | null;
}) {
  const {
    credential,
    accessPointId = 'ap-1',
    accessPointName = 'Main Entrance',
    activeCredentialId = credential?.id ?? 'cred-1',
  } = options;

  const controller = new NFCAccessController(API_BASE_URL);
  const manager = controller.getCredentialManager();

  const getById = jest.spyOn(manager, 'getCredentialById').mockReturnValue(credential ?? undefined);
  const logSpy = jest.spyOn(manager, 'logAccessAttempt').mockResolvedValue(undefined);
  const usageSpy = jest.spyOn(manager, 'updateCredentialUsage').mockResolvedValue(undefined);

  // Seed the active credential (set by startAccessSession in production).
  (controller as unknown as { activeCredentialId: string | null }).activeCredentialId =
    activeCredentialId;

  const result = await controller.handleTap(accessPointId, accessPointName);
  return { controller, manager, result, getById, logSpy, usageSpy };
}

afterEach(() => {
  jest.restoreAllMocks();
});

describe('NFCAccessController.handleTap — precondition guards', () => {
  it('denies with invalid_credential when no credential session is active', async () => {
    const { result, getById } = await tap({ credential: null, activeCredentialId: null });

    expect(result.granted).toBe(false);
    expect(result.denialReason).toBe('invalid_credential');
    expect(result.message).toBe('No active credential');
    expect(result.accessPointId).toBe('ap-1');
    expect(result.accessPointName).toBe('Main Entrance');
    // Short-circuits before any credential lookup.
    expect(getById).not.toHaveBeenCalled();
  });

  it('denies with invalid_credential when the active credential is gone from the cache', async () => {
    const { result } = await tap({ credential: null, activeCredentialId: 'cred-1' });

    expect(result.granted).toBe(false);
    expect(result.denialReason).toBe('invalid_credential');
    expect(result.message).toBe('Credential not found');
  });
});

describe('NFCAccessController authorization — credential status', () => {
  it('grants access for an active, in-window credential at an authorized access point', async () => {
    const { result, logSpy, usageSpy } = await tap({ credential: makeCredential() });

    expect(result.granted).toBe(true);
    expect(result.denialReason).toBeUndefined();
    expect(result.accessPointName).toBe('Main Entrance');
    expect(result.message).toBe('Access granted: Main Entrance');

    // A granted tap is logged as 'granted' and bumps usage.
    expect(logSpy).toHaveBeenCalledWith('cred-1', 'ap-1', 'Main Entrance', 'granted', undefined);
    expect(usageSpy).toHaveBeenCalledWith('cred-1');
  });

  it.each<[CredentialStatus, string]>([
    ['revoked', 'credential_revoked'],
    ['suspended', 'credential_suspended'],
    ['expired', 'credential_expired'],
  ])('denies a %s credential with %s', async (status, denialReason) => {
    const { result, logSpy, usageSpy } = await tap({
      credential: makeCredential({ status }),
    });

    expect(result.granted).toBe(false);
    expect(result.denialReason).toBe(denialReason);
    expect(result.message).toBe(`Credential is ${status}`);
    // The denial resolves the access-point name from the credential.
    expect(result.accessPointName).toBe('Main Entrance');

    // A denied tap is still logged (as 'denied') but never counts as usage.
    expect(logSpy).toHaveBeenCalledWith('cred-1', 'ap-1', 'Main Entrance', 'denied', denialReason);
    expect(usageSpy).not.toHaveBeenCalled();
  });

  it('maps an unknown non-active status to invalid_credential', async () => {
    const { result } = await tap({
      credential: makeCredential({ status: 'pending_activation' }),
    });

    expect(result.granted).toBe(false);
    expect(result.denialReason).toBe('invalid_credential');
    expect(result.message).toBe('Credential is pending_activation');
  });

  it('resolves the denial access-point name to "Unknown" when the id is not on the credential', async () => {
    const { result } = await tap({
      credential: makeCredential({ status: 'revoked' }),
      accessPointId: 'ap-does-not-exist',
    });

    expect(result.granted).toBe(false);
    expect(result.denialReason).toBe('credential_revoked');
    expect(result.accessPointName).toBe('Unknown');
  });
});

describe('NFCAccessController authorization — validity window (expiry)', () => {
  it('denies an active credential whose validUntil is in the past', async () => {
    const { result } = await tap({
      credential: makeCredential({ validUntil: '2000-01-01T00:00:00.000Z' }),
    });

    expect(result.granted).toBe(false);
    expect(result.denialReason).toBe('credential_expired');
    expect(result.message).toBe('Credential has expired');
  });

  it('checks status before the validity window (a revoked-and-expired credential reads as revoked)', async () => {
    const { result } = await tap({
      credential: makeCredential({
        status: 'revoked',
        validUntil: '2000-01-01T00:00:00.000Z',
      }),
    });

    expect(result.denialReason).toBe('credential_revoked');
  });
});

describe('NFCAccessController authorization — access point', () => {
  it('denies access to an access point not listed on the credential', async () => {
    const { result } = await tap({
      credential: makeCredential({
        accessPoints: [makeAccessPoint('ap-1', 'Main Entrance')],
      }),
      accessPointId: 'ap-garage',
      accessPointName: 'Garage Gate',
    });

    expect(result.granted).toBe(false);
    expect(result.denialReason).toBe('access_point_not_allowed');
    expect(result.message).toBe('Access point not authorized');
    // The unauthorized-point denial reports the name as 'Unknown'.
    expect(result.accessPointName).toBe('Unknown');
  });

  it('authorizes the matching access point out of several on the credential', async () => {
    const { result } = await tap({
      credential: makeCredential({
        accessPoints: [
          makeAccessPoint('ap-1', 'Main Entrance'),
          makeAccessPoint('ap-2', 'Parking Gate'),
        ],
      }),
      accessPointId: 'ap-2',
      accessPointName: 'Parking Gate',
    });

    expect(result.granted).toBe(true);
    expect(result.accessPointName).toBe('Parking Gate');
    expect(result.message).toBe('Access granted: Parking Gate');
  });

  it('getAccessPointsForCredential returns the credential access points, or [] when unknown', () => {
    const controller = new NFCAccessController(API_BASE_URL);
    const manager = controller.getCredentialManager();
    const points = [makeAccessPoint('ap-1', 'Main Entrance')];
    jest
      .spyOn(manager, 'getCredentialById')
      .mockImplementation((id: string) =>
        id === 'cred-1' ? makeCredential({ accessPoints: points }) : undefined
      );

    expect(controller.getAccessPointsForCredential('cred-1')).toEqual(points);
    expect(controller.getAccessPointsForCredential('missing')).toEqual([]);
  });
});

describe('NFCAccessController authorization — time restrictions', () => {
  // A Wednesday noon in UTC. getDay() is derived from this instant inside each
  // test, so the suite does not depend on hand-computed weekday arithmetic.
  const NOON = new Date('2026-06-10T12:00:00.000Z');

  beforeEach(() => {
    jest.useFakeTimers();
    jest.setSystemTime(NOON);
  });

  afterEach(() => {
    jest.useRealTimers();
  });

  /** Day-of-week (0=Sun..6=Sat) for the currently-mocked instant. */
  function today(): number {
    return new Date().getDay();
  }

  it('grants when the current day and time fall inside a restriction window', async () => {
    const { result } = await tap({
      credential: makeCredential({
        accessPoints: [
          makeAccessPoint('ap-1', 'Main Entrance', [
            { days: [today()], startTime: '08:00', endTime: '18:00' },
          ]),
        ],
      }),
    });

    expect(result.granted).toBe(true);
    expect(result.message).toBe('Access granted: Main Entrance');
  });

  it('denies with time_restriction when the current time is before the window starts', async () => {
    const { result } = await tap({
      credential: makeCredential({
        accessPoints: [
          makeAccessPoint('ap-1', 'Main Entrance', [
            { days: [today()], startTime: '13:00', endTime: '18:00' },
          ]),
        ],
      }),
    });

    expect(result.granted).toBe(false);
    expect(result.denialReason).toBe('time_restriction');
    expect(result.message).toBe('Access not allowed at this time');
    // The time-restriction denial keeps the real access-point name.
    expect(result.accessPointName).toBe('Main Entrance');
  });

  it('denies with time_restriction when the current day is not in the allowed days', async () => {
    const otherDay = (today() + 1) % 7;
    const { result } = await tap({
      credential: makeCredential({
        accessPoints: [
          makeAccessPoint('ap-1', 'Main Entrance', [
            { days: [otherDay], startTime: '00:00', endTime: '23:59' },
          ]),
        ],
      }),
    });

    expect(result.granted).toBe(false);
    expect(result.denialReason).toBe('time_restriction');
  });

  it('grants when the current time satisfies any one of several restrictions', async () => {
    const { result } = await tap({
      credential: makeCredential({
        accessPoints: [
          makeAccessPoint('ap-1', 'Main Entrance', [
            // Does not match (wrong day), ...
            { days: [(today() + 1) % 7], startTime: '00:00', endTime: '23:59' },
            // ...but this one does.
            { days: [today()], startTime: '11:00', endTime: '13:00' },
          ]),
        ],
      }),
    });

    expect(result.granted).toBe(true);
  });

  describe('windows that cross midnight (start > end)', () => {
    it('grants late at night when now is after the start of a 22:00–06:00 window', async () => {
      jest.setSystemTime(new Date('2026-06-10T23:00:00.000Z'));
      const { result } = await tap({
        credential: makeCredential({
          accessPoints: [
            makeAccessPoint('ap-1', 'Main Entrance', [
              { days: [today()], startTime: '22:00', endTime: '06:00' },
            ]),
          ],
        }),
      });

      expect(result.granted).toBe(true);
    });

    it('denies midday inside a 22:00–06:00 window', async () => {
      // NOON (12:00) is neither >= 22:00 nor <= 06:00.
      const { result } = await tap({
        credential: makeCredential({
          accessPoints: [
            makeAccessPoint('ap-1', 'Main Entrance', [
              { days: [today()], startTime: '22:00', endTime: '06:00' },
            ]),
          ],
        }),
      });

      expect(result.granted).toBe(false);
      expect(result.denialReason).toBe('time_restriction');
    });
  });

  it('treats an empty restrictions array as unrestricted (grants)', async () => {
    const { result } = await tap({
      credential: makeCredential({
        accessPoints: [makeAccessPoint('ap-1', 'Main Entrance', [])],
      }),
    });

    expect(result.granted).toBe(true);
  });
});

describe('NFCAccessController — hardware state surface (no native module)', () => {
  it('reports an unsupported state and human-readable description before initialize', () => {
    const controller = new NFCAccessController(API_BASE_URL);

    expect(controller.getState()).toBe('unsupported');
    expect(controller.isSupported()).toBe(false);
    expect(controller.isReady()).toBe(false);
    expect(controller.getStateDescription()).toBe('NFC is not supported on this device');
  });

  it('exposes the credential manager passed the configured base url', () => {
    const controller = new NFCAccessController(API_BASE_URL);
    const manager: NFCCredentialManager = controller.getCredentialManager();
    expect(manager).toBeDefined();
    expect(typeof manager.getActiveCredentials).toBe('function');
  });
});
