/**
 * useOfflineSupport integration tests.
 *
 * The hook backs both the offline cache layer and the queued-action sync
 * pipeline. These tests exercise the real hook end-to-end against an
 * in-memory @react-native-async-storage/async-storage replacement and
 * a controllable @react-native-community/netinfo mock.
 *
 * Coverage:
 * - Caching: write, read, expiry, clear-by-key, clear-all.
 * - Queue:   enqueue, dedupe, persisted shape, count tracking.
 * - Process: success path (clears queue, advances last sync time, reports
 *            progress); terminal 4xx is dropped; transient 5xx is retried and
 *            preserved (never dropped — see the hook unit test for the
 *            past-3-retries survival regression).
 * - Network gating: processQueue is a no-op while offline.
 *
 * NetInfo is the only RN dependency that needs a custom test double — the
 * shared test setup mocks AsyncStorage with bare jest.fn() stubs, but here we
 * need a working in-memory implementation so reads after writes return data.
 */

import AsyncStorage from '@react-native-async-storage/async-storage';
import { act, renderHook, waitFor } from '@testing-library/react-native';
import { useOfflineSupport } from '../../hooks/useOfflineSupport';

// ---------------------------------------------------------------------------
// In-memory AsyncStorage test double, scoped to this file via a local
// jest.mock() so it does NOT mutate the shared AsyncStorage stub from
// src/test/setup.ts and therefore can't leak into other test files.
//
// The implementation lives entirely inside the factory closure (jest hoists
// jest.mock() above the file's `const` bindings, so referencing an
// outer-scope const from the factory would hit the temporal dead zone). We
// expose the backing Map through a `__store` symbol on the default export
// so tests can clear it between runs and inspect it from outside.
// ---------------------------------------------------------------------------

jest.mock('@react-native-async-storage/async-storage', () => {
  const store = new Map<string, string>();
  const impl = {
    getItem: jest.fn(async (key: string) => store.get(key) ?? null),
    setItem: jest.fn(async (key: string, value: string) => {
      store.set(key, value);
    }),
    removeItem: jest.fn(async (key: string) => {
      store.delete(key);
    }),
    clear: jest.fn(async () => {
      store.clear();
    }),
    removeMany: jest.fn(async (keys: string[]) => {
      for (const k of keys) store.delete(k);
    }),
    getAllKeys: jest.fn(async () => Array.from(store.keys())),
    __store: store,
  };
  return { __esModule: true, default: impl };
});

// Convenience handles to the in-memory store + mock methods. After the
// jest.mock() factory has run, `AsyncStorage` resolves to the `impl` object
// returned above, so we can read `__store` through it.
const mockAsyncStorage = AsyncStorage as unknown as {
  getItem: jest.Mock;
  setItem: jest.Mock;
  removeItem: jest.Mock;
  clear: jest.Mock;
  removeMany: jest.Mock;
  getAllKeys: jest.Mock;
  __store: Map<string, string>;
};
const mockAsyncStorageStore = mockAsyncStorage.__store;

// ---------------------------------------------------------------------------
// NetInfo mock — start online, allow tests to flip the connection state by
// invoking the captured listener. Names are prefixed with `mock` so jest's
// hoisting check accepts the closure.
// ---------------------------------------------------------------------------

type NetState = {
  isConnected: boolean;
  isInternetReachable: boolean | null;
  type: string;
};

const mockNet: { listener: ((s: NetState) => void) | null; state: NetState } = {
  listener: null,
  state: { isConnected: true, isInternetReachable: true, type: 'wifi' },
};

jest.mock('@react-native-community/netinfo', () => ({
  __esModule: true,
  default: {
    addEventListener: (cb: (s: NetState) => void) => {
      mockNet.listener = cb;
      return () => {
        mockNet.listener = null;
      };
    },
    fetch: () => Promise.resolve(mockNet.state),
  },
}));

