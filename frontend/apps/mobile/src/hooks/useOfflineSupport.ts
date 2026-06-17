import AsyncStorage from '@react-native-async-storage/async-storage';
import NetInfo, { type NetInfoState } from '@react-native-community/netinfo';
import * as SecureStore from 'expo-secure-store';
import { useCallback, useEffect, useState } from 'react';
import { getApiBaseUrl } from '../config/api';

// Storage keys
const CACHE_PREFIX = 'ppt_cache_';
const QUEUE_KEY = 'ppt_offline_queue';
const LAST_SYNC_KEY = 'ppt_last_sync';
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
}

/**
 * Decode a JWT payload (no signature verification — that happens server-side)
 * and pull the `tenant_id` claim. Used so replayed offline actions can send
 * the X-Tenant-ID header that tenant-scoped routes require.
 */
function extractTenantIdFromJwt(token: string): string | null {
  try {
    const parts = token.split('.');
    if (parts.length < 2) return null;
    const padded = parts[1].replace(/-/g, '+').replace(/_/g, '/');
    const padding = '='.repeat((4 - (padded.length % 4)) % 4);
    const claims = JSON.parse(atob(padded + padding)) as Record<string, unknown>;
    const value = claims.tenant_id;
    return typeof value === 'string' ? value : null;
  } catch {
    return null;
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
  const executeQueuedAction = useCallback(async (action: QueuedAction): Promise<void> => {
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
      const tenantId = extractTenantIdFromJwt(accessToken);
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
  }, []);

  // Process offline queue when back online
  const processQueue = useCallback(
    async (onProgress?: SyncProgressCallback): Promise<{ success: number; failed: number }> => {
      if (!isConnected || !isInternetReachable) {
        return { success: 0, failed: 0 };
      }

      setIsSyncing(true);
      let success = 0;
      let failed = 0;

      try {
        const queue = await getQueuedActions();
        const total = queue.length;

        if (total === 0) {
          return { success: 0, failed: 0 };
        }

        // Initialize progress
        const initialProgress: SyncProgress = { total, current: 0, failed: 0, isComplete: false };
        setSyncProgress(initialProgress);
        onProgress?.(initialProgress);

        const remainingActions: QueuedAction[] = [];

        for (let i = 0; i < queue.length; i++) {
          const action = queue[i];
          try {
            await executeQueuedAction(action);
            success++;
          } catch (error) {
            // 4xx responses are marked `permanent` by executeQueuedAction
            // and dropped immediately — replaying them won't help.
            // Everything else (5xx, network) goes through the retry budget.
            const isPermanent = (error as { permanent?: boolean })?.permanent === true;
            action.retries++;
            if (!isPermanent && action.retries < 3) {
              remainingActions.push(action);
            } else {
              failed++;
              console.error('Action failed after max retries:', action);
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

        // Update queue with remaining actions
        await AsyncStorage.setItem(QUEUE_KEY, JSON.stringify(remainingActions));
        setQueuedActionsCount(remainingActions.length);

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
        const queue = await getQueuedActions();

        const newAction: QueuedAction = {
          ...action,
          id: `${Date.now()}-${Math.random().toString(36).substring(2)}`,
          timestamp: Date.now(),
          retries: 0,
        };

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
