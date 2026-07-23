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

const mockStore = (AsyncStorage as unknown as { __store: Map<string, string> }).__store;

describe('resetLocalData', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockStore.clear();
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
    // Keys outside the tenant-cache namespace must survive. `ppt_access_token`
    // starts with `ppt_` but not `ppt_cache_`, so prefix scoping must not
    // over-match it.
    mockStore.set('ppt_access_token', 'tok');
    mockStore.set('some_other_key', 'x');

    await resetLocalData();

    expect(Array.from(mockStore.keys()).sort()).toEqual(['ppt_access_token', 'some_other_key']);
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
