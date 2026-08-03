import AsyncStorage from '@react-native-async-storage/async-storage';
import NetInfo, { type NetInfoState } from '@react-native-community/netinfo';
import * as SecureStore from 'expo-secure-store';
import { useCallback, useEffect, useRef, useState } from 'react';
import { getApiBaseUrl } from '../config/api';
import { CACHE_PREFIX, LAST_SYNC_KEY, QUEUE_KEY } from '../services/localCacheKeys';
import { extractTenantId } from '../utils/jwt';

// Storage keys. The AsyncStorage cache/queue namespaces live in
// `services/localCacheKeys` (shared with `resetLocalData`, issue #2399). The
// access token is a SecureStore key, not part of that purgeable namespace, so
// it stays local here.
const ACCESS_TOKEN_KEY = 'ppt_access_token';

export interface CacheOptions {
  expiresIn?: number; // milliseconds
  key: string;
}

export type SyncItemStatus = 'pending' | 'syncing' | 'synced' | 'failed';

export interface QueuedAction {
  id: string;
  type: 'CREATE' | 'UPDATE' | 'DELETE';
  endpoint: string;
  method: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE';
  body?: unknown;
  timestamp: number;
  retries: number;
  syncStatus?: SyncItemStatus;
  /**
   * Offline-edit linkage (issue #1767).
   *
   * On a CREATE this carries the temporary client-side id the entity was given
   * while offline (e.g. a fault the user reported before reconnecting). The
   * entity has no server id until the CREATE flushes, so any UPDATE/DELETE the
   * user queues against that same entity references the temp id instead.
   *
   * - On a CREATE: the temp id this action will materialise into a real id.
   * - On a dependent UPDATE/DELETE: the temp id it targets. After the CREATE
   *   replays and the server returns the real id, `processQueue` rewrites this
   *   action's `endpoint`/`body` occurrences of the temp id to the server id so
   *   the replayed edit hits the right resource.
   */
  localId?: string;
}

/**
 * Pull a server-assigned id out of a CREATE response (issue #1767).
 *
 * The api-server create endpoints return the new resource id under `id`
 * (e.g. `POST /api/v1/faults` → `{ id, message }`). Kept lenient — anything
 * non-string yields null, so an unexpected body simply skips id linkage rather
 * than corrupting a queued edit.
 */
function extractServerId(response: Record<string, unknown>): string | null {
  const id = response.id;
  return typeof id === 'string' && id.length > 0 ? id : null;
}

/**
 * Rewrite a queued action's references to a temp id with the real server id
 * (issue #1767, offline-edit linkage).
 *
 * A UPDATE/DELETE that was queued against an offline-created entity targets the
 * temp id in two places:
 *   - its `endpoint` path segment (e.g. `/api/v1/faults/<tempId>`), and
 *   - optionally its `localId` marker and any id field in its JSON `body`.
 * Once the CREATE flushes and we learn the real id, replace every occurrence so
 * the replayed edit hits the right resource. Mutates `action` in place.
 */
function remapActionIds(action: QueuedAction, mapping: Map<string, string>): void {
  if (mapping.size === 0) return;

  // The temp id this action targets, if recorded explicitly.
  const serverId = action.localId ? mapping.get(action.localId) : undefined;

  // Serialize the body once so we can do a single string replace pass over both
  // string and object bodies. Objects are round-tripped through JSON; a body
  // that isn't JSON-serializable is left untouched.
  let bodyJson: string | null = null;
  if (typeof action.body === 'string') {
    bodyJson = action.body;
  } else if (action.body !== undefined && action.body !== null) {
    try {
      bodyJson = JSON.stringify(action.body);
    } catch {
      bodyJson = null;
    }
  }
  const bodyWasObject = bodyJson !== null && typeof action.body !== 'string';

  for (const [tempId, realId] of mapping) {
    if (action.endpoint.includes(tempId)) {
      action.endpoint = action.endpoint.split(tempId).join(realId);
    }
    if (bodyJson?.includes(tempId)) {
      bodyJson = bodyJson.split(tempId).join(realId);
    }
  }

  if (bodyJson !== null) {
    if (bodyWasObject) {
      try {
        action.body = JSON.parse(bodyJson);
      } catch {
        // leave the original object body in place if it somehow won't reparse
      }
    } else {
      action.body = bodyJson;
    }
  }

  // Once linked, clear the marker so a later cycle does not try to remap again
  // against a temp id that no longer exists in the mapping.
  if (serverId) {
    action.localId = undefined;
  }
}

