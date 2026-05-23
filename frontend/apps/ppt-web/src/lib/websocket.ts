/**
 * WebSocket Service for ppt-web (Story 79.4)
 *
 * Provides WebSocket connection management with:
 * - Authentication via JWT token
 * - Automatic reconnection with exponential backoff
 * - Connection state machine
 * - Heartbeat/ping-pong mechanism
 * - Event emitter pattern for message handling
 */

/**
 * WebSocket message format as defined in story spec.
 */
export interface WebSocketMessage {
  type: string;
  payload: unknown;
  timestamp: string;
  requestId?: string;
}

/**
 * WebSocket event types.
 */
export type WebSocketEventType =
  | 'message:new'
  | 'notification:announcement'
  | 'notification:fault'
  | 'notification:vote'
  | 'entity:updated'
  | 'entity:created'
  | 'entity:deleted'
  | 'connection:authenticated'
  | 'connection:error'
  | 'connection:max-retries-exceeded';

/**
 * Default maximum number of reconnect attempts before giving up.
 * After this many failed attempts in a row, the client stops trying
 * and emits a `connection:max-retries-exceeded` event so callers can
 * surface a UI-level "offline" state instead of silently retrying forever.
 */
const DEFAULT_MAX_RECONNECT_ATTEMPTS = 10;

/**
 * Connection states for the WebSocket.
 */
export type ConnectionState = 'connecting' | 'connected' | 'disconnected' | 'error';

/**
 * Event handler type for WebSocket messages.
 */
export type MessageHandler = (message: WebSocketMessage) => void;

/**
 * Event handler type for connection state changes.
 */
export type ConnectionStateHandler = (state: ConnectionState, error?: Error) => void;

/**
 * Configuration options for WebSocketService.
 */
export interface WebSocketServiceConfig {
  /**
   * WebSocket server URL.
   *
   * Defaults to:
   * 1. `config.url` if provided,
   * 2. `VITE_WS_URL` env variable if set,
   * 3. `ws://localhost:8080/ws` in dev (`import.meta.env.DEV`),
   * 4. `wss://${location.host}/ws` in production (no cleartext fallback).
   */
  url?: string;

  /**
   * Function to get the current auth token.
   */
  getToken: () => string | null;

  /**
   * Minimum reconnection delay in milliseconds.
   * @default 1000
   */
  minReconnectDelay?: number;

  /**
   * Maximum reconnection delay in milliseconds.
   * @default 30000
   */
  maxReconnectDelay?: number;

  /**
   * Heartbeat interval in milliseconds.
   * @default 30000
   */
  heartbeatInterval?: number;

  /**
   * Pong timeout in milliseconds (how long to wait for pong after ping).
   * @default 10000
   */
  pongTimeout?: number;

  /**
   * Maximum number of reconnect attempts before giving up.
   * After exceeding this, the client emits a `connection:max-retries-exceeded`
   * message and stops retrying. Call `connect()` again to resume.
   * @default 10
   */
  maxReconnectAttempts?: number;
}

/**
 * Compute the default WebSocket URL.
 *
 * In dev we keep the explicit cleartext `ws://localhost:8080/ws` so the
 * local Vite stack works without TLS. In production we never default to
 * cleartext: we derive `wss://${location.host}/ws` so the WS scheme
 * tracks the page scheme and the deploy host. Callers can still override
 * via the `url` config option or the `VITE_WS_URL` env variable.
 */
function defaultWebSocketUrl(): string {
  if (import.meta.env.DEV) {
    return 'ws://localhost:8080/ws';
  }

  // SSR / non-browser safety net — should not happen for ppt-web (SPA),
  // but keeps the function total.
  if (typeof window === 'undefined' || !window.location) {
    return 'wss://localhost/ws';
  }

  return `wss://${window.location.host}/ws`;
}

/**
 * WebSocket service that manages connection, reconnection, and message handling.
 */
