/**
 * Unit tests for `NFCCredentialManager`'s credential-handling and expiry logic.
 *
 * The existing `NFCCredentialManager.test.ts` covers the emergency revoke,
 * access-log corruption guard, SecureStore chunking, and the cross-tenant
 * purge. This companion suite covers the previously untested query / expiry /
 * refresh / usage surface that drives every door tap:
 *
 *  - `getActiveCredentials` — the live-credential filter (status === 'active'
 *    AND validUntil strictly in the future) that `startAccessSession` uses to
 *    auto-select a credential.
 *  - `getCredentialForBuilding` / `getCredentialById` — the lookups.
 *  - `credentialsNeedRefresh` — the 24 h expiry threshold that decides whether
 *    a background refresh is due.
 *  - `updateCredentialUsage` — the per-tap usage counter + lastUsed stamp.
 *  - `logAccessAttempt` — building-id resolution, the 100-entry log cap, and
 *    best-effort server sync that must not throw when offline.
 *  - `getRecentAccessAttempts` — the bounded recent-attempts slice.
 *  - `fetchCredentials` — persist-on-success, fall back to cache on failure.
 *  - the one-time legacy AsyncStorage -> SecureStore credential migration.
 *
 * These assert existing behaviour only; nothing in the manager changes.
 */
import AsyncStorage from '@react-native-async-storage/async-storage';
import * as SecureStore from 'expo-secure-store';

import { NFCCredentialManager } from './NFCCredentialManager';
import type { AccessLogEntry, NFCCredential } from './types';

// Same in-memory SecureStore shape the sibling suite uses: an isolated map
// backing getItemAsync/setItemAsync/deleteItemAsync. Seeding the raw-array
// legacy single-slot format under CREDENTIALS_KEY lets us populate the
// manager's private credential cache through its public `initialize()` without
// reaching into internals.
jest.mock('expo-secure-store', () => {
  const store = new Map<string, string>();
  return {
    __store: store,
    getItemAsync: jest.fn(async (k: string) => store.get(k) ?? null),
    setItemAsync: jest.fn(async (k: string, v: string) => {
      store.set(k, v);
    }),
    deleteItemAsync: jest.fn(async (k: string) => {
      store.delete(k);
    }),
  };
});

const secureStore = (SecureStore as unknown as { __store: Map<string, string> }).__store;

const asyncGet = AsyncStorage.getItem as jest.Mock;
const asyncSet = AsyncStorage.setItem as jest.Mock;
const asyncRemove = AsyncStorage.removeItem as jest.Mock;

const CREDENTIALS_KEY = 'ppt_nfc_credentials';
const LEGACY_CREDENTIALS_KEY = '@ppt/nfc_credentials';
const ACCESS_LOG_KEY = '@ppt/access_log';
const API_BASE = 'https://api.test';

// A fixed "now" so expiry/refresh thresholds are deterministic regardless of
// the wall clock on the runner.
const NOW = new Date('2026-06-15T12:00:00.000Z').getTime();
const HOUR = 60 * 60 * 1000;
const DAY = 24 * HOUR;

function makeCredential(overrides: Partial<NFCCredential> = {}): NFCCredential {
  return {
    id: 'cred-1',
    buildingId: 'bld-1',
    buildingName: 'Building One',
    userId: 'user-1',
    accessLevel: 'resident',
    accessPoints: [{ id: 'ap-1', name: 'Main Entrance', type: 'main_entrance' }],
    validFrom: new Date(NOW - 30 * DAY).toISOString(),
    validUntil: new Date(NOW + 30 * DAY).toISOString(),
    status: 'active',
    usageCount: 0,
    encryptedData: 'payload',
    ...overrides,
  };
}

/** Seed SecureStore with the legacy single-slot array and load it. */
async function managerWithCredentials(creds: NFCCredential[]): Promise<NFCCredentialManager> {
  secureStore.set(CREDENTIALS_KEY, JSON.stringify(creds));
  const manager = new NFCCredentialManager(API_BASE);
  await manager.initialize();
  return manager;
}

