/**
 * Network Status Hook for ppt-web.
 *
 * Provides network connectivity status and reconnection detection.
 * Uses browser online/offline events for real-time updates.
 */

import { useCallback, useEffect, useRef, useState, useSyncExternalStore } from 'react';

/**
 * Network status state.
 */
export interface NetworkStatus {
  /** Whether the browser is currently online */
  isOnline: boolean;
  /** Whether the connection was recently restored (for showing reconnection messages) */
  wasOffline: boolean;
  /** Timestamp of last online event */
  lastOnlineAt: Date | null;
  /** Timestamp of last offline event */
  lastOfflineAt: Date | null;
}

/**
 * Get initial network status from navigator.
 */
function getInitialOnlineStatus(): boolean {
  return typeof navigator !== 'undefined' ? navigator.onLine : true;
}

/**
 * Subscribe to network status changes.
 *
 * Guarded for non-browser environments (SSR / web-worker / unsupported):
 * when `window` is unavailable there are no online/offline events to listen
 * for, so we return a no-op unsubscribe instead of crashing.
 */
function subscribeToNetworkStatus(callback: () => void): () => void {
  if (typeof window === 'undefined') {
    return () => {};
  }

  window.addEventListener('online', callback);
  window.addEventListener('offline', callback);

  return () => {
    window.removeEventListener('online', callback);
    window.removeEventListener('offline', callback);
  };
}

/**
 * Get current online status snapshot.
 *
 * Guarded for environments without `navigator` (SSR / non-browser /
 * unsupported): falls back to the optimistic "online" default rather than
 * throwing, matching `getInitialOnlineStatus` / `isOnline`.
 */
function getOnlineSnapshot(): boolean {
  return getInitialOnlineStatus();
}

/**
 * Server snapshot for SSR (always online).
 */
function getServerSnapshot(): boolean {
  return true;
}

/**
 * Hook to track network connectivity status.
 *
 * @returns NetworkStatus object with current online state and history
 *
 * @example
 * ```tsx
 * function MyComponent() {
 *   const { isOnline, wasOffline } = useNetworkStatus();
 *
 *   if (!isOnline) {
 *     return <OfflineIndicator />;
 *   }
 *
 *   if (wasOffline) {
 *     return <ReconnectedBanner />;
 *   }
 *
 *   return <MainContent />;
 * }
 * ```
 */
export function useNetworkStatus(): NetworkStatus {
  // Use useSyncExternalStore for reliable online/offline tracking
  const isOnline = useSyncExternalStore(
    subscribeToNetworkStatus,
    getOnlineSnapshot,
    getServerSnapshot
  );

  const [wasOffline, setWasOffline] = useState(false);
  const [lastOnlineAt, setLastOnlineAt] = useState<Date | null>(null);
  const [lastOfflineAt, setLastOfflineAt] = useState<Date | null>(null);

  // Track previous online state
  const wasOnlineRef = useRef(getInitialOnlineStatus());
  const wasOfflineTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    // Detect transition from offline to online
    if (isOnline && !wasOnlineRef.current) {
      setLastOnlineAt(new Date());
      setWasOffline(true);

      // Clear the "was offline" flag after 5 seconds
      wasOfflineTimeoutRef.current = setTimeout(() => {
        setWasOffline(false);
      }, 5000);
    }

    // Detect transition from online to offline
    if (!isOnline && wasOnlineRef.current) {
      setLastOfflineAt(new Date());
      setWasOffline(false);

      // Clear any pending timeout
      if (wasOfflineTimeoutRef.current) {
        clearTimeout(wasOfflineTimeoutRef.current);
        wasOfflineTimeoutRef.current = null;
      }
    }

    wasOnlineRef.current = isOnline;
  }, [isOnline]);

  // Cleanup timeout on unmount
  useEffect(() => {
    return () => {
      if (wasOfflineTimeoutRef.current) {
        clearTimeout(wasOfflineTimeoutRef.current);
      }
    };
  }, []);

  return {
    isOnline,
    wasOffline,
    lastOnlineAt,
    lastOfflineAt,
  };
}

/**
 * Callback type for network status changes.
 */
export type NetworkStatusCallback = (status: { isOnline: boolean }) => void;

/**
 * Hook to execute a callback when network status changes.
 *
 * @param onOnline - Callback when going online
 * @param onOffline - Callback when going offline
 *
 * @example
 * ```tsx
 * useNetworkStatusEffect(
 *   () => {
 *     // Refetch data when coming back online
 *     queryClient.refetchQueries();
 *   },
 *   () => {
 *     // Show offline notification
 *     showToast({ title: 'You are offline', type: 'warning' });
 *   }
 * );
 * ```
 */
export function useNetworkStatusEffect(onOnline?: () => void, onOffline?: () => void): void {
  const onOnlineRef = useRef(onOnline);
  const onOfflineRef = useRef(onOffline);

  // Keep refs up to date
  useEffect(() => {
    onOnlineRef.current = onOnline;
    onOfflineRef.current = onOffline;
  });

  const handleOnline = useCallback(() => {
    onOnlineRef.current?.();
  }, []);

  const handleOffline = useCallback(() => {
    onOfflineRef.current?.();
  }, []);

  useEffect(() => {
    if (typeof window === 'undefined') {
      return;
    }

    window.addEventListener('online', handleOnline);
    window.addEventListener('offline', handleOffline);

    return () => {
      window.removeEventListener('online', handleOnline);
      window.removeEventListener('offline', handleOffline);
    };
  }, [handleOnline, handleOffline]);
}

/**
 * Check if the browser is currently online.
 *
 * This is a simple utility function for non-reactive checks.
 * For reactive updates, use the useNetworkStatus hook instead.
 */
export function isOnline(): boolean {
  return typeof navigator !== 'undefined' ? navigator.onLine : true;
}