export class WebSocketService {
  private socket: WebSocket | null = null;
  private connectionState: ConnectionState = 'disconnected';
  private lastError: Error | null = null;

  // Configuration
  private readonly url: string;
  private readonly getToken: () => string | null;
  private readonly minReconnectDelay: number;
  private readonly maxReconnectDelay: number;
  private readonly heartbeatInterval: number;
  private readonly pongTimeout: number;
  private readonly maxReconnectAttempts: number;

  // Reconnection state
  private reconnectAttempts = 0;
  private reconnectTimeoutId: ReturnType<typeof setTimeout> | null = null;
  private shouldReconnect = true;

  // Heartbeat state
  private heartbeatIntervalId: ReturnType<typeof setInterval> | null = null;
  private pongTimeoutId: ReturnType<typeof setTimeout> | null = null;
  private awaitingPong = false;

  // Event handlers
  private messageHandlers: Map<string, Set<MessageHandler>> = new Map();
  private connectionStateHandlers: Set<ConnectionStateHandler> = new Set();

  // Track last event timestamp for gap detection
  private lastEventTimestamp: string | null = null;

  constructor(config: WebSocketServiceConfig) {
    this.url = config.url ?? import.meta.env.VITE_WS_URL ?? defaultWebSocketUrl();
    this.getToken = config.getToken;
    this.minReconnectDelay = config.minReconnectDelay ?? 1000;
    this.maxReconnectDelay = config.maxReconnectDelay ?? 30000;
    this.heartbeatInterval = config.heartbeatInterval ?? 30000;
    this.pongTimeout = config.pongTimeout ?? 10000;
    this.maxReconnectAttempts = config.maxReconnectAttempts ?? DEFAULT_MAX_RECONNECT_ATTEMPTS;
  }

  /**
   * Get the current connection state.
   */
  getConnectionState(): ConnectionState {
    return this.connectionState;
  }

  /**
   * Get the last error, if any.
   */
  getLastError(): Error | null {
    return this.lastError;
  }

  /**
   * Get the last event timestamp for gap detection.
   */
  getLastEventTimestamp(): string | null {
    return this.lastEventTimestamp;
  }

  /**
   * Check if the connection is currently open.
   */
  isConnected(): boolean {
    return this.connectionState === 'connected' && this.socket?.readyState === WebSocket.OPEN;
  }

  /**
   * Connect to the WebSocket server.
   */
  connect(): void {
    if (this.socket && this.socket.readyState === WebSocket.OPEN) {
      return; // Already connected
    }

    const token = this.getToken();
    if (!token) {
      this.setConnectionState('error', new Error('No auth token available'));
      return;
    }

    this.shouldReconnect = true;
    this.clearReconnectTimeout();

    // P0-11: pass the JWT through the WebSocket subprotocol header,
    // not as a query parameter. The URL form (?token=...) ended up in
    // browser history, reverse-proxy access logs, DevTools network
    // tabs, and any SIEM that ships logs upstream — every one of
    // those is a token-exfiltration surface. The subprotocol value
    // is sent in `Sec-WebSocket-Protocol` and never logged by
    // standard HTTP middleware.
    //
    // Wire format: two subprotocols, `bearer.<JWT>` for the auth
    // payload and a `ppt.v1` discriminator so the server can negotiate
    // future protocol versions. The server's WebSocketUpgrade handler
    // must accept one of these subprotocols in its response — without
    // that echo, browsers reject the handshake.
    this.setConnectionState('connecting');

    try {
      this.socket = new WebSocket(this.url, [`bearer.${token}`, 'ppt.v1']);
      this.setupSocketHandlers();
    } catch (error) {
      const err = error instanceof Error ? error : new Error('Failed to create WebSocket');
      this.setConnectionState('error', err);
      this.scheduleReconnect();
    }
  }

  /**
   * Disconnect from the WebSocket server.
   */
  disconnect(): void {
    this.shouldReconnect = false;
    this.clearReconnectTimeout();
    this.stopHeartbeat();

    if (this.socket) {
      this.socket.close(1000, 'Client disconnecting');
      this.socket = null;
    }

    this.setConnectionState('disconnected');
  }