/** A fetch mock that returns `body` as a 200 JSON response. */
function okFetch(body: unknown): jest.Mock {
  return jest.fn(async () => ({
    ok: true,
    status: 200,
    json: async () => body,
  })) as unknown as jest.Mock;
}

let originalFetch: typeof globalThis.fetch;

beforeEach(() => {
  jest.clearAllMocks();
  jest.useFakeTimers();
  jest.setSystemTime(NOW);
  secureStore.clear();
  asyncGet.mockResolvedValue(null);
  asyncSet.mockResolvedValue(undefined);
  asyncRemove.mockResolvedValue(undefined);
  originalFetch = globalThis.fetch;
});

afterEach(() => {
  jest.useRealTimers();
  globalThis.fetch = originalFetch;
});

describe('NFCCredentialManager — active-credential filter (expiry)', () => {
  it('returns only active, not-yet-expired credentials', async () => {
    const manager = await managerWithCredentials([
      makeCredential({
        id: 'live',
        status: 'active',
        validUntil: new Date(NOW + DAY).toISOString(),
      }),
      makeCredential({ id: 'suspended', status: 'suspended' }),
      makeCredential({ id: 'revoked', status: 'revoked' }),
      // Active status but the window has already closed — must be excluded.
      makeCredential({
        id: 'expired-window',
        status: 'active',
        validUntil: new Date(NOW - 1).toISOString(),
      }),
    ]);

    const ids = manager.getActiveCredentials().map((c) => c.id);
    expect(ids).toEqual(['live']);
  });

  it('treats a credential expiring exactly now as no longer active (strict >)', async () => {
    const manager = await managerWithCredentials([
      makeCredential({ id: 'edge', status: 'active', validUntil: new Date(NOW).toISOString() }),
    ]);
    expect(manager.getActiveCredentials()).toHaveLength(0);
  });
});

describe('NFCCredentialManager — credential lookups', () => {
  it('getCredentialForBuilding returns the active credential matching the building', async () => {
    const manager = await managerWithCredentials([
      makeCredential({ id: 'a', buildingId: 'bld-1', status: 'active' }),
      makeCredential({ id: 'b', buildingId: 'bld-2', status: 'active' }),
    ]);
    expect(manager.getCredentialForBuilding('bld-2')?.id).toBe('b');
  });

  it('getCredentialForBuilding ignores a non-active credential for that building', async () => {
    const manager = await managerWithCredentials([
      makeCredential({ id: 'a', buildingId: 'bld-1', status: 'revoked' }),
    ]);
    expect(manager.getCredentialForBuilding('bld-1')).toBeUndefined();
  });

  it('getCredentialById finds by id and returns undefined for a miss', async () => {
    const manager = await managerWithCredentials([makeCredential({ id: 'target' })]);
    expect(manager.getCredentialById('target')?.id).toBe('target');
    expect(manager.getCredentialById('nope')).toBeUndefined();
  });
});

describe('NFCCredentialManager.credentialsNeedRefresh — 24 h threshold', () => {
  it('is true when an active credential expires within 24 h', async () => {
    const manager = await managerWithCredentials([
      makeCredential({ status: 'active', validUntil: new Date(NOW + 23 * HOUR).toISOString() }),
    ]);
    expect(manager.credentialsNeedRefresh()).toBe(true);
  });

  it('is false when every active credential is comfortably in the future', async () => {
    const manager = await managerWithCredentials([
      makeCredential({ status: 'active', validUntil: new Date(NOW + 10 * DAY).toISOString() }),
    ]);
    expect(manager.credentialsNeedRefresh()).toBe(false);
  });

  it('ignores non-active credentials even if they are about to expire', async () => {
    const manager = await managerWithCredentials([
      makeCredential({ status: 'revoked', validUntil: new Date(NOW + HOUR).toISOString() }),
    ]);
    expect(manager.credentialsNeedRefresh()).toBe(false);
  });
});

