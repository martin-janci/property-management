/**
 * Characterization tests for `NFCCredentialManager`'s remaining untested
 * credential-lifecycle and expiry surface.
 *
 * Two sibling suites already cover most of the manager:
 *  - `NFCCredentialManager.test.ts` — emergency revoke, access-log corruption
 *    guard, SecureStore chunking round-trip, cross-tenant purge.
 *  - `NFCCredentialManager.credentials.test.ts` (#3826) — the query / active
 *    filter / 24 h refresh-threshold / usage-counter / log-cap / fetch-fallback
 *    / legacy-migration surface.
 *
 * This suite fills the genuinely-uncovered gaps that neither touches, all of
 * which are credential parsing / expiry / lifecycle:
 *
 *  - `refreshCredential` — the single-credential rotation path (fetch fresh
 *    encrypted data, splice it into the cache at the right index, persist; and
 *    the "unknown id" branch that returns the server copy without a local
 *    write).
 *  - server-clock-corrected expiry at the *manager* level — `now()` /
 *    `getServerTimeOffsetMs()` learned from a response `Date` header, and its
 *    effect on `getActiveCredentials` and `credentialsNeedRefresh`. The #3826
 *    suite exercises those filters only at offset 0 (its fetch double carries no
 *    `Date` header); the controller's serverClock suite drives the offset but
 *    only through `validateAccess`, never through the manager's own expiry
 *    filters.
 *  - `syncServerClock` best-effort guards — a missing / unparseable `Date`
 *    header, and a `headers` object without a usable `.get`, must leave the
 *    prior offset untouched and never throw on the API hot path.
 *  - `deserializeStoredCredentials` unrecognized-format branch — a stored scalar
 *    (neither a chunk manifest nor a legacy array) is treated as corruption and
 *    discarded rather than crashing startup.
 *
 * Every assertion pins existing behaviour; nothing in the manager changes.
 */
import AsyncStorage from '@react-native-async-storage/async-storage';
import * as SecureStore from 'expo-secure-store';

import { NFCCredentialManager } from './NFCCredentialManager';
import type { NFCCredential } from './types';

// In-memory SecureStore backed by an isolated map, matching the shape the
// sibling suites use. Seeding the legacy raw-array format under CREDENTIALS_KEY
// lets us populate the manager's private cache through the public
// `initialize()` without reaching into internals.
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
const API_BASE = 'https://api.test';

// A fixed "now" so expiry / refresh thresholds are deterministic regardless of
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

/** A fetch mock returning `body` as a 200 JSON response (no `Date` header). */
function okFetch(body: unknown): jest.Mock {
  return jest.fn(async () => ({
    ok: true,
    status: 200,
    json: async () => body,
  })) as unknown as jest.Mock;
}

