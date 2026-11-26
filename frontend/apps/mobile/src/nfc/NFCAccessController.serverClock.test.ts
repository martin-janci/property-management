/**
 * Regression tests for `code-review-mobile-rn-nfc-device-clock-expiry`.
 *
 * The on-device access pre-check (`NFCAccessController.validateAccess`) judges a
 * credential's `validUntil` expiry. The device wall clock is attacker-
 * controlled: a resident holding an EXPIRED building-access credential can wind
 * their phone's clock BACK to before `validUntil` so the credential's validity
 * window still reads as open, and the client would transmit it. The physical
 * reader re-validates server-side, but the on-device gate must not grant on a
 * spoofed clock either — defence in depth.
 *
 * The hardening: `NFCCredentialManager` learns the server/device clock offset
 * from the `Date` header of every API response and exposes `now()` (device
 * clock + offset). `validateAccess` judges expiry against `now()` instead of the
 * raw device clock, so once the app has talked to the server at all, a wound-
 * back device clock no longer extends an expired credential.
 *
 * These drive the REAL sync path (`fetchCredentials` reading a response `Date`
 * header), then reach the private `validateAccess` core through a typed cast —
 * the same technique the sibling time-restriction suite uses.
 */
import { NFCAccessController } from './NFCAccessController';
import type { AccessAttemptResult, NFCCredential } from './types';

/** An active credential that expired at the given instant. */
function credentialExpiringAt(validUntil: string): NFCCredential {
  return {
    id: 'cred-1',
    buildingId: 'bld-1',
    buildingName: 'Test Building',
    userId: 'user-1',
    accessLevel: 'resident',
    accessPoints: [{ id: 'ap-1', name: 'Main Entrance', type: 'main_entrance' }],
    validFrom: '2020-01-01T00:00:00.000Z',
    validUntil,
    status: 'active',
    usageCount: 0,
    encryptedData: 'enc',
  };
}

/** Reach the private, side-effect-free decision core. */
function validateAccess(
  controller: NFCAccessController,
  credential: NFCCredential
): AccessAttemptResult {
  return (
    controller as unknown as {
      validateAccess(c: NFCCredential, apId: string): AccessAttemptResult;
    }
  ).validateAccess(credential, 'ap-1');
}

/** A fetch double whose response advertises `serverIso` in its `Date` header. */
function fetchWithServerDate(serverIso: string): jest.Mock {
  const httpDate = new Date(serverIso).toUTCString();
  return jest.fn(async () => ({
    ok: true,
    status: 200,
    headers: { get: (name: string) => (name.toLowerCase() === 'date' ? httpDate : null) },
    json: async () => ({ credentials: [] }),
  }));
}

describe('NFCAccessController expiry — server-authoritative clock', () => {
  const VALID_UNTIL = '2024-01-01T00:00:00.000Z';
  // Device clock wound BACK to well before expiry — the spoof.
  const DEVICE_SPOOFED = Date.parse('2023-06-01T00:00:00.000Z');
  // The true (server) time, comfortably AFTER expiry.
  const SERVER_REAL = '2024-06-01T00:00:00.000Z';

  let dateNow: jest.SpyInstance;
  let originalFetch: typeof globalThis.fetch;

  beforeEach(() => {
    originalFetch = globalThis.fetch;
    // Pin the DEVICE clock to the spoofed (wound-back) instant. `Date.now()` is
    // what both the credential manager's offset calc and `now()` read.
    dateNow = jest.spyOn(Date, 'now').mockReturnValue(DEVICE_SPOOFED);
  });

  afterEach(() => {
    dateNow.mockRestore();
    globalThis.fetch = originalFetch;
    jest.restoreAllMocks();
  });

  it('denies an expired credential even when the device clock is wound back before expiry, once the server clock is known', async () => {
    globalThis.fetch = fetchWithServerDate(SERVER_REAL) as unknown as typeof fetch;

    const controller = new NFCAccessController('http://localhost:8080');
    const manager = controller.getCredentialManager();

    // One server round-trip teaches the manager the real time (~+1 year ahead
    // of the spoofed device clock).
    await manager.fetchCredentials();
    expect(manager.getServerTimeOffsetMs()).toBeGreaterThan(0);

    const result = validateAccess(controller, credentialExpiringAt(VALID_UNTIL));

    // Raw device clock (2023-06) would say "still valid"; the server-corrected
    // clock (2024-06) correctly says expired.
    expect(result.granted).toBe(false);
    expect(result.denialReason).toBe('credential_expired');
    expect(result.message).toBe('Credential has expired');
  });

  it('grants a genuinely-unexpired credential under the same server-corrected clock', async () => {
    globalThis.fetch = fetchWithServerDate(SERVER_REAL) as unknown as typeof fetch;

    const controller = new NFCAccessController('http://localhost:8080');
    const manager = controller.getCredentialManager();
    await manager.fetchCredentials();

    // validUntil is after the real server time (2024-06), so it must be granted.
    const result = validateAccess(controller, credentialExpiringAt('2025-01-01T00:00:00.000Z'));
    expect(result.granted).toBe(true);
  });

  it('falls back to the device clock before any server sync (offset 0)', () => {
    // No fetch yet: offset is 0, so the gate uses the raw device clock. This is
    // the documented fail-safe — the server reader is still the final authority
    // at the door. The spoofed 2023 clock reads the 2024 credential as valid.
    const controller = new NFCAccessController('http://localhost:8080');
    expect(controller.getCredentialManager().getServerTimeOffsetMs()).toBe(0);

    const result = validateAccess(controller, credentialExpiringAt(VALID_UNTIL));
    expect(result.granted).toBe(true);
  });
});
