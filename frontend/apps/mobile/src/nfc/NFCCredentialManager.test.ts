/**
 * Unit tests for NFCCredentialManager access-log parsing.
 *
 * Regression guard: getAccessLog() runs an unguarded JSON.parse on the stored
 * blob. Because logAccessAttempt() reads the log AFTER access has already been
 * granted at the door, a corrupted blob would throw a SyntaxError and reject
 * the tap flow post-grant. The parse must be guarded: a corrupted blob is
 * treated as an empty/unavailable log rather than throwing.
 */
import AsyncStorage from '@react-native-async-storage/async-storage';
import * as SecureStore from 'expo-secure-store';

import { NFCCredentialManager } from './NFCCredentialManager';
import type { NFCCredential } from './types';

const getItem = AsyncStorage.getItem as jest.Mock;
const setItem = AsyncStorage.setItem as jest.Mock;

const secureGet = SecureStore.getItemAsync as jest.Mock;
const secureSet = SecureStore.setItemAsync as jest.Mock;
const secureDelete = SecureStore.deleteItemAsync as jest.Mock;

const ACCESS_LOG_KEY = '@ppt/access_log';
const CREDENTIALS_KEY = 'ppt_nfc_credentials';
// SecureStore's Android per-value cap. Every persisted slot must stay under it.
const SECURE_STORE_MAX_BYTES = 2048;

/** UTF-8 byte length of a string, without depending on Node's Buffer type. */
function utf8ByteLength(str: string): number {
  // encodeURIComponent percent-escapes each non-ASCII byte, so the decoded
  // length equals the UTF-8 byte count.
  return unescape(encodeURIComponent(str)).length;
}

/** Back the mocked SecureStore with a real in-memory map for round-trip tests. */
function installStatefulSecureStore(): Map<string, string> {
  const store = new Map<string, string>();
  secureGet.mockImplementation(async (key: string) => store.get(key) ?? null);
  secureSet.mockImplementation(async (key: string, value: string) => {
    store.set(key, value);
  });
  secureDelete.mockImplementation(async (key: string) => {
    store.delete(key);
  });
  return store;
}

/** Build a credential whose serialized size is dominated by `encryptedData`. */
function makeCredential(id: string, payloadBytes: number): NFCCredential {
  return {
    id,
    buildingId: `building-${id}`,
    buildingName: `Building ${id}`,
    userId: 'user-1',
    accessLevel: 'resident',
    accessPoints: [{ id: `ap-${id}`, name: 'Main Entrance', type: 'main_entrance' }],
    validFrom: '2026-01-01T00:00:00.000Z',
    validUntil: '2027-01-01T00:00:00.000Z',
    status: 'active',
    usageCount: 0,
    // 'x'.repeat stands in for the base64 encrypted credential payload.
    encryptedData: 'x'.repeat(payloadBytes),
  };
}

describe('NFCCredentialManager.getAccessLog', () => {
  let manager: NFCCredentialManager;
  let warnSpy: jest.SpyInstance;

  beforeEach(() => {
    jest.clearAllMocks();
    manager = new NFCCredentialManager('http://localhost:8080');
    warnSpy = jest.spyOn(console, 'warn').mockImplementation(() => {});
  });

  afterEach(() => {
    warnSpy.mockRestore();
  });

  it('returns an empty array when nothing is stored', async () => {
    getItem.mockResolvedValueOnce(null);
    await expect(manager.getAccessLog()).resolves.toEqual([]);
  });

  it('parses a valid stored log', async () => {
    const entries = [{ id: 'log-1', credentialId: 'c1', accessPointId: 'ap1', result: 'granted' }];
    getItem.mockResolvedValueOnce(JSON.stringify(entries));
    await expect(manager.getAccessLog()).resolves.toEqual(entries);
  });

  it('treats a corrupted blob as an empty log instead of throwing', async () => {
    getItem.mockResolvedValueOnce('{not valid json');
    // Must resolve (not reject) so a corrupted blob cannot break callers.
    await expect(manager.getAccessLog()).resolves.toEqual([]);
    expect(warnSpy).toHaveBeenCalledWith(
      expect.stringContaining('access log corrupted'),
      expect.anything()
    );
  });

  it('does not reject logAccessAttempt when the stored log is corrupted (post-grant safety)', async () => {
    // A corrupted blob is present when we go to record a just-granted access.
    getItem.mockResolvedValue('}}garbage[[');

    await expect(
      manager.logAccessAttempt('cred-1', 'ap-1', 'Front Door', 'granted')
    ).resolves.toBeUndefined();

    // The attempt is still persisted — the corrupted prior log is discarded and
    // replaced by a fresh single-entry array.
    expect(setItem).toHaveBeenCalledWith(ACCESS_LOG_KEY, expect.any(String));
    const [, written] = setItem.mock.calls[setItem.mock.calls.length - 1];
    const parsed = JSON.parse(written as string);
    expect(Array.isArray(parsed)).toBe(true);
    expect(parsed).toHaveLength(1);
    expect(parsed[0]).toMatchObject({
      accessPointId: 'ap-1',
      accessPointName: 'Front Door',
      result: 'granted',
    });
  });
});

