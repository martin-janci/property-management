/**
 * Unit tests for `NFCCredentialManager`.
 *
 * Focus: the emergency "lost phone" revoke must clear the local credential
 * cache even when the server-side revoke call fails, so a failed server revoke
 * can never leave credentials usable offline (code-review finding
 * `code-review-mobile-rn-nfc-revoke-local-persists-offline`).
 */
import AsyncStorage from '@react-native-async-storage/async-storage';
import * as SecureStore from 'expo-secure-store';

import { NFCCredentialManager } from './NFCCredentialManager';
import type { NFCCredential } from './types';

const CREDENTIALS_KEY = 'ppt_nfc_credentials';

// In-memory SecureStore so we can assert exactly what is persisted.
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

function makeCredential(id: string): NFCCredential {
  return {
    id,
    buildingId: `building-${id}`,
    status: 'active',
    validFrom: '2020-01-01T00:00:00.000Z',
    validUntil: '2999-01-01T00:00:00.000Z',
    usageCount: 0,
  } as NFCCredential;
}

describe('NFCCredentialManager.emergencyRevokeAll', () => {
  let originalFetch: typeof globalThis.fetch;

  beforeEach(() => {
    jest.clearAllMocks();
    secureStore.clear();
    (AsyncStorage.getItem as jest.Mock).mockResolvedValue(null);
    originalFetch = globalThis.fetch;
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  it('clears the local credential cache when the server revoke succeeds', async () => {
    const okFetch = jest.fn(async () => ({
      ok: true,
      status: 200,
      json: async () => ({}),
    }));
    globalThis.fetch = okFetch as unknown as typeof fetch;

    const mgr = new NFCCredentialManager('http://localhost:8080');
    secureStore.set(CREDENTIALS_KEY, JSON.stringify([makeCredential('a')]));
    await mgr.initialize();
    expect(mgr.getActiveCredentials()).toHaveLength(1);

    await mgr.emergencyRevokeAll();

    expect(mgr.getActiveCredentials()).toHaveLength(0);
    expect(secureStore.get(CREDENTIALS_KEY)).toBe('[]');
  });

  it('clears the local credential cache even when the server revoke fails, and surfaces the error', async () => {
    // Server revoke fails (e.g. device offline / server error). The panic
    // action must still block offline use of the cached credentials.
    const failingFetch = jest.fn(async () => ({
      ok: false,
      status: 503,
      json: async () => ({}),
    }));
    globalThis.fetch = failingFetch as unknown as typeof fetch;

    const mgr = new NFCCredentialManager('http://localhost:8080');
    secureStore.set(CREDENTIALS_KEY, JSON.stringify([makeCredential('a'), makeCredential('b')]));
    await mgr.initialize();
    expect(mgr.getActiveCredentials()).toHaveLength(2);

    // The failure must surface so the caller can prompt a retry.
    await expect(mgr.emergencyRevokeAll()).rejects.toThrow();

    // ...but the local cache — in memory AND persisted — must be cleared so the
    // credentials cannot be presented at an access point offline.
    expect(mgr.getActiveCredentials()).toHaveLength(0);
    expect(mgr.getCredentialForBuilding('building-a')).toBeUndefined();
    expect(secureStore.get(CREDENTIALS_KEY)).toBe('[]');
  });
});
