/**
 * Unit tests for `resetLocalData` (issue #2399).
 *
 * Uses an in-memory `@react-native-async-storage/async-storage` replacement so
 * we can assert exactly which keys are purged and — just as importantly —
 * which are left untouched.
 */
import AsyncStorage from '@react-native-async-storage/async-storage';
import { resetLocalData } from './resetLocalData';

jest.mock('@react-native-async-storage/async-storage', () => {
  const store = new Map<string, string>();
  return {
    __store: store,
    getItem: jest.fn(async (k: string) => store.get(k) ?? null),
    setItem: jest.fn(async (k: string, v: string) => {
      store.set(k, v);
    }),
    removeItem: jest.fn(async (k: string) => {
      store.delete(k);
    }),
    getAllKeys: jest.fn(async () => Array.from(store.keys())),
    removeMany: jest.fn(async (keys: string[]) => {
      for (const k of keys) store.delete(k);
    }),
  };
});

// NFC credential material lives in expo-secure-store, not AsyncStorage, so the
// purge reaches into it via `NFCCredentialManager.clearAllLocalCredentials()`.
// Back it with an in-memory map so we can assert the credential blob + chunk
// slots are gone while unrelated SecureStore keys (auth tokens) survive.
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

const mockStore = (AsyncStorage as unknown as { __store: Map<string, string> }).__store;
const mockSecureStore = (jest.requireMock('expo-secure-store') as { __store: Map<string, string> })
  .__store;

describe('resetLocalData', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockStore.clear();
    mockSecureStore.clear();
  });

  it('purges offline cache, queue, sync marker, widget and layout namespaces', async () => {
    mockStore.set('ppt_cache_faults_list', '[]');
    mockStore.set('ppt_cache_buildings', '[]');
    mockStore.set('ppt_offline_queue', '[]');
    mockStore.set('ppt_last_sync', '111');
    mockStore.set('@ppt/widget_data', '{}');
    mockStore.set('@ppt/widget_configs', '[]');
    // Tenant-scoped dashboard layout cache — swept by `ppt_layout_` prefix
    // regardless of the dynamic scope/screen suffix (issue #2486).
    mockStore.set('ppt_layout_org-a_ppt_dashboard', '{}');
    mockStore.set('ppt_layout_org-b_ppt_dashboard', '{}');
    // Feature caches that hold tenant-scoped history/queues (issue #2947).
    mockStore.set('@ppt/access_log', '[]');
    mockStore.set('@ppt/qr_scan_history', '[]');
    mockStore.set('@ppt/feedback_drafts', '[]');
    mockStore.set('@ppt/pending_feedback', '[]');
    mockStore.set('@ppt/faq_votes', '{}');
    // Keys outside the tenant-cache namespace must survive. `ppt_access_token`
    // starts with `ppt_` but not `ppt_cache_`, so prefix scoping must not
    // over-match it.
    mockStore.set('ppt_access_token', 'tok');
    mockStore.set('some_other_key', 'x');

    await resetLocalData();

    expect(Array.from(mockStore.keys()).sort()).toEqual(['ppt_access_token', 'some_other_key']);
  });

  // issue #2947 — the purge omitted these tenant-scoped feature caches, so on a
  // shared device the next tenant saw the prior tenant's NFC access log / QR
  // scan history, and queued offline feedback flushed under the wrong identity.
  it.each([
    '@ppt/access_log',
    '@ppt/qr_scan_history',
    '@ppt/feedback_drafts',
    '@ppt/pending_feedback',
    '@ppt/faq_votes',
  ])('purges tenant-scoped feature cache %s on session change (#2947)', async (key) => {
    mockStore.set(key, '[]');
    // An unrelated `@ppt/` key that is not tenant-scoped must survive, so the
    // fix cannot be a blanket `@ppt/` wipe.
    mockStore.set('@ppt/theme', 'dark');

    await resetLocalData();

    expect(mockStore.has(key)).toBe(false);
    expect(mockStore.get('@ppt/theme')).toBe('dark');
  });

  // issue #2953 — the actual NFC building-access credentials live in
  // expo-secure-store (encrypted, chunked) plus a legacy unencrypted
  // AsyncStorage migration blob, neither covered by the AsyncStorage sweep. On
  // a shared device the prior tenant's credential material survived the handoff
  // and the next tenant's loadStoredCredentials() could adopt it. The
  // session-change purge must now clear both surfaces.
  it('purges NFC credential material from SecureStore and the legacy key (#2953)', async () => {
    // Encrypted chunked credential blob: manifest + one chunk slot.
    mockSecureStore.set('ppt_nfc_credentials', JSON.stringify({ v: 1, chunks: 1 }));
    mockSecureStore.set('ppt_nfc_credentials_c0', '[{"id":"cred-a"}]');
    // Legacy unencrypted migration blob in AsyncStorage.
    mockStore.set('@ppt/nfc_credentials', '[{"id":"cred-a"}]');
    // Auth tokens also live in SecureStore but are owned by AuthContext, not
    // this purge — they must survive so scoping stays tight.
    mockSecureStore.set('ppt_access_token', 'tok');

    await resetLocalData();

    expect(mockSecureStore.has('ppt_nfc_credentials')).toBe(false);
    expect(mockSecureStore.has('ppt_nfc_credentials_c0')).toBe(false);
    expect(mockStore.has('@ppt/nfc_credentials')).toBe(false);
    expect(mockSecureStore.get('ppt_access_token')).toBe('tok');
  });

  it('purges NFC credentials even when the AsyncStorage sweep throws (#2953)', async () => {
    // The credential purge must not be skipped when the (less sensitive)
    // AsyncStorage sweep fails first — it runs in its own best-effort block.
    (AsyncStorage.getAllKeys as jest.Mock).mockRejectedValueOnce(new Error('boom'));
    mockSecureStore.set('ppt_nfc_credentials', JSON.stringify({ v: 1, chunks: 1 }));
    mockSecureStore.set('ppt_nfc_credentials_c0', '[{"id":"cred-a"}]');

    await resetLocalData();

    expect(mockSecureStore.has('ppt_nfc_credentials')).toBe(false);
    expect(mockSecureStore.has('ppt_nfc_credentials_c0')).toBe(false);
  });

  it('does not call removeMany when nothing matches', async () => {
    mockStore.set('unrelated', '1');

    await resetLocalData();

    expect(AsyncStorage.removeMany).not.toHaveBeenCalled();
    expect(mockStore.has('unrelated')).toBe(true);
  });

  it('swallows storage errors so it can never wedge login/logout', async () => {
    (AsyncStorage.getAllKeys as jest.Mock).mockRejectedValueOnce(new Error('boom'));

    await expect(resetLocalData()).resolves.toBeUndefined();
  });
});
