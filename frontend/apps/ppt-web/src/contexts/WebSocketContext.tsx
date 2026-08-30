/**
 * WebSocket Context and Provider for ppt-web (Story 79.4)
 *
 * Provides WebSocket connectivity to the application with:
 * - Connection state management
 * - Subscribe/unsubscribe methods for events
 * - Integration with authentication for token
 * - Query invalidation on server push
 */

import type { ReactNode } from 'react';
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import { useNetworkStatusEffect } from '../hooks/useNetworkStatus';
import {
  type ConnectionState,
  type MessageHandler,
  type WebSocketMessage,
  WebSocketService,
} from '../lib/websocket';

/**
 * Query key mapping for realtime events emitted by the api-server.
 *
 * The api-server pushes canonical `domain.action` event envelopes on the
 * per-user `notifications:{user_id}` channel (see `ws_notifications.rs` and its
 * publishers). It never emits the legacy `entity:*` names this map used to key
 * on, so before this mapping was corrected every realtime frame missed the
 * lookup and no TanStack Query cache was ever invalidated (100% dead sync).
 *
 * Keys are the exact `event` strings the server sends; values are the
 * first-segment query roots (see `lib/queryKeys.ts`) to invalidate. The
 * polymorphic `notification.created` event is handled separately — its
 * affected entity list is carried in `payload.category`, see
 * {@link categoryToQueryKeys}.
 */
export const eventToQueryKeys: Record<string, string[]> = {
  // Direct message sent to the user (`messaging.rs::dispatch_new_message_event`).
  'message.created': ['messages'],
  // Notification-preference toggled (`notification_preferences.rs`).
  'preference.updated': ['notifications'],
};

/**
 * Map a `notification.created` payload `category` to the query roots to
 * invalidate alongside the `notifications` root.
 *
 * `notification.created` (`notification_pipeline.rs::DbInAppAdapter::send`) is
 * polymorphic: a single event type covers every domain, and its
 * `payload.category` (serialized snake_case, see `NotificationCategory`) pins
 * which entity list just changed. A new announcement notification, for example,
 * should refresh both the notification list and the announcements list.
 */
export const categoryToQueryKeys: Record<string, string[]> = {
  announcements: ['announcements'],
  faults: ['faults'],
  votes: ['votes'],
  messages: ['messages'],
  documents: ['documents'],
  financial: ['financial'],
};

/**
 * Auth context interface that WebSocketContext expects.
 * This allows flexible integration with different auth implementations.
 */
export interface AuthContextForWebSocket {
  accessToken: string | null;
  isAuthenticated: boolean;
}

/**
 * Value provided by the WebSocket context.
 */
export interface WebSocketContextValue {
  /**
   * Whether the WebSocket is currently connected.
   */
  isConnected: boolean;

  /**
   * Whether the WebSocket is currently connecting.
   */
  isConnecting: boolean;

  /**
   * The current connection state.
   */
  connectionState: ConnectionState;

  /**
   * The last connection error, if any.
   */
  error: Error | null;

  /**
   * Whether the client has exhausted its reconnect-attempt budget and given up
   * retrying for now. Cleared automatically once a connection is
   * re-established (e.g. after the browser regains connectivity, or a manual
   * {@link reconnect}). UIs can use this to surface an explicit "reconnect"
   * affordance instead of a transient error.
   */
  maxRetriesExceeded: boolean;

  /**
   * Subscribe to WebSocket events of a specific type.
   *
   * @param eventType - The event type to subscribe to, or '*' for all events.
   * @param handler - The handler function.
   * @returns An unsubscribe function.
   */
  subscribe: (eventType: string, handler: MessageHandler) => () => void;

  /**
   * Send a message through the WebSocket.
   *
   * @param message - The message to send.
   * @returns true if the message was sent successfully.
   */
  send: (message: WebSocketMessage) => boolean;

  /**
   * Get the last event timestamp for gap detection.
   */
  getLastEventTimestamp: () => string | null;

  /**
   * Manually reconnect to the WebSocket server.
   */
  reconnect: () => void;
}