  /**
   * Send a message through the WebSocket.
   */
  send(message: WebSocketMessage): boolean {
    if (!this.isConnected()) {
      console.warn('[WebSocket] Cannot send message: not connected');
      return false;
    }

    try {
      this.socket!.send(JSON.stringify(message));
      return true;
    } catch (error) {
      console.error('[WebSocket] Failed to send message:', error);
      return false;
    }
  }

  /**
   * Subscribe to messages of a specific type.
   *
   * @param eventType - The event type to subscribe to, or '*' for all events.
   * @param handler - The handler function to call when a message is received.
   * @returns An unsubscribe function.
   */
  subscribe(eventType: string, handler: MessageHandler): () => void {
    if (!this.messageHandlers.has(eventType)) {
      this.messageHandlers.set(eventType, new Set());
    }

    this.messageHandlers.get(eventType)!.add(handler);

    return () => {
      this.messageHandlers.get(eventType)?.delete(handler);
    };
  }

  /**
   * Subscribe to connection state changes.
   *
   * @param handler - The handler function to call when connection state changes.
   * @returns An unsubscribe function.
   */
  onConnectionStateChange(handler: ConnectionStateHandler): () => void {
    this.connectionStateHandlers.add(handler);

    // Immediately call with current state
    handler(this.connectionState, this.lastError ?? undefined);

    return () => {
      this.connectionStateHandlers.delete(handler);
    };
  }

  /**
   * Reset reconnection attempts (call after successful operations).
   */
  resetReconnectAttempts(): void {
    this.reconnectAttempts = 0;
  }

  // Private methods

  private setupSocketHandlers(): void {
    if (!this.socket) return;

    this.socket.onopen = () => {
      this.reconnectAttempts = 0;
      this.lastError = null;
      this.setConnectionState('connected');
      this.startHeartbeat();
    };

    this.socket.onclose = (event) => {
      this.stopHeartbeat();

      if (event.wasClean) {
        this.setConnectionState('disconnected');
      } else {
        this.setConnectionState(
          'error',
          new Error(`Connection closed unexpectedly: ${event.code}`)
        );
      }

      if (this.shouldReconnect) {
        this.scheduleReconnect();
      }
    };

    this.socket.onerror = () => {
      // The error event doesn't provide useful info; onclose will follow
      this.lastError = new Error('WebSocket error occurred');
    };

    this.socket.onmessage = (event) => {
      this.handleMessage(event);
    };
  }

  private handleMessage(event: MessageEvent): void {
    try {
      const data = JSON.parse(event.data as string);

      // Handle pong response
      if (data.type === 'pong') {
        this.handlePong();
        return;
      }

      const message = data as WebSocketMessage;

      // Track last event timestamp
      if (message.timestamp) {
        this.lastEventTimestamp = message.timestamp;
      }

      // Notify all handlers for this specific event type
      const typeHandlers = this.messageHandlers.get(message.type);
      if (typeHandlers) {
        for (const handler of typeHandlers) {
          try {
            handler(message);
          } catch (handlerError) {
            console.error(`[WebSocket] Handler error for ${message.type}:`, handlerError);
          }
        }
      }

      // Notify wildcard handlers
      const wildcardHandlers = this.messageHandlers.get('*');
      if (wildcardHandlers) {
        for (const handler of wildcardHandlers) {
          try {
            handler(message);
          } catch (handlerError) {
            console.error('[WebSocket] Wildcard handler error:', handlerError);
          }
        }
      }
    } catch (error) {
      console.error('[WebSocket] Failed to parse message:', error);
    }
  }

  private setConnectionState(state: ConnectionState, error?: Error): void {
    this.connectionState = state;

    if (error) {
      this.lastError = error;
    }

    for (const handler of this.connectionStateHandlers) {
      try {
        handler(state, error);
      } catch (handlerError) {
        console.error('[WebSocket] Connection state handler error:', handlerError);
      }
    }
  }