export interface SyncProgress {
  total: number;
  current: number;
  failed: number;
  isComplete: boolean;
}

export type SyncProgressCallback = (progress: SyncProgress) => void;

export interface OfflineState {
  isConnected: boolean;
  isInternetReachable: boolean | null;
  connectionType: string | null;
  queuedActionsCount: number;
  lastSyncTime: Date | null;
}

export interface UseOfflineSupportReturn extends OfflineState {
  // Caching
  cacheData: <T>(key: string, data: T, expiresIn?: number) => Promise<void>;
  getCachedData: <T>(key: string) => Promise<T | null>;
  clearCache: (key?: string) => Promise<void>;
  // Offline queue
  addToQueue: (action: Omit<QueuedAction, 'id' | 'timestamp' | 'retries'>) => Promise<void>;
  getQueuedActions: () => Promise<QueuedAction[]>;
  processQueue: (onProgress?: SyncProgressCallback) => Promise<{ success: number; failed: number }>;
  clearQueue: () => Promise<void>;
  // Sync
  syncData: (onProgress?: SyncProgressCallback) => Promise<void>;
  isSyncing: boolean;
  // Sync progress (for UI binding)
  syncProgress: SyncProgress | null;
}

export function useOfflineSupport(): UseOfflineSupportReturn {
  const [isConnected, setIsConnected] = useState(true);
  const [isInternetReachable, setIsInternetReachable] = useState<boolean | null>(null);
  const [connectionType, setConnectionType] = useState<string | null>(null);
  const [queuedActionsCount, setQueuedActionsCount] = useState(0);
  const [lastSyncTime, setLastSyncTime] = useState<Date | null>(null);
  const [isSyncing, setIsSyncing] = useState(false);
  const [syncProgress, setSyncProgress] = useState<SyncProgress | null>(null);

  // Synchronous re-entrancy guard for processQueue (issue #1767).
  //
  // processQueue is invoked from two places that can overlap: the
  // NetInfo.addEventListener callback (on every connectivity flip) and
  // syncData. The `isSyncing` React state is stale inside the long-lived
  // NetInfo closure (it captures the value from the render that registered
  // the listener), so it cannot guard against a concurrent run. A ref is read
  // synchronously and always current, so it is the real lock: set true at
  // entry, cleared in `finally`. Without it, two overlapping flushes each read
  // the same queue and double-dispatch every action.
  const syncingRef = useRef(false);

  // Get all queued actions
  const getQueuedActions = useCallback(async (): Promise<QueuedAction[]> => {
    try {
      const queue = await AsyncStorage.getItem(QUEUE_KEY);
      return queue ? JSON.parse(queue) : [];
    } catch (error) {
      console.error('Failed to get queued actions:', error);
      return [];
    }
  }, []);

  const loadLastSyncTime = useCallback(async () => {
    try {
      const timestamp = await AsyncStorage.getItem(LAST_SYNC_KEY);
      if (timestamp) {
        setLastSyncTime(new Date(Number.parseInt(timestamp, 10)));
      }
    } catch (error) {
      console.error('Failed to load last sync time:', error);
    }
  }, []);

  const loadQueueCount = useCallback(async () => {
    try {
      const queue = await getQueuedActions();
      setQueuedActionsCount(queue.length);
    } catch (error) {
      console.error('Failed to load queue count:', error);
    }
  }, [getQueuedActions]);

  // Execute a single queued action against the real backend.
  //
  // The queue stores `endpoint` as either an absolute URL or a path
  // beginning with `/`. For relative paths we prefix the configured API
  // base URL. The bearer token (if available) is read from SecureStore
  // at dispatch time so token rotations between enqueue and replay are
  // honored.
  const executeQueuedAction = useCallback(
    async (action: QueuedAction): Promise<Record<string, unknown> | null> => {
      const url = action.endpoint.startsWith('http')
        ? action.endpoint
        : `${getApiBaseUrl()}${action.endpoint}`;

      const accessToken = await SecureStore.getItemAsync(ACCESS_TOKEN_KEY);
      const headers: Record<string, string> = {
        'Content-Type': 'application/json',
      };
      if (accessToken) {
        headers.Authorization = `Bearer ${accessToken}`;
        // Tenant-scoped api-server routes (RlsConnection extractor on
        // /faults, /voting, /buildings, …) reject requests without
        // X-Tenant-ID. The tenant id lives in the JWT's `tenant_id`
        // claim — extract it so replayed offline actions hit the same
        // tenant the user was signed into when they enqueued.
        const tenantId = extractTenantId(accessToken);
        if (tenantId) {
          headers['X-Tenant-ID'] = tenantId;
        }
      }

      const response = await fetch(url, {
        method: action.method,
        headers,
        body: action.body !== undefined ? JSON.stringify(action.body) : undefined,
      });

      if (!response.ok) {
        // Treat 4xx as terminal (the request will never succeed even on
        // retry — bad payload, gone, unauthorized, …) so the queue drops
        // the action without burning the retry budget on hopeless replays.
        // 5xx and network failures bubble up as a normal error so the
        // outer retry loop handles them.
        const error = new Error(`HTTP ${response.status} on ${action.method} ${action.endpoint}`);
        if (response.status >= 400 && response.status < 500) {
          // Mark the error so processQueue can decide to drop instead of retry.
          (error as Error & { permanent?: boolean }).permanent = true;
        }
        throw error;
      }

      // Parse and return the response body so the caller can read a CREATE's
      // server-assigned id (issue #1767: offline-edit linkage remaps a queued
      // UPDATE/DELETE's temp id onto this id). 204 / empty / non-JSON bodies
      // return null — there is nothing to link against.
      if (response.status === 204) {
        return null;
      }
      try {
        return (await response.json()) as Record<string, unknown>;
      } catch {
        return null;
      }
    },
    []
  );

  // Process offline queue when back online
  const processQueue = useCallback(
    async (onProgress?: SyncProgressCallback): Promise<{ success: number; failed: number }> => {
      if (!isConnected || !isInternetReachable) {
        return { success: 0, failed: 0 };
      }

      // Re-entrancy guard (issue #1767): bail if a flush is already running so
      // the NetInfo-listener call and a syncData call can never overlap and
      // double-dispatch the same actions. Checked + set synchronously before
      // any await so there is no interleaving window.
      if (syncingRef.current) {
        return { success: 0, failed: 0 };
      }
      syncingRef.current = true;

      setIsSyncing(true);
      let success = 0;
      let failed = 0;

      try {
        const queue = await getQueuedActions();
        const total = queue.length;

        if (total === 0) {
          return { success: 0, failed: 0 };
        }

        // Snapshot the ids present when the flush started. Anything in the
        // queue afterwards that is NOT in this set was enqueued *during* the
        // flush and must be preserved verbatim (issue #1767 — the old code
        // overwrote the whole queue and clobbered those).
        const flushStartIds = new Set(queue.map((a) => a.id));

        // Initialize progress
        const initialProgress: SyncProgress = { total, current: 0, failed: 0, isComplete: false };
        setSyncProgress(initialProgress);
        onProgress?.(initialProgress);

        // Classify every action in the ORIGINAL queue order so we can rebuild
        // the carry-forward list as a strict-FIFO subsequence (issue #1767):
        //   - `succeededIds`: flushed OK (or terminally failed) → drop.
        //   - terminal failures (permanent 4xx or retries exhausted) → drop.
        //   - everything else → keep, in original order, so a later UPDATE can
        //     never overtake an earlier CREATE it depends on.
        const succeededIds = new Set<string>();
        const terminalFailureIds = new Set<string>();

        // Offline-edit linkage (issue #1767): when a CREATE that carries a temp
        // `localId` flushes and the server returns the real id, record the
        // mapping so subsequent queued actions targeting that temp id can be
        // rewritten before they are dispatched.
        const tempIdToServerId = new Map<string, string>();

        for (let i = 0; i < queue.length; i++) {
          const action = queue[i];

          // Apply any known temp→server id remap to this action before sending
          // it, so a dependent UPDATE/DELETE hits the freshly created resource
          // rather than the stale temp id.
          remapActionIds(action, tempIdToServerId);

          try {
            const result = await executeQueuedAction(action);
            success++;
            succeededIds.add(action.id);

            // If this CREATE represented a temp id and the server handed back a
            // real one, remember the mapping for the rest of the loop.
            if (action.localId && result) {
              const serverId = extractServerId(result);
              if (serverId) {
                tempIdToServerId.set(action.localId, serverId);
              }
            }
          } catch (error) {
            // Distinguish TERMINAL from TRANSIENT outcomes:
            //   - TERMINAL (4xx): executeQueuedAction marks the error
            //     `permanent`. Replaying will never succeed (bad payload, gone,
            //     forbidden, …), so the action is dropped — keeping it would
            //     block the queue forever on a request that can't win.
            //   - TRANSIENT (5xx / network): the server or link is temporarily
            //     unavailable. These are NEVER dropped, no matter how many times
            //     they have been retried — dropping them silently loses
            //     offline-created content (a fault the user reported offline,
            //     etc.). They are carried forward and re-attempted every
            //     reconnect until they succeed or turn terminal.
            const isPermanent = (error as { permanent?: boolean })?.permanent === true;
            // `retries` is still bumped for observability / UI (and so an
            // exponential-backoff scheduler could read it later), but it no
            // longer gates dropping a transient action — that was the data-loss
            // bug: a 5xx that persisted across 3 reconnects silently discarded
            // the queued item.
            action.retries++;
            if (isPermanent) {
              failed++;
              terminalFailureIds.add(action.id);
              console.error('Action permanently failed (4xx), dropping:', action);
              // A terminal failure can't block its successors — skip it and keep
              // draining the rest of the queue this cycle.
            } else {
              // Strict-FIFO head-of-line blocking (issue #1767): a *transient*
              // failure halts the cycle. The failed action and every action
              // after it are carried forward in their original order, so a later
              // UPDATE can never be dispatched ahead of an earlier CREATE that
              // is still being retried (which would hit a resource that doesn't
              // exist server-side yet). They retry next reconnect, in order.
              // Flag it so the UI can surface "waiting to sync" without implying
              // the item was lost.
              action.syncStatus = 'failed';
              const haltedProgress: SyncProgress = {
                total,
                current: i + 1,
                failed,
                isComplete: false,
              };
              setSyncProgress(haltedProgress);
              onProgress?.(haltedProgress);
              break;
            }
          }

          // Update progress after each item
          const currentProgress: SyncProgress = {
            total,
            current: i + 1,
            failed,
            isComplete: false,
          };
          setSyncProgress(currentProgress);
          onProgress?.(currentProgress);
        }

        // Re-read the queue so we pick up anything enqueued during the flush,
        // then persist atomically by merge rather than overwrite (issue #1767):
        //   newlyEnqueued = actions whose id was NOT present at flush start
        //   carriedForward = original actions that neither succeeded nor
        //                    terminally failed, in their ORIGINAL order (FIFO)
        // Order: carried-forward retries first (they were enqueued earliest),
        // then the during-flush arrivals, preserving overall FIFO.
        const latest = await getQueuedActions();
        const byId = new Map(queue.map((a) => [a.id, a] as const));
        const carriedForward = queue.filter(
          (a) => !succeededIds.has(a.id) && !terminalFailureIds.has(a.id)
        );
        const newlyEnqueued = latest
          .filter((a) => !flushStartIds.has(a.id))
          // A during-flush arrival may itself reference a temp id that was
          // resolved in this run (e.g. an edit queued right after the create);
          // apply the remap so its replay next cycle targets the server id.
          .map((a) => {
            remapActionIds(a, tempIdToServerId);
            return a;
          });
        // Prefer the original (post-retry-increment) objects for carried items
        // so their bumped `retries` count is what gets persisted.
        const merged = [...carriedForward.map((a) => byId.get(a.id) ?? a), ...newlyEnqueued];

        await AsyncStorage.setItem(QUEUE_KEY, JSON.stringify(merged));
        setQueuedActionsCount(merged.length);

        // Update last sync time
        const now = Date.now();
        await AsyncStorage.setItem(LAST_SYNC_KEY, now.toString());
        setLastSyncTime(new Date(now));

        // Final progress update
        const finalProgress: SyncProgress = { total, current: total, failed, isComplete: true };
        setSyncProgress(finalProgress);
        onProgress?.(finalProgress);

        return { success, failed };
      } catch (error) {
        console.error('Failed to process queue:', error);
        return { success, failed };
      } finally {
        syncingRef.current = false;
        setIsSyncing(false);
      }
    },
    [isConnected, isInternetReachable, getQueuedActions, executeQueuedAction]
  );

  // Monitor network status
  useEffect(() => {
    const unsubscribe = NetInfo.addEventListener((state: NetInfoState) => {
      setIsConnected(state.isConnected ?? false);
      setIsInternetReachable(state.isInternetReachable);
      setConnectionType(state.type);

      // Auto-sync when coming back online
      if (state.isConnected && state.isInternetReachable) {
        processQueue();
      }
    });

    // Initial network check
    NetInfo.fetch().then((state: NetInfoState) => {
      setIsConnected(state.isConnected ?? false);
      setIsInternetReachable(state.isInternetReachable);
      setConnectionType(state.type);
    });

    // Load last sync time
    loadLastSyncTime();

    // Load queue count
    loadQueueCount();

    return () => unsubscribe();
  }, [processQueue, loadLastSyncTime, loadQueueCount]);

  // Cache data locally
  const cacheData = useCallback(
    async <T>(key: string, data: T, expiresIn?: number): Promise<void> => {
      try {
        const cacheEntry = {
          data,
          timestamp: Date.now(),
          expiresAt: expiresIn ? Date.now() + expiresIn : null,
        };
        await AsyncStorage.setItem(`${CACHE_PREFIX}${key}`, JSON.stringify(cacheEntry));
      } catch (error) {
        console.error('Failed to cache data:', error);
      }
    },
    []
  );

  // Get cached data
  const getCachedData = useCallback(async <T>(key: string): Promise<T | null> => {
    try {
      const cached = await AsyncStorage.getItem(`${CACHE_PREFIX}${key}`);
      if (!cached) return null;

      const { data, expiresAt } = JSON.parse(cached);

      // Check if expired
      if (expiresAt && Date.now() > expiresAt) {
        await AsyncStorage.removeItem(`${CACHE_PREFIX}${key}`);
        return null;
      }

      return data as T;
    } catch (error) {
      console.error('Failed to get cached data:', error);
      return null;
    }
  }, []);

  // Clear cache
  const clearCache = useCallback(async (key?: string): Promise<void> => {
    try {
      if (key) {
        await AsyncStorage.removeItem(`${CACHE_PREFIX}${key}`);
      } else {
        // Clear all cache entries
        const keys = await AsyncStorage.getAllKeys();
        const cacheKeys = keys.filter((k: string) => k.startsWith(CACHE_PREFIX));
        await AsyncStorage.removeMany(cacheKeys);
      }
    } catch (error) {
      console.error('Failed to clear cache:', error);
    }
  }, []);

  // Add action to offline queue
  const addToQueue = useCallback(
    async (action: Omit<QueuedAction, 'id' | 'timestamp' | 'retries'>): Promise<void> => {
      try {
        const newAction: QueuedAction = {
          ...action,
          id: `${Date.now()}-${Math.random().toString(36).substring(2)}`,
          timestamp: Date.now(),
          retries: 0,
        };

        // Re-read the queue immediately before writing so an enqueue that races
        // a concurrent flush appends to the freshest queue rather than to a
        // stale snapshot read earlier (issue #1767). Combined with processQueue
        // merging by id, this keeps a fault enqueued mid-flush from being lost.
        const queue = await getQueuedActions();
        queue.push(newAction);
        await AsyncStorage.setItem(QUEUE_KEY, JSON.stringify(queue));
        setQueuedActionsCount(queue.length);
      } catch (error) {
        console.error('Failed to add to queue:', error);
      }
    },
    [getQueuedActions]
  );

  // Clear the offline queue
  const clearQueue = useCallback(async (): Promise<void> => {
    try {
      await AsyncStorage.removeItem(QUEUE_KEY);
      setQueuedActionsCount(0);
    } catch (error) {
      console.error('Failed to clear queue:', error);
    }
  }, []);

  // Sync all data
  const syncData = useCallback(
    async (onProgress?: SyncProgressCallback): Promise<void> => {
      if (!isConnected || !isInternetReachable) {
        return;
      }

      setIsSyncing(true);

      try {
        // Process offline queue first. Per-domain prefetching (announcements,
        // faults, votes) is intentionally left to each screen's own
        // useQuery + cacheData call so this hook stays endpoint-agnostic.
        await processQueue(onProgress);
      } catch (error) {
        console.error('Failed to sync data:', error);
      } finally {
        setIsSyncing(false);
      }
    },
    [isConnected, isInternetReachable, processQueue]
  );

  return {
    isConnected,
    isInternetReachable,
    connectionType,
    queuedActionsCount,
    lastSyncTime,
    cacheData,
    getCachedData,
    clearCache,
    addToQueue,
    getQueuedActions,
    processQueue,
    clearQueue,
    syncData,
    isSyncing,
    syncProgress,
  };
}