describe('NFCCredentialManager.updateCredentialUsage', () => {
  it('increments the usage count and stamps lastUsed for a known credential', async () => {
    const manager = await managerWithCredentials([makeCredential({ id: 'u', usageCount: 2 })]);

    await manager.updateCredentialUsage('u');

    const updated = manager.getCredentialById('u');
    expect(updated?.usageCount).toBe(3);
    expect(updated?.lastUsed).toBe(new Date(NOW).toISOString());
    // The mutated cache is persisted back to SecureStore.
    expect(SecureStore.setItemAsync as jest.Mock).toHaveBeenCalled();
  });

  it('is a no-op for an unknown credential id (no write)', async () => {
    const manager = await managerWithCredentials([makeCredential({ id: 'u', usageCount: 2 })]);
    (SecureStore.setItemAsync as jest.Mock).mockClear();

    await manager.updateCredentialUsage('missing');

    expect(manager.getCredentialById('u')?.usageCount).toBe(2);
    expect(SecureStore.setItemAsync as jest.Mock).not.toHaveBeenCalled();
  });
});

describe('NFCCredentialManager.logAccessAttempt', () => {
  it('resolves the building id from the credential and prepends the entry', async () => {
    globalThis.fetch = okFetch({});
    const manager = await managerWithCredentials([
      makeCredential({ id: 'c', buildingId: 'bld-9' }),
    ]);

    await manager.logAccessAttempt('c', 'ap-1', 'Main Entrance', 'granted');

    const written = JSON.parse(asyncSet.mock.calls.at(-1)?.[1] as string) as AccessLogEntry[];
    expect(written[0]).toMatchObject({
      credentialId: 'c',
      buildingId: 'bld-9',
      accessPointId: 'ap-1',
      result: 'granted',
    });
  });

  it('records a denial reason and leaves building id empty for an unknown credential', async () => {
    globalThis.fetch = okFetch({});
    const manager = await managerWithCredentials([]);

    await manager.logAccessAttempt('ghost', 'ap-1', 'Side Door', 'denied', 'credential_expired');

    const written = JSON.parse(asyncSet.mock.calls.at(-1)?.[1] as string) as AccessLogEntry[];
    expect(written[0]).toMatchObject({
      result: 'denied',
      denialReason: 'credential_expired',
      buildingId: '',
    });
  });

  it('caps the stored log at 100 entries, dropping the oldest', async () => {
    globalThis.fetch = okFetch({});
    const manager = await managerWithCredentials([
      makeCredential({ id: 'c', buildingId: 'bld-1' }),
    ]);

    // Pre-seed a full log; the newest sits at index 0.
    const existing: AccessLogEntry[] = Array.from({ length: 100 }, (_, i) => ({
      id: `old-${i}`,
      credentialId: 'c',
      buildingId: 'bld-1',
      accessPointId: 'ap-1',
      accessPointName: 'Main Entrance',
      result: 'granted',
      timestamp: new Date(NOW - (i + 1) * 1000).toISOString(),
    }));
    asyncGet.mockResolvedValueOnce(JSON.stringify(existing));

    await manager.logAccessAttempt('c', 'ap-1', 'Main Entrance', 'granted');

    const written = JSON.parse(asyncSet.mock.calls.at(-1)?.[1] as string) as AccessLogEntry[];
    expect(written).toHaveLength(100);
    // The brand-new entry is first; the formerly-oldest (`old-99`) fell off.
    expect(written[0].id).not.toMatch(/^old-/);
    expect(written.some((e) => e.id === 'old-99')).toBe(false);
  });

  it('does not throw when the best-effort server sync fails', async () => {
    globalThis.fetch = jest.fn(async () => {
      throw new Error('offline');
    }) as unknown as jest.Mock;
    const manager = await managerWithCredentials([makeCredential({ id: 'c' })]);

    await expect(
      manager.logAccessAttempt('c', 'ap-1', 'Main Entrance', 'granted')
    ).resolves.toBeUndefined();
    // The local write still happened even though the sync rejected.
    expect(asyncSet).toHaveBeenCalledWith(ACCESS_LOG_KEY, expect.any(String));
  });
});