const WebSocketContext = createContext<WebSocketContextValue | null>(null);

/**
 * Hook to access the WebSocket context.
 *
 * @throws Error if used outside of WebSocketProvider.
 */
export function useWebSocketContext(): WebSocketContextValue {
  const context = useContext(WebSocketContext);

  if (!context) {
    throw new Error('useWebSocketContext must be used within a WebSocketProvider');
  }

  return context;
}

/**
 * Props for WebSocketProvider.
 */
export interface WebSocketProviderProps {
  children: ReactNode;

  /**
   * Auth context value. Provide accessToken and isAuthenticated.
   */
  auth: AuthContextForWebSocket;

  /**
   * WebSocket server URL.
   *
   * Defaults to (in order): the `VITE_WS_URL` env variable; then
   * `ws://localhost:8080/ws` in dev (`import.meta.env.DEV`); then
   * `wss://${location.host}/ws` in production. We never default to
   * cleartext `ws://` outside dev to avoid leaking the bearer token
   * in the URL over an unencrypted channel.
   */
  wsUrl?: string;

  /**
   * Optional callback when an entity event is received.
   * Can be used to invalidate queries.
   */
  onEntityEvent?: (eventType: string, queryKeys: string[], message: WebSocketMessage) => void;

  /**
   * Optional callback when the connection is established.
   */
  onConnected?: () => void;

  /**
   * Optional callback when the connection is lost.
   */
  onDisconnected?: () => void;

  /**
   * Optional callback when reconnection occurs.
   */
  onReconnected?: () => void;
}

/**
 * WebSocket provider component.
 *
 * Wrap your app with this to enable WebSocket connectivity.
 * Requires auth context to be provided.
 */