/** A 200 fetch double whose response advertises `serverIso` in its `Date` header. */
function fetchWithServerDate(serverIso: string, body: unknown): jest.Mock {
  const httpDate = new Date(serverIso).toUTCString();
  return jest.fn(async () => ({
    ok: true,
    status: 200,
    headers: { get: (name: string) => (name.toLowerCase() === 'date' ? httpDate : null) },
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

describe('NFCCredentialManager.refreshCredential', () => {
  it('replaces the cached credential in place and persists the refreshed copy', async () => {
    const manager = await managerWithCredentials([
      makeCredential({ id: 'a', encryptedData: 'old-a', usageCount: 7 }),
      makeCredential({ id: 'b', encryptedData: 'old-b' }),
    ]);

    const refreshed = makeCredential({ id: 'a', encryptedData: 'new-a', usageCount: 0 });
    globalThis.fetch = fetchWithServerDate(NOW_ISO(), { credential: refreshed });

    const result = await manager.refreshCredential('a');

    // The returned credential is the server copy.
    expect(result.encryptedData).toBe('new-a');
    // The cache entry for 'a' was swapped in place; 'b' is untouched and order holds.
    expect(manager.getCredentialById('a')?.encryptedData).toBe('new-a');
    expect(manager.getCredentialById('a')?.usageCount).toBe(0);
    expect(manager.getCredentialById('b')?.encryptedData).toBe('old-b');
    // The mutated cache is persisted back to SecureStore.
    expect(SecureStore.setItemAsync as jest.Mock).toHaveBeenCalledWith(
      CREDENTIALS_KEY,
      expect.any(String)
    );
  });

  it('returns the server credential without a local write when the id is not cached', async () => {
    const manager = await managerWithCredentials([makeCredential({ id: 'a' })]);
    (SecureStore.setItemAsync as jest.Mock).mockClear();

    const refreshed = makeCredential({ id: 'ghost', encryptedData: 'fresh' });
    globalThis.fetch = okFetch({ credential: refreshed });

    const result = await manager.refreshCredential('ghost');

    expect(result.encryptedData).toBe('fresh');
    // No matching cache entry => no persist, and the unknown id is not adopted.
    expect(SecureStore.setItemAsync as jest.Mock).not.toHaveBeenCalled();
    expect(manager.getCredentialById('ghost')).toBeUndefined();
  });

  it('propagates a server error (unlike fetchCredentials, there is no cache fallback)', async () => {
    const manager = await managerWithCredentials([makeCredential({ id: 'a' })]);
    globalThis.fetch = jest.fn(async () => ({
      ok: false,
      status: 500,
      json: async () => ({}),
    })) as unknown as jest.Mock;

    await expect(manager.refreshCredential('a')).rejects.toThrow('API request failed: 500');
  });
});

describe('NFCCredentialManager — server-clock-corrected expiry (manager level)', () => {
  it('learns a positive offset from the response Date header and exposes it via now()', async () => {
    // Server clock is 2 h ahead of the (faked) device clock.
    const serverIso = new Date(NOW + 2 * HOUR).toISOString();
    globalThis.fetch = fetchWithServerDate(serverIso, { credentials: [] });

    const manager = new NFCCredentialManager(API_BASE);
    await manager.initialize();
    expect(manager.getServerTimeOffsetMs()).toBe(0); // no network yet

    await manager.fetchCredentials();

    expect(manager.getServerTimeOffsetMs()).toBe(2 * HOUR);
    expect(manager.now()).toBe(NOW + 2 * HOUR);
  });

  it('excludes a credential that is expired on the server clock though still future on the device clock', async () => {
    // Device clock reads NOW; the credential expires 1 h after NOW, so on the
    // raw device clock it still looks active.
    const manager = await managerWithCredentials([
      makeCredential({
        id: 'soon',
        status: 'active',
        validUntil: new Date(NOW + HOUR).toISOString(),
      }),
    ]);
    // Baseline: with no learned offset the credential is active.
    expect(manager.getActiveCredentials().map((c) => c.id)).toEqual(['soon']);

    // One round-trip teaches the manager the server is 3 h ahead of the device.
    globalThis.fetch = fetchWithServerDate(new Date(NOW + 3 * HOUR).toISOString(), {
      credentials: manager.getActiveCredentials(),
    });
    await manager.fetchCredentials();

    // Server-corrected now() (NOW + 3 h) is past validUntil (NOW + 1 h): expired.
    expect(manager.getServerTimeOffsetMs()).toBe(3 * HOUR);
    expect(manager.getActiveCredentials()).toHaveLength(0);
  });

  it('flags refresh-needed once the server-corrected clock crosses the 24 h threshold', async () => {
    // Credential expires 25 h out: comfortably outside the 24 h window on the
    // device clock.
    const manager = await managerWithCredentials([
      makeCredential({ status: 'active', validUntil: new Date(NOW + 25 * HOUR).toISOString() }),
    ]);
    expect(manager.credentialsNeedRefresh()).toBe(false);

    // Server is 2 h ahead: effective remaining life is 23 h < 24 h threshold.
    globalThis.fetch = fetchWithServerDate(new Date(NOW + 2 * HOUR).toISOString(), {
      credentials: manager.getActiveCredentials(),
    });
    await manager.fetchCredentials();

    expect(manager.credentialsNeedRefresh()).toBe(true);
  });
});

describe('NFCCredentialManager.syncServerClock — best-effort guards', () => {
  it('leaves the offset at 0 when the response has no Date header', async () => {
    globalThis.fetch = jest.fn(async () => ({
      ok: true,
      status: 200,
      headers: { get: () => null },
      json: async () => ({ credentials: [] }),
    })) as unknown as jest.Mock;

    const manager = new NFCCredentialManager(API_BASE);
    await manager.initialize();
    await manager.fetchCredentials();

    expect(manager.getServerTimeOffsetMs()).toBe(0);
  });

  it('leaves the offset untouched when the Date header is unparseable', async () => {
    globalThis.fetch = jest.fn(async () => ({
      ok: true,
      status: 200,
      headers: { get: (name: string) => (name.toLowerCase() === 'date' ? 'not-a-date' : null) },
      json: async () => ({ credentials: [] }),
    })) as unknown as jest.Mock;

    const manager = new NFCCredentialManager(API_BASE);
    await manager.initialize();
    await manager.fetchCredentials();

    expect(manager.getServerTimeOffsetMs()).toBe(0);
    expect(manager.now()).toBe(NOW);
  });

  it('does not throw when the response headers object has no usable get()', async () => {
    // Some fetch polyfills / test doubles return a plain object for `headers`.
    globalThis.fetch = jest.fn(async () => ({
      ok: true,
      status: 200,
      headers: {},
      json: async () => ({ credentials: [] }),
    })) as unknown as jest.Mock;

    const manager = new NFCCredentialManager(API_BASE);
    await manager.initialize();

    await expect(manager.fetchCredentials()).resolves.toBeDefined();
    expect(manager.getServerTimeOffsetMs()).toBe(0);
  });
});

describe('NFCCredentialManager — stored-credential parsing (unrecognized format)', () => {
  it('discards a stored scalar (neither manifest nor array) as corruption and starts empty', async () => {
    const warnSpy = jest.spyOn(console, 'warn').mockImplementation(() => {});
    // A bare JSON number is valid JSON but is neither a chunk manifest nor a
    // legacy credential array: deserialize must reject it as corruption.
    secureStore.set(CREDENTIALS_KEY, JSON.stringify(42));

    const manager = new NFCCredentialManager(API_BASE);
    await expect(manager.initialize()).resolves.toBeUndefined();

    expect(manager.getActiveCredentials()).toEqual([]);
    expect(warnSpy).toHaveBeenCalledWith(
      expect.stringContaining('credentials corrupted'),
      expect.anything()
    );
    // The bad blob is cleared from SecureStore.
    expect(SecureStore.deleteItemAsync as jest.Mock).toHaveBeenCalledWith(CREDENTIALS_KEY);
    warnSpy.mockRestore();
  });
});

/** Current faked system time as an ISO string (server clock == device clock). */
function NOW_ISO(): string {
  return new Date(NOW).toISOString();
}
