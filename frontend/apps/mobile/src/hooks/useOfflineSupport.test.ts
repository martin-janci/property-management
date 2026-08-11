/**
 * useOfflineSupport — offline queue concurrency / ordering / edit-dedup tests
 * (issue #1767).
 *
 * These pin the four queue-correctness fixes:
 *   (a) processQueue is re-entrant-safe — a second call while the first is in
 *       flight is a no-op, so every action's fetch fires exactly once.
 *   (b) an action enqueued *during* a flush is not clobbered by the queue
 *       write-back (read-merge, not overwrite).
 *   (c) strict FIFO across retry cycles — a later UPDATE never overtakes an
 *       earlier CREATE that failed and is being retried.
 *   (d) offline-edit linkage — after a CREATE returns its server id, a queued
 *       UPDATE that targeted the temp id is replayed against the real id.
 *   (e) halted-cycle honesty (issue #2637) — a cycle that stops early on a
 *       retryable head-of-line failure reports isComplete:false with pending>0,
 *       never a false completion.
 *
 * The hook is a real React hook (useState/useRef/useEffect), so it is driven
 * through a tiny `react-test-renderer` harness. We deliberately do NOT import
 * `@testing-library/react-native` / `renderHook` — that wrapper hard-fails on
 * the repo's `react-test-renderer` 19.2.0-vs-19.2.6 peer-dep mismatch (the same
 * reason the other hook tests avoid it). `react-test-renderer` itself loads
 * fine and is all we need.
 *
 * AsyncStorage is mocked with an in-memory store; NetInfo, expo-secure-store,
 * and global.fetch are mocked so the queue logic runs without a native runtime.
 */

import { createElement } from 'react';
// `react-test-renderer` ships no bundled types and the repo has no
// `@types/react-test-renderer` (the other tests avoid this module entirely —
// see the file header). Suppress the resulting implicit-any on the import
// rather than pull in a new dependency for a single test file.
// @ts-expect-error -- no type declarations for react-test-renderer
import TestRenderer, { act } from 'react-test-renderer';

// A shared in-memory backing store for the AsyncStorage mock. It lives on
// globalThis because a `jest.mock` factory (hoisted above all imports) may only
// reference globals — not module-scope `const`s.
const TEST_GLOBAL = globalThis as typeof globalThis & {
  __offlineStore?: Map<string, string>;
};
TEST_GLOBAL.__offlineStore = new Map<string, string>();

// --- In-memory AsyncStorage ------------------------------------------------

jest.mock('@react-native-async-storage/async-storage', () => {
  const g = globalThis as typeof globalThis & { __offlineStore: Map<string, string> };
  return {
    getItem: jest.fn(async (key: string) =>
      g.__offlineStore.has(key) ? g.__offlineStore.get(key) : null
    ),
    setItem: jest.fn(async (key: string, value: string) => {
      g.__offlineStore.set(key, value);
    }),
    removeItem: jest.fn(async (key: string) => {
      g.__offlineStore.delete(key);
    }),
    getAllKeys: jest.fn(async () => Array.from(g.__offlineStore.keys())),
    removeMany: jest.fn(async (keys: string[]) => {
      for (const k of keys) g.__offlineStore.delete(k);
    }),
  };
});

// --- NetInfo: report a connected, internet-reachable network ---------------

jest.mock('@react-native-community/netinfo', () => {
  const connectedState = { isConnected: true, isInternetReachable: true, type: 'wifi' };
  return {
    __esModule: true,
    default: {
      addEventListener: jest.fn(() => () => undefined),
      fetch: jest.fn(async () => connectedState),
    },
  };
});

// --- expo-secure-store: no token (keeps headers simple) --------------------

jest.mock('expo-secure-store', () => ({
  getItemAsync: jest.fn(async () => null),
}));

// --- config/api: stable base URL -------------------------------------------

jest.mock('../config/api', () => ({
  getApiBaseUrl: () => 'https://api.test',
}));

const store = TEST_GLOBAL.__offlineStore;

import {
  type QueuedAction,
  type SyncProgress,
  type UseOfflineSupportReturn,
  useOfflineSupport,
} from './useOfflineSupport';

// Mirror of the hook's private `QUEUE_KEY` constant. Kept in sync by the tests
// below, which exercise the same persisted key the hook reads/writes.
const QUEUE_KEY = 'ppt_offline_queue';

// --- Harness: drive the hook via react-test-renderer -----------------------

let api: UseOfflineSupportReturn;
function Harness() {
  api = useOfflineSupport();
  return null;
}

/** Mount the hook and flush its mount effects (NetInfo.fetch resolving). */
async function mountHook(): Promise<void> {
  await act(async () => {
    TestRenderer.create(createElement(Harness));
  });
  // Let the NetInfo.fetch().then(...) microtask settle so isInternetReachable
  // flips to true (processQueue early-returns while it is null).
  await act(async () => {
    await Promise.resolve();
  });
}

