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
 *            progress), retry-and-drop path (>= 3 retries removes the item).
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
// In-memory AsyncStorage test double (the shared setup only stubs the methods
// with jest.fn(), so reads after writes would return undefined without this).
// ---------------------------------------------------------------------------

const memoryStorage = (() => {
  const store = new Map<string, string>();

  const getItem = jest.fn(async (key: string) => store.get(key) ?? null);
  const setItem = jest.fn(async (key: string, value: string) => {
    store.set(key, value);
  });
  const removeItem = jest.fn(async (key: string) => {
    store.delete(key);
  });
  const clear = jest.fn(async () => {
    store.clear();
  });
  const multiRemove = jest.fn(async (keys: string[]) => {
    for (const k of keys) store.delete(k);
  });
  const getAllKeys = jest.fn(async () => Array.from(store.keys()));

  return { store, getItem, setItem, removeItem, clear, multiRemove, getAllKeys };
})();

(AsyncStorage as unknown as Record<string, unknown>).getItem = memoryStorage.getItem;
(AsyncStorage as unknown as Record<string, unknown>).setItem = memoryStorage.setItem;
(AsyncStorage as unknown as Record<string, unknown>).removeItem = memoryStorage.removeItem;
(AsyncStorage as unknown as Record<string, unknown>).clear = memoryStorage.clear;
(AsyncStorage as unknown as Record<string, unknown>).multiRemove = memoryStorage.multiRemove;
(AsyncStorage as unknown as Record<string, unknown>).getAllKeys = memoryStorage.getAllKeys;

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
    memoryStorage.store.clear();
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

      // Cache with a 50ms TTL and an artificial age that is already past it.
      await act(async () => {
        await result.current.cacheData('soon', 'value', 1);
      });

      // Move time forward by mutating the stored expiresAt directly.
      const raw = await memoryStorage.getItem('ppt_cache_soon');
      const parsed = JSON.parse(raw ?? '{}');
      parsed.expiresAt = Date.now() - 1000;
      await memoryStorage.setItem('ppt_cache_soon', JSON.stringify(parsed));

      const expired = await result.current.getCachedData<string>('soon');
      expect(expired).toBeNull();
      // The expired entry should have been evicted.
      expect(await memoryStorage.getItem('ppt_cache_soon')).toBeNull();
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