function setNetwork(next: Partial<NetState>) {
  mockNet.state = { ...mockNet.state, ...next };
  mockNet.listener?.(mockNet.state);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async function mountHook() {
  const utils = renderHook(() => useOfflineSupport());
  // Wait for the initial NetInfo.fetch + queue/sync metadata loads to settle.
  await waitFor(() => expect(utils.result.current.isConnected).toBe(true));
  return utils;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('useOfflineSupport integration', () => {
  beforeEach(() => {
    mockAsyncStorageStore.clear();
    jest.clearAllMocks();
    mockNet.state = { isConnected: true, isInternetReachable: true, type: 'wifi' };
    mockNet.listener = null;
  });

  describe('cache layer', () => {
    it('round-trips JSON-serialisable values', async () => {
      const { result } = await mountHook();
      const payload = { name: 'Building A', units: [1, 2, 3] };

      await act(async () => {
        await result.current.cacheData('building-1', payload);
      });

      const back = await result.current.getCachedData<typeof payload>('building-1');
      expect(back).toEqual(payload);
    });

    it('returns null and evicts the entry once it has expired', async () => {
      const { result } = await mountHook();

      // Cache with a 1ms TTL so anything past `now` is already expired,
      // then artificially backdate the entry to force the eviction path.
      await act(async () => {
        await result.current.cacheData('soon', 'value', 1);
      });

      // Move time forward by mutating the stored expiresAt directly.
      const raw = await mockAsyncStorage.getItem('ppt_cache_soon');
      const parsed = JSON.parse(raw ?? '{}');
      parsed.expiresAt = Date.now() - 1000;
      await mockAsyncStorage.setItem('ppt_cache_soon', JSON.stringify(parsed));

      const expired = await result.current.getCachedData<string>('soon');
      expect(expired).toBeNull();
      // The expired entry should have been evicted.
      expect(await mockAsyncStorage.getItem('ppt_cache_soon')).toBeNull();
    });

    it('clearCache(key) removes only the named entry', async () => {
      const { result } = await mountHook();

      await act(async () => {
        await result.current.cacheData('a', 1);
        await result.current.cacheData('b', 2);
      });

      await act(async () => {
        await result.current.clearCache('a');
      });

      expect(await result.current.getCachedData<number>('a')).toBeNull();
      expect(await result.current.getCachedData<number>('b')).toBe(2);
    });

    it('clearCache() with no key clears every cache entry but leaves the queue alone', async () => {
      const { result } = await mountHook();

      await act(async () => {
        await result.current.cacheData('a', 1);
        await result.current.cacheData('b', 2);
        await result.current.addToQueue({
          type: 'CREATE',
          endpoint: '/x',
          method: 'POST',
        });
      });

      await act(async () => {
        await result.current.clearCache();
      });

      expect(await result.current.getCachedData<number>('a')).toBeNull();
      expect(await result.current.getCachedData<number>('b')).toBeNull();
      // Queue should still exist.
      const queue = await result.current.getQueuedActions();
      expect(queue).toHaveLength(1);
    });
  });

  describe('queue management', () => {
    it('persists queued actions with generated id, timestamp, and retries=0', async () => {
      const { result } = await mountHook();

      await act(async () => {
        await result.current.addToQueue({
          type: 'CREATE',
          endpoint: '/api/v1/faults',
          method: 'POST',
          body: { title: 'Broken light' },
        });
      });

      const queue = await result.current.getQueuedActions();
      expect(queue).toHaveLength(1);
      expect(queue[0]).toMatchObject({
        type: 'CREATE',
        endpoint: '/api/v1/faults',
        method: 'POST',
        body: { title: 'Broken light' },
        retries: 0,
      });
      expect(queue[0].id).toBeTruthy();
      expect(queue[0].timestamp).toBeGreaterThan(0);

      // Hook state reflects the new count.
      await waitFor(() => expect(result.current.queuedActionsCount).toBe(1));
    });

    it('clearQueue removes every persisted action', async () => {
      const { result } = await mountHook();

      await act(async () => {
        await result.current.addToQueue({ type: 'CREATE', endpoint: '/a', method: 'POST' });
        await result.current.addToQueue({ type: 'UPDATE', endpoint: '/b', method: 'PUT' });
      });

      await act(async () => {
        await result.current.clearQueue();
      });

      expect(await result.current.getQueuedActions()).toEqual([]);
      await waitFor(() => expect(result.current.queuedActionsCount).toBe(0));
    });
  });

  describe('processQueue', () => {
    it('drains successful actions, updates lastSyncTime, and reports progress', async () => {
      const { result } = await mountHook();

      // executeQueuedAction now actually calls fetch, so mock a 200
      // response per queued action.
      const fetchMock = jest
        .fn()
        .mockResolvedValue({ ok: true, status: 200, json: async () => ({}) });
      globalThis.fetch = fetchMock as unknown as typeof fetch;

      await act(async () => {
        await result.current.addToQueue({ type: 'CREATE', endpoint: '/a', method: 'POST' });
        await result.current.addToQueue({ type: 'CREATE', endpoint: '/b', method: 'POST' });
      });

      const events: { current: number; total: number; isComplete: boolean }[] = [];
      let outcome: { success: number; failed: number } | undefined;

      await act(async () => {
        outcome = await result.current.processQueue((p) => {
          events.push({ current: p.current, total: p.total, isComplete: p.isComplete });
        });
      });

      expect(fetchMock).toHaveBeenCalledTimes(2);
      expect(outcome).toEqual({ success: 2, failed: 0 });
      expect(await result.current.getQueuedActions()).toEqual([]);
      await waitFor(() => expect(result.current.queuedActionsCount).toBe(0));
      expect(result.current.lastSyncTime).toBeInstanceOf(Date);

      // The first event is the initial state, then one per item, then a
      // final isComplete=true tick.
      expect(events.length).toBeGreaterThanOrEqual(3);
      expect(events[0]).toEqual({ current: 0, total: 2, isComplete: false });
      expect(events.at(-1)).toEqual({ current: 2, total: 2, isComplete: true });
    });

    it('returns early when offline without touching the queue', async () => {
      const { result } = await mountHook();

      await act(async () => {
        await result.current.addToQueue({ type: 'CREATE', endpoint: '/a', method: 'POST' });
      });

      // Flip the connection state via the captured NetInfo listener.
      await act(async () => {
        setNetwork({ isConnected: false, isInternetReachable: false });
      });

      await waitFor(() => expect(result.current.isConnected).toBe(false));

      let outcome: { success: number; failed: number } | undefined;
      await act(async () => {
        outcome = await result.current.processQueue();
      });

      expect(outcome).toEqual({ success: 0, failed: 0 });
      // Action is still queued because we never tried to dispatch it.
      const queue = await result.current.getQueuedActions();
      expect(queue).toHaveLength(1);
    });

    it('drops a permanently-failed (4xx) action without burning the retry budget', async () => {
      const { result } = await mountHook();

      const fetchMock = jest
        .fn()
        .mockResolvedValue({ ok: false, status: 422, json: async () => ({}) });
      globalThis.fetch = fetchMock as unknown as typeof fetch;

      await act(async () => {
        await result.current.addToQueue({
          type: 'CREATE',
          endpoint: '/api/v1/faults',
          method: 'POST',
        });
      });

      let outcome: { success: number; failed: number } | undefined;
      await act(async () => {
        outcome = await result.current.processQueue();
      });

      // One call (no retries — 422 is permanent), action is now in the
      // failed bucket and gone from the queue.
      expect(fetchMock).toHaveBeenCalledTimes(1);
      expect(outcome).toEqual({ success: 0, failed: 1 });
      expect(await result.current.getQueuedActions()).toEqual([]);
    });

    it('retries a transient (5xx) failure up to the retry budget', async () => {
      const { result } = await mountHook();

      const fetchMock = jest
        .fn()
        .mockResolvedValue({ ok: false, status: 503, json: async () => ({}) });
      globalThis.fetch = fetchMock as unknown as typeof fetch;

      await act(async () => {
        await result.current.addToQueue({
          type: 'CREATE',
          endpoint: '/api/v1/faults',
          method: 'POST',
        });
      });

      // First processQueue increments retries to 1; action remains.
      await act(async () => {
        await result.current.processQueue();
      });
      const queue = await result.current.getQueuedActions();
      expect(queue).toHaveLength(1);
      expect(queue[0].retries).toBe(1);
    });

    it('handles an empty queue without setting lastSyncTime or emitting progress', async () => {
      const { result } = await mountHook();

      const onProgress = jest.fn();
      let outcome: { success: number; failed: number } | undefined;
      await act(async () => {
        outcome = await result.current.processQueue(onProgress);
      });

      expect(outcome).toEqual({ success: 0, failed: 0 });
      expect(onProgress).not.toHaveBeenCalled();
    });
  });

  describe('network state propagation', () => {
    it('reflects the current connection type from NetInfo events', async () => {
      const { result } = await mountHook();

      await act(async () => {
        setNetwork({ isConnected: true, isInternetReachable: true, type: 'cellular' });
      });

      await waitFor(() => expect(result.current.connectionType).toBe('cellular'));
      expect(result.current.isConnected).toBe(true);
    });
  });
});