describe('NFCCredentialManager.getRecentAccessAttempts', () => {
  it('returns at most the requested number of most-recent entries', async () => {
    const manager = await managerWithCredentials([]);
    const log: AccessLogEntry[] = Array.from({ length: 5 }, (_, i) => ({
      id: `e-${i}`,
      credentialId: 'c',
      buildingId: 'bld-1',
      accessPointId: 'ap-1',
      accessPointName: 'Main Entrance',
      result: 'granted',
      timestamp: new Date(NOW - i * 1000).toISOString(),
    }));
    asyncGet.mockResolvedValueOnce(JSON.stringify(log));

    const recent = await manager.getRecentAccessAttempts(2);
    expect(recent.map((e) => e.id)).toEqual(['e-0', 'e-1']);
  });

  it('defaults to the first 10 entries', async () => {
    const manager = await managerWithCredentials([]);
    const log: AccessLogEntry[] = Array.from({ length: 15 }, (_, i) => ({
      id: `e-${i}`,
      credentialId: 'c',
      buildingId: 'bld-1',
      accessPointId: 'ap-1',
      accessPointName: 'Main Entrance',
      result: 'granted',
      timestamp: new Date(NOW - i * 1000).toISOString(),
    }));
    asyncGet.mockResolvedValueOnce(JSON.stringify(log));

    expect(await manager.getRecentAccessAttempts()).toHaveLength(10);
  });
});

describe('NFCCredentialManager.fetchCredentials', () => {
  it('stores and returns the server credentials on success', async () => {
    const fresh = [makeCredential({ id: 'server-1' })];
    globalThis.fetch = okFetch({ credentials: fresh });
    const manager = new NFCCredentialManager(API_BASE);
    await manager.initialize();

    const result = await manager.fetchCredentials();

    expect(result.map((c) => c.id)).toEqual(['server-1']);
    // Persisted to SecureStore for offline use.
    expect(SecureStore.setItemAsync as jest.Mock).toHaveBeenCalledWith(
      CREDENTIALS_KEY,
      expect.any(String)
    );
  });

  it('falls back to the cached credentials when the fetch fails', async () => {
    const manager = await managerWithCredentials([makeCredential({ id: 'cached' })]);
    globalThis.fetch = jest.fn(async () => {
      throw new Error('network down');
    }) as unknown as jest.Mock;

    const result = await manager.fetchCredentials();
    expect(result.map((c) => c.id)).toEqual(['cached']);
  });
});

describe('NFCCredentialManager — legacy AsyncStorage migration', () => {
  it('adopts legacy AsyncStorage credentials, re-persists them to SecureStore, and clears the legacy key', async () => {
    // No SecureStore blob, but a legacy array lives in AsyncStorage.
    const legacy = [makeCredential({ id: 'legacy-1' })];
    asyncGet.mockImplementation(async (key: string) =>
      key === LEGACY_CREDENTIALS_KEY ? JSON.stringify(legacy) : null
    );

    const manager = new NFCCredentialManager(API_BASE);
    await manager.initialize();

    expect(manager.getCredentialById('legacy-1')?.id).toBe('legacy-1');
    // Migrated into the encrypted store and the legacy key removed.
    expect(SecureStore.setItemAsync as jest.Mock).toHaveBeenCalledWith(
      CREDENTIALS_KEY,
      expect.any(String)
    );
    expect(asyncRemove).toHaveBeenCalledWith(LEGACY_CREDENTIALS_KEY);
  });

  it('discards a corrupted legacy blob without throwing', async () => {
    asyncGet.mockImplementation(async (key: string) =>
      key === LEGACY_CREDENTIALS_KEY ? '{ not json' : null
    );

    const manager = new NFCCredentialManager(API_BASE);
    await expect(manager.initialize()).resolves.toBeUndefined();
    expect(manager.getActiveCredentials()).toHaveLength(0);
    expect(asyncRemove).toHaveBeenCalledWith(LEGACY_CREDENTIALS_KEY);
  });
});