/** A deferred promise whose resolve we control, to hold a fetch in flight. */
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

/** Seed the persisted queue directly (bypassing addToQueue) with full actions. */
function seedQueue(actions: QueuedAction[]) {
  store.set(QUEUE_KEY, JSON.stringify(actions));
}

function readQueue(): QueuedAction[] {
  const raw = store.get(QUEUE_KEY);
  return raw ? (JSON.parse(raw) as QueuedAction[]) : [];
}

function ok(body: unknown = {}, status = 200): Response {
  return {
    ok: status >= 200 && status < 300,
    status,
    json: async () => body,
  } as unknown as Response;
}

function serverError(status = 503): Response {
  return { ok: false, status, json: async () => ({}) } as unknown as Response;
}

beforeEach(() => {
  store.clear();
  jest.clearAllMocks();
  (globalThis as { fetch?: unknown }).fetch = jest.fn();
});

function makeAction(over: Partial<QueuedAction> & Pick<QueuedAction, 'id'>): QueuedAction {
  return {
    type: 'CREATE',
    endpoint: '/api/v1/faults',
    method: 'POST',
    body: { title: 't' },
    timestamp: Date.now(),
    retries: 0,
    ...over,
  };
}

describe('useOfflineSupport.processQueue — issue #1767', () => {
  it('(a) is re-entrant safe: each queued action dispatches exactly once', async () => {
    seedQueue([makeAction({ id: 'a1' }), makeAction({ id: 'a2' })]);
    const fetchMock = globalThis.fetch as jest.Mock;
    fetchMock.mockResolvedValue(ok({ id: 'srv' }));

    await mountHook();

    // Fire two overlapping flushes without awaiting the first.
    await act(async () => {
      const p1 = api.processQueue();
      const p2 = api.processQueue();
      await Promise.all([p1, p2]);
    });

    // Two actions, each fetched once — the second processQueue was a no-op.
    expect(fetchMock).toHaveBeenCalledTimes(2);
    const urls = fetchMock.mock.calls.map((c) => c[0]);
    expect(urls.filter((u) => u === 'https://api.test/api/v1/faults')).toHaveLength(2);
    // Both drained.
    expect(readQueue()).toHaveLength(0);
  });

  it('(b) does not lose an action enqueued during an in-flight flush', async () => {
    seedQueue([makeAction({ id: 'A', body: { title: 'A' } })]);
    const fetchMock = globalThis.fetch as jest.Mock;
    const d = deferred<Response>();
    fetchMock.mockReturnValueOnce(d.promise); // hold A's fetch in flight

    await mountHook();

    let flush!: Promise<{ success: number; failed: number }>;
    await act(async () => {
      flush = api.processQueue();
      // While A is still in flight, the user enqueues B.
      await api.addToQueue({
        type: 'CREATE',
        endpoint: '/api/v1/faults',
        method: 'POST',
        body: { title: 'B' },
      });
      // Now let A resolve and the flush finish.
      d.resolve(ok({ id: 'srv-A' }));
      await flush;
    });

    // A succeeded and was removed; B, enqueued mid-flush, survived.
    const remaining = readQueue();
    const bodies = remaining.map((a) => (a.body as { title: string }).title);
    expect(bodies).toContain('B');
    expect(bodies).not.toContain('A');
  });

  it('(c) strict FIFO: a later UPDATE never fires before an earlier failed CREATE succeeds', async () => {
    seedQueue([
      makeAction({ id: 'create', type: 'CREATE', endpoint: '/api/v1/faults', body: { t: 'c' } }),
      makeAction({
        id: 'update',
        type: 'UPDATE',
        method: 'PUT',
        endpoint: '/api/v1/faults/known-id',
        body: { t: 'u' },
      }),
    ]);
    const fetchMock = globalThis.fetch as jest.Mock;
    const callOrder: string[] = [];
    let createAttempts = 0;
    fetchMock.mockImplementation(async (url: string) => {
      callOrder.push(url);
      if (url.endsWith('/api/v1/faults')) {
        createAttempts++;
        if (createAttempts === 1) return serverError(503); // CREATE fails once
        return ok({ id: 'srv-create' });
      }
      return ok({});
    });

    await mountHook();

    // First cycle: CREATE fails (5xx, retryable), so the UPDATE must NOT run.
    await act(async () => {
      await api.processQueue();
    });
    expect(callOrder).toEqual(['https://api.test/api/v1/faults']);
    expect(callOrder).not.toContain('https://api.test/api/v1/faults/known-id');

    // Second cycle: CREATE succeeds, then the UPDATE finally fires — after it.
    await act(async () => {
      await api.processQueue();
    });
    expect(callOrder).toEqual([
      'https://api.test/api/v1/faults',
      'https://api.test/api/v1/faults',
      'https://api.test/api/v1/faults/known-id',
    ]);
  });

  it('(d) offline-edit linkage: a queued UPDATE is replayed against the CREATE server id', async () => {
    const TEMP = 'temp-fault-1';
    seedQueue([
      makeAction({
        id: 'create',
        type: 'CREATE',
        endpoint: '/api/v1/faults',
        body: { title: 'leak', localRef: TEMP },
        localId: TEMP,
      }),
      makeAction({
        id: 'update',
        type: 'UPDATE',
        method: 'PUT',
        endpoint: `/api/v1/faults/${TEMP}`,
        body: { id: TEMP, priority: 'high' },
        localId: TEMP,
      }),
    ]);
    const fetchMock = globalThis.fetch as jest.Mock;
    const calls: Array<{ url: string; body: string | undefined }> = [];
    fetchMock.mockImplementation(async (url: string, init: RequestInit) => {
      calls.push({ url, body: init.body as string | undefined });
      if (url.endsWith('/api/v1/faults')) return ok({ id: 'srv-1' });
      return ok({});
    });

    await mountHook();
    await act(async () => {
      await api.processQueue();
    });

    const updateCall = calls.find((c) => c.url.includes('/api/v1/faults/'));
    expect(updateCall).toBeDefined();
    // URL temp id rewritten to the server id…
    expect(updateCall?.url).toBe('https://api.test/api/v1/faults/srv-1');
    expect(updateCall?.url).not.toContain(TEMP);
    // …and the temp id inside the body too.
    expect(updateCall?.body).toContain('srv-1');
    expect(updateCall?.body).not.toContain(TEMP);
    // Queue fully drained.
    expect(readQueue()).toHaveLength(0);
  });

  it('(e) does NOT report isComplete after a head-of-line-blocked (halted) cycle', async () => {
    // issue #2637: a retryable failure halts the flush mid-cycle. The final
    // progress must NOT claim the queue is complete while items are still
    // pending — the old code unconditionally overwrote the halted state with
    // { isComplete: true } after the loop.
    seedQueue([
      makeAction({ id: 'create', type: 'CREATE', endpoint: '/api/v1/faults', body: { t: 'c' } }),
      makeAction({
        id: 'update',
        type: 'UPDATE',
        method: 'PUT',
        endpoint: '/api/v1/faults/known-id',
        body: { t: 'u' },
      }),
    ]);
    const fetchMock = globalThis.fetch as jest.Mock;
    // The head-of-line CREATE fails with a retryable 5xx, halting the cycle.
    fetchMock.mockImplementation(async (url: string) => {
      if (url.endsWith('/api/v1/faults')) return serverError(503);
      return ok({});
    });

    await mountHook();

    const events: SyncProgress[] = [];
    let result!: { success: number; failed: number };
    await act(async () => {
      result = await api.processQueue((p) => events.push({ ...p }));
    });

    // The CREATE blocked the UPDATE, so nothing succeeded and nothing was
    // terminally dropped (503 is retryable) — one attempt, then halt.
    expect(result).toEqual({ success: 0, failed: 0 });

    // The FINAL progress tick must reflect a halt, not a completion.
    const last = events.at(-1);
    expect(last).toBeDefined();
    expect(last?.isComplete).toBe(false);
    expect(last?.pending).toBeGreaterThan(0);

    // The exposed syncProgress state agrees with the callback's last tick.
    expect(api.syncProgress?.isComplete).toBe(false);
    expect(api.syncProgress?.pending).toBeGreaterThan(0);

    // Both actions survive in the queue for the next reconnect (strict FIFO).
    expect(readQueue().map((a) => a.id)).toEqual(['create', 'update']);
  });

  it('reports isComplete with pending:0 when the queue fully drains', async () => {
    // Positive counterpart to (e): a clean drain must still report completion.
    seedQueue([makeAction({ id: 'a1' }), makeAction({ id: 'a2' })]);
    const fetchMock = globalThis.fetch as jest.Mock;
    fetchMock.mockResolvedValue(ok({ id: 'srv' }));

    await mountHook();

    const events: SyncProgress[] = [];
    await act(async () => {
      await api.processQueue((p) => events.push({ ...p }));
    });

    const last = events.at(-1);
    expect(last?.isComplete).toBe(true);
    expect(last?.pending).toBe(0);
    expect(readQueue()).toHaveLength(0);
  });

  it('(f) transient (5xx) failure is NEVER dropped, even past the old 3-retry budget, and keeps being re-attempted', async () => {
    // Regression for silent offline data loss: a queued action that keeps
    // hitting a transient 5xx used to be permanently discarded (with only a
    // console.error) once `retries >= 3`, losing offline-created content.
    // With head-of-line halting (issue #2637) the action also stays at the
    // front of the queue across reconnects, but the drop-condition fix is what
    // guarantees it is never discarded no matter how many times it is retried.
    seedQueue([makeAction({ id: 'transient', body: { title: 'reported-offline' } })]);
    const fetchMock = globalThis.fetch as jest.Mock;
    // Server is down for the whole test — every attempt is a transient 5xx.
    fetchMock.mockResolvedValue(serverError(503));

    await mountHook();

    // Run four reconnect cycles — one more than the old drop threshold (3).
    for (let cycle = 0; cycle < 4; cycle++) {
      await act(async () => {
        await api.processQueue();
      });
    }

    // The action was re-attempted on every cycle (not dropped after 3)…
    expect(fetchMock).toHaveBeenCalledTimes(4);

    // …and it is STILL in the queue — offline content survived.
    const remaining = readQueue();
    expect(remaining).toHaveLength(1);
    expect(remaining[0].id).toBe('transient');
    expect((remaining[0].body as { title: string }).title).toBe('reported-offline');
    // Retry counter reflects the attempts (observability), but did not cause a drop.
    expect(remaining[0].retries).toBeGreaterThanOrEqual(4);

    // One more cycle, now with the server recovered, drains it successfully.
    fetchMock.mockResolvedValue(ok({ id: 'srv-late' }));
    await act(async () => {
      await api.processQueue();
    });
    expect(readQueue()).toHaveLength(0);
  });

  it('(g) preserves temp→server id linkage for a dependent edit carried past a mid-cycle halt', async () => {
    // Regression: the temp→server id mapping is in-memory for one cycle only.
    // If a CREATE resolves its temp id but an UNRELATED action between it and
    // its dependent UPDATE fails transiently (halting the cycle), the dependent
    // UPDATE sits *after* the halt point and was never remapped in-loop. Unless
    // it is remapped before persisting, next cycle the CREATE is gone, the
    // mapping is lost, and the edit orphans onto a temp id the server never
    // knew — a silent loss of an offline edit.
    const TEMP = 'temp-fault-g';
    seedQueue([
      makeAction({
        id: 'create',
        type: 'CREATE',
        endpoint: '/api/v1/faults',
        body: { title: 'leak', localRef: TEMP },
        localId: TEMP,
      }),
      // Unrelated action that fails transiently and halts the cycle — it sits
      // BETWEEN the create and the dependent update.
      makeAction({
        id: 'blocker',
        type: 'UPDATE',
        method: 'PUT',
        endpoint: '/api/v1/announcements/known-id',
        body: { seen: true },
      }),
      // Dependent update targeting the temp id — never reached this cycle.
      makeAction({
        id: 'dependent',
        type: 'UPDATE',
        method: 'PUT',
        endpoint: `/api/v1/faults/${TEMP}`,
        body: { id: TEMP, priority: 'high' },
        localId: TEMP,
      }),
    ]);
    const fetchMock = globalThis.fetch as jest.Mock;
    fetchMock.mockImplementation(async (url: string) => {
      if (url.endsWith('/api/v1/faults')) return ok({ id: 'srv-g' }); // CREATE resolves
      if (url.includes('/api/v1/announcements/')) return serverError(503); // blocker halts
      return ok({});
    });

    await mountHook();

    // Cycle 1: CREATE succeeds and resolves TEMP→srv-g; the blocker 503s and
    // halts the cycle before the dependent update is reached.
    await act(async () => {
      await api.processQueue();
    });

    // The dependent's persisted endpoint/body must already point at the server
    // id — the mapping was applied before persisting, not lost with the cycle.
    const afterHalt = readQueue();
    const dependent = afterHalt.find((a) => a.id === 'dependent');
    expect(dependent).toBeDefined();
    expect(dependent?.endpoint).toBe('/api/v1/faults/srv-g');
    expect(dependent?.endpoint).not.toContain(TEMP);
    expect(JSON.stringify(dependent?.body)).toContain('srv-g');
    expect(JSON.stringify(dependent?.body)).not.toContain(TEMP);
    // The CREATE is gone (succeeded); blocker + dependent remain, in FIFO order.
    expect(afterHalt.map((a) => a.id)).toEqual(['blocker', 'dependent']);

    // Cycle 2: the server has recovered — the dependent now hits the real
    // resource and drains cleanly (rather than orphaning onto the temp id).
    fetchMock.mockImplementation(async () => ok({}));
    await act(async () => {
      await api.processQueue();
    });
    const calls = fetchMock.mock.calls.map((c) => c[0] as string);
    expect(calls).toContain('https://api.test/api/v1/faults/srv-g');
    expect(calls.every((u) => !u.includes(TEMP))).toBe(true);
    expect(readQueue()).toHaveLength(0);
  });
});