  private scheduleReconnect(): void {
    if (!this.shouldReconnect) return;

    this.clearReconnectTimeout();

    // Stop retrying once we hit the cap. Without this the client would
    // hammer a dead endpoint forever, spamming console + server logs and
    // acting as a small DoS amplifier in dev.
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      this.shouldReconnect = false;
      console.warn(
        `[WebSocket] Giving up after ${this.reconnectAttempts} reconnect attempts. ` +
          'Call connect() to resume.'
      );
      this.notifyMaxRetriesExceeded();
      this.setConnectionState(
        'error',
        new Error(`Max reconnect attempts (${this.maxReconnectAttempts}) exceeded`)
      );
      return;
    }

    // Exponential backoff capped at maxReconnectDelay, plus random jitter
    // in [0.5, 1.0) to avoid a thundering-herd on shared outages.
    const baseDelay = Math.min(
      this.minReconnectDelay * 2 ** this.reconnectAttempts,
      this.maxReconnectDelay
    );
    const delay = baseDelay * (0.5 + Math.random() * 0.5);

    this.reconnectAttempts++;

    this.reconnectTimeoutId = setTimeout(() => {
      this.connect();
    }, delay);
  }

  private notifyMaxRetriesExceeded(): void {
    const message: WebSocketMessage = {
      type: 'connection:max-retries-exceeded',
      payload: { attempts: this.reconnectAttempts },
      timestamp: new Date().toISOString(),
    };

    const typeHandlers = this.messageHandlers.get(message.type);
    if (typeHandlers) {
      for (const handler of typeHandlers) {
        try {
          handler(message);
        } catch (handlerError) {
          console.error('[WebSocket] max-retries handler error:', handlerError);
        }
      }
    }

    const wildcardHandlers = this.messageHandlers.get('*');
    if (wildcardHandlers) {
      for (const handler of wildcardHandlers) {
        try {
          handler(message);
        } catch (handlerError) {
          console.error('[WebSocket] Wildcard handler error:', handlerError);
        }
      }
    }
  }

  private clearReconnectTimeout(): void {
    if (this.reconnectTimeoutId) {
      clearTimeout(this.reconnectTimeoutId);
      this.reconnectTimeoutId = null;
    }
  }

  private startHeartbeat(): void {
    this.stopHeartbeat();

    this.heartbeatIntervalId = setInterval(() => {
      this.sendPing();
    }, this.heartbeatInterval);
  }

  private stopHeartbeat(): void {
    if (this.heartbeatIntervalId) {
      clearInterval(this.heartbeatIntervalId);
      this.heartbeatIntervalId = null;
    }

    if (this.pongTimeoutId) {
      clearTimeout(this.pongTimeoutId);
      this.pongTimeoutId = null;
    }

    this.awaitingPong = false;
  }

  private sendPing(): void {
    if (!this.isConnected() || this.awaitingPong) {
      return;
    }

    const pingMessage: WebSocketMessage = {
      type: 'ping',
      payload: null,
      timestamp: new Date().toISOString(),
    };

    if (this.send(pingMessage)) {
      this.awaitingPong = true;

      this.pongTimeoutId = setTimeout(() => {
        if (this.awaitingPong) {
          console.warn('[WebSocket] Pong timeout - closing connection');
          this.socket?.close(4000, 'Pong timeout');
        }
      }, this.pongTimeout);
    }
  }

  private handlePong(): void {
    this.awaitingPong = false;

    if (this.pongTimeoutId) {
      clearTimeout(this.pongTimeoutId);
      this.pongTimeoutId = null;
    }
  }
}

/**
 * Create a WebSocket service instance.
 *
 * @param config - Configuration options.
 * @returns A new WebSocketService instance.
 */
export function createWebSocketService(config: WebSocketServiceConfig): WebSocketService {
  return new WebSocketService(config);
}