export function WebSocketProvider({
  children,
  auth,
  wsUrl,
  onEntityEvent,
  onConnected,
  onDisconnected,
  onReconnected,
}: WebSocketProviderProps) {
  const [connectionState, setConnectionState] = useState<ConnectionState>('disconnected');
  const [error, setError] = useState<Error | null>(null);
  const [maxRetriesExceeded, setMaxRetriesExceeded] = useState(false);
  const serviceRef = useRef<WebSocketService | null>(null);
  const wasConnectedRef = useRef(false);
  const previousStateRef = useRef<ConnectionState>('disconnected');

  // Store callbacks in refs to avoid re-creating the service on callback changes
  const onConnectedRef = useRef(onConnected);
  const onDisconnectedRef = useRef(onDisconnected);
  const onReconnectedRef = useRef(onReconnected);

  // Update refs when callbacks change
  useEffect(() => {
    onConnectedRef.current = onConnected;
    onDisconnectedRef.current = onDisconnected;
    onReconnectedRef.current = onReconnected;
  }, [onConnected, onDisconnected, onReconnected]);

  // Store auth token getter in ref
  const authRef = useRef(auth);
  useEffect(() => {
    authRef.current = auth;
  }, [auth]);

  // Store wsUrl in ref
  const wsUrlRef = useRef(wsUrl);
  useEffect(() => {
    wsUrlRef.current = wsUrl;
  }, [wsUrl]);

  // Create WebSocket service on mount
  useEffect(() => {
    const service = new WebSocketService({
      url: wsUrlRef.current,
      getToken: () => authRef.current.accessToken,
    });

    serviceRef.current = service;

    // Subscribe to connection state changes
    const unsubscribe = service.onConnectionStateChange((state, err) => {
      const previousState = previousStateRef.current;
      previousStateRef.current = state;
      setConnectionState(state);
      setError(err ?? null);

      if (state === 'connected') {
        // A live connection clears any prior give-up state.
        setMaxRetriesExceeded(false);
        if (wasConnectedRef.current) {
          // This is a reconnection
          onReconnectedRef.current?.();
        } else {
          onConnectedRef.current?.();
        }
        wasConnectedRef.current = true;
      } else if (state === 'disconnected' || state === 'error') {
        if (wasConnectedRef.current && previousState === 'connected') {
          onDisconnectedRef.current?.();
        }
      }
    });

    // Surface the terminal give-up state so the UI can offer an explicit
    // reconnect affordance rather than silently losing realtime for the
    // session. Cleared again on the next successful 'connected' transition.
    const unsubscribeMaxRetries = service.subscribe('connection:max-retries-exceeded', () => {
      setMaxRetriesExceeded(true);
    });

    return () => {
      unsubscribe();
      unsubscribeMaxRetries();
      service.disconnect();
    };
  }, []);

  // Resume the socket when the browser regains connectivity. The service's
  // exponential-backoff budget (default ~3 min) can be exhausted by a longer
  // outage — sleep/resume, a VPN drop, or a server redeploy — after which the
  // service gives up permanently for the session and nothing re-arms it.
  // A regained-connectivity signal is exactly when to retry: connect() resets
  // the reconnect budget and re-establishes the socket.
  useNetworkStatusEffect(
    useCallback(() => {
      const service = serviceRef.current;
      if (!service) return;
      // Only resume while still authenticated — otherwise connect() would just
      // error out on a missing token.
      if (authRef.current.isAuthenticated && authRef.current.accessToken) {
        service.connect();
      }
    }, [])
  );

  // Handle auth changes - connect when authenticated, disconnect when not
  useEffect(() => {
    const service = serviceRef.current;
    if (!service) return;

    if (auth.isAuthenticated && auth.accessToken) {
      service.connect();
    } else {
      service.disconnect();
      wasConnectedRef.current = false;
    }
  }, [auth.isAuthenticated, auth.accessToken]);

  // Set up entity event handler for query invalidation
  useEffect(() => {
    const service = serviceRef.current;
    if (!service || !onEntityEvent) return;

    const unsubscribers: (() => void)[] = [];

    // Subscribe to the fixed `domain.action` events whose query roots are known
    // up front.
    for (const [eventType, queryKeys] of Object.entries(eventToQueryKeys)) {
      const unsubscribe = service.subscribe(eventType, (message) => {
        onEntityEvent(eventType, queryKeys, message);
      });
      unsubscribers.push(unsubscribe);
    }

    // `notification.created` is polymorphic: one event type spans every domain,
    // with `payload.category` naming the affected entity list. Refresh the
    // `notifications` root (list + unread count) plus that entity's list.
    const unsubscribeNotification = service.subscribe('notification.created', (message) => {
      const payload = message.payload as { category?: string } | null;
      const category = payload?.category;
      const entityKeys = category ? categoryToQueryKeys[category] : undefined;
      const queryKeys = entityKeys ? ['notifications', ...entityKeys] : ['notifications'];
      onEntityEvent('notification.created', queryKeys, message);
    });
    unsubscribers.push(unsubscribeNotification);

    return () => {
      for (const unsubscribe of unsubscribers) {
        unsubscribe();
      }
    };
  }, [onEntityEvent]);

  const subscribe = useCallback((eventType: string, handler: MessageHandler): (() => void) => {
    const service = serviceRef.current;
    if (!service) {
      return () => {
        // noop
      };
    }

    return service.subscribe(eventType, handler);
  }, []);

  const send = useCallback((message: WebSocketMessage): boolean => {
    const service = serviceRef.current;
    if (!service) {
      return false;
    }

    return service.send(message);
  }, []);

  const getLastEventTimestamp = useCallback((): string | null => {
    return serviceRef.current?.getLastEventTimestamp() ?? null;
  }, []);

  const reconnect = useCallback((): void => {
    const service = serviceRef.current;
    if (!service) return;

    service.disconnect();
    service.connect();
  }, []);

  const value = useMemo<WebSocketContextValue>(
    () => ({
      isConnected: connectionState === 'connected',
      isConnecting: connectionState === 'connecting',
      connectionState,
      error,
      maxRetriesExceeded,
      subscribe,
      send,
      getLastEventTimestamp,
      reconnect,
    }),
    [connectionState, error, maxRetriesExceeded, subscribe, send, getLastEventTimestamp, reconnect]
  );

  return <WebSocketContext.Provider value={value}>{children}</WebSocketContext.Provider>;
}

WebSocketProvider.displayName = 'WebSocketProvider';