/**
 * Regression guard: credentials must be chunked across SecureStore slots so a
 * large fetched set is never silently dropped by SecureStore's ~2 KB per-value
 * Android cap. The pre-fix implementation stringified the whole array into one
 * slot; a set larger than 2 KB would exceed the cap and fail to persist.
 */
describe('NFCCredentialManager credential storage (SecureStore chunking)', () => {
  let warnSpy: jest.SpyInstance;

  beforeEach(() => {
    jest.clearAllMocks();
    installStatefulSecureStore();
    // No legacy AsyncStorage credentials in these suites.
    getItem.mockResolvedValue(null);
    warnSpy = jest.spyOn(console, 'warn').mockImplementation(() => {});
  });

  afterEach(() => {
    warnSpy.mockRestore();
  });

  /** Drive fetchCredentials() to persist a given credential set. */
  async function persist(manager: NFCCredentialManager, creds: NFCCredential[]): Promise<void> {
    const fetchSpy = jest
      .spyOn(
        manager as unknown as { apiRequest: (...a: unknown[]) => Promise<unknown> },
        'apiRequest'
      )
      .mockResolvedValueOnce({ credentials: creds });
    await manager.fetchCredentials();
    fetchSpy.mockRestore();
  }

  it('keeps every SecureStore value under the 2 KB cap for a large credential set', async () => {
    const manager = new NFCCredentialManager('http://localhost:8080');
    // Five ~1 KB credentials => ~5 KB serialized, well past the single-slot cap.
    const creds = Array.from({ length: 5 }, (_, i) => makeCredential(`c${i}`, 1000));

    await persist(manager, creds);

    // Multiple slots were written (manifest + chunks), and none breaches the cap.
    expect(secureSet.mock.calls.length).toBeGreaterThan(1);
    for (const [, value] of secureSet.mock.calls) {
      expect(utf8ByteLength(value as string)).toBeLessThanOrEqual(SECURE_STORE_MAX_BYTES);
    }
  });

  it('round-trips a large credential set through a fresh manager instance', async () => {
    const writer = new NFCCredentialManager('http://localhost:8080');
    const creds = Array.from({ length: 6 }, (_, i) => makeCredential(`c${i}`, 900));
    await persist(writer, creds);

    // A brand-new instance sharing the same (in-memory) SecureStore must load
    // the identical credential set back.
    const reader = new NFCCredentialManager('http://localhost:8080');
    await reader.initialize();
    expect(reader.getCredentialById('c0')).toEqual(creds[0]);
    expect(reader.getCredentialById('c5')).toEqual(creds[5]);
    expect(reader.getActiveCredentials()).toHaveLength(6);
  });

  it('reads the legacy single-slot format (raw JSON array under the key)', async () => {
    const store = installStatefulSecureStore();
    const legacy = [makeCredential('legacy-1', 200)];
    store.set(CREDENTIALS_KEY, JSON.stringify(legacy));

    const manager = new NFCCredentialManager('http://localhost:8080');
    await manager.initialize();
    expect(manager.getCredentialById('legacy-1')).toEqual(legacy[0]);
  });

  it('cleans up stale chunks when the credential set shrinks', async () => {
    const store = installStatefulSecureStore();
    const manager = new NFCCredentialManager('http://localhost:8080');

    await persist(
      manager,
      Array.from({ length: 6 }, (_, i) => makeCredential(`c${i}`, 900))
    );
    const chunkKeysAfterLarge = [...store.keys()].filter((k) =>
      k.startsWith(`${CREDENTIALS_KEY}_c`)
    );
    expect(chunkKeysAfterLarge.length).toBeGreaterThan(1);

    // Shrink to a single small credential.
    await persist(manager, [makeCredential('c0', 100)]);
    const chunkKeysAfterSmall = [...store.keys()].filter((k) =>
      k.startsWith(`${CREDENTIALS_KEY}_c`)
    );
    expect(chunkKeysAfterSmall).toEqual([`${CREDENTIALS_KEY}_c0`]);

    // The shrunken set still reloads correctly (no stranded-chunk corruption).
    const reader = new NFCCredentialManager('http://localhost:8080');
    await reader.initialize();
    expect(reader.getActiveCredentials()).toHaveLength(1);
    expect(reader.getCredentialById('c0')).toBeDefined();
  });

  it('treats a missing chunk as corruption and falls back to an empty list', async () => {
    const store = installStatefulSecureStore();
    const manager = new NFCCredentialManager('http://localhost:8080');
    await persist(
      manager,
      Array.from({ length: 5 }, (_, i) => makeCredential(`c${i}`, 900))
    );

    // Simulate a torn write: drop one chunk slot.
    const chunkKey = [...store.keys()].find((k) => k.startsWith(`${CREDENTIALS_KEY}_c`));
    store.delete(chunkKey as string);

    const reader = new NFCCredentialManager('http://localhost:8080');
    await expect(reader.initialize()).resolves.toBeUndefined();
    expect(reader.getActiveCredentials()).toEqual([]);
    expect(warnSpy).toHaveBeenCalledWith(
      expect.stringContaining('credentials corrupted'),
      expect.anything()
    );
  });
});
