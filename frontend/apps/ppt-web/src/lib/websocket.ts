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
 * Canonical server path for the realtime notification WebSocket.
 *
 * The api-server mounts the handler at
 * `/api/v1/users/me/notifications/ws` (see
 * `backend/servers/api-server/src/lib.rs` nest + `ws_notifications::router`).
 * The client must connect to this exact path — a bare `/ws` does not exist
 * and yields a 404 on upgrade (#775, #819).
 */
export const WS_NOTIFICATIONS_PATH = '/api/v1/users/me/notifications/ws';

/**
 * WebSocket message format used internally by the client.
 *
 * The server speaks the canonical `{ event, payload }` envelope (see
 * `WsEvent` in `ws_notifications.rs`). {@link parseServerMessage} normalises
 * that envelope into this shape, mapping the server's `event` field onto
 * `type` so subscribers can key off the real event names
 * (`notification.created`, `preference.updated`, …).
 */
export interface WebSocketMessage {
  type: string;
  payload: unknown;
  timestamp: string;
  requestId?: string;
}

/**
 * Wire envelope emitted by the api-server WebSocket handler.
 * Mirrors `WsEvent` (`{ event, payload }`) in `ws_notifications.rs`.
 */
export interface ServerWsEnvelope {
  event: string;
  payload: unknown;
}

/**
 * Normalise one decoded server frame into the client's internal
 * {@link WebSocketMessage} shape.
 *
 * Returns `null` for frames that are not a recognised event envelope (e.g. a
 * bare `pong` heartbeat or malformed JSON) so callers can short-circuit.
 *
 * The server emits `{ event, payload }`; we map `event` -> `type` and supply a
 * client-side receive `timestamp` (the server envelope carries none) so gap
 * detection keeps working.
 */
export function parseServerMessage(raw: unknown): WebSocketMessage | null {
  if (typeof raw !== 'object' || raw === null) {
    return null;
  }

  const obj = raw as Record<string, unknown>;

  // Canonical server envelope: { event, payload }.
  if (typeof obj.event === 'string') {
    return {
      type: obj.event,
      payload: obj.payload ?? null,
      timestamp: new Date().toISOString(),
    };
  }

  return null;
}

/**
 * Build the full WebSocket connection URL: base origin + canonical path +
 * `?token=<jwt>` query param.
 *
 * Browsers cannot set an `Authorization` header on a WebSocket handshake, so
 * the api-server reads the JWT from the `token` query parameter (see
 * `WsQuery` in `ws_notifications.rs`). The `base` may be a bare origin
 * (`ws://localhost:8080`) or already include the path — in either case we
 * normalise to the canonical path and attach the token.
 */
export function buildWebSocketUrl(base: string, token: string): string {
  // `URL` needs an absolute base; ws/wss are valid absolute schemes.
  const url = new URL(base);
  // Force the canonical path regardless of what the base carried, so a
  // legacy `/ws` (or empty) path is corrected to the mounted handler.
  url.pathname = WS_NOTIFICATIONS_PATH;
  url.searchParams.set('token', token);
  return url.toString();
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
 * Compute the default WebSocket base origin (scheme + host, no path).
 *
 * The canonical handler path ({@link WS_NOTIFICATIONS_PATH}) and `?token=`
 * query param are appended by {@link buildWebSocketUrl} at connect time, so
 * this only needs to resolve the origin.
 *
 * In dev we keep the explicit cleartext `ws://localhost:8080` so the local
 * Vite stack works without TLS. In production we never default to cleartext:
 * we derive `wss://${location.host}` so the WS scheme tracks the page scheme
 * and the deploy host. Callers can still override via the `url` config option
 * or the `VITE_WS_URL` env variable.
 */
function defaultWebSocketUrl(): string {
  if (import.meta.env.DEV) {
    return 'ws://localhost:8080';
  }

  // SSR / non-browser safety net — should not happen for ppt-web (SPA),
  // but keeps the function total.
  if (typeof window === 'undefined' || !window.location) {
    return 'wss://localhost';
  }

  return `wss://${window.location.host}`;
}

/**
 * Dev-only diagnostic logging for the WebSocket client.
 *
 * The connection diagnostics below (send failures, handler errors, pong
 * timeouts, reconnect give-up) are valuable while developing but must not
 * leak to the browser console in production builds: they surface internal
 * connection state and add noise for end users. Gating on `import.meta.env.DEV`
 * — the same convention `lib/api.ts` uses for its retry logging — lets Vite
 * dead-code-eliminate these calls from the production bundle entirely.
 */
function wsWarn(...args: unknown[]): void {
  if (import.meta.env.DEV) {
    console.warn(...args);
  }
}

function wsError(...args: unknown[]): void {
  if (import.meta.env.DEV) {
    console.error(...args);
  }
}

/**
 * WebSocket service that manages connection, reconnection, and message handling.
 */
export class WebSocketService {
  private socket: WebSocket | null = null;
  private connectionState: ConnectionState = 'disconnected';
  private lastError: Error | null = null;

  /**
   * The auth token the current (open or in-flight) socket was opened with.
   *
   * Used by {@link connect} to detect token rotation: if `connect()` is called
   * again while a socket is already live but the token has since changed, the
   * stale socket must be torn down and re-opened with the new token — otherwise
   * the early-return would leave a live connection authenticated with a
   * rotated-out token, and the server may close it on the next validation.
   */
  private currentToken: string | null = null;

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
   *
   * An explicit `connect()` is a fresh (re)start request, so it resets the
   * reconnect-attempt budget via {@link resetReconnectAttempts}. This is what
   * makes a resume after a previous give-up work: once
   * {@link scheduleReconnect} has hit `maxReconnectAttempts` and flipped
   * `shouldReconnect` off, the budget is stuck at the cap; without the reset a
   * subsequent `connect()` would open one socket and then immediately give up
   * again on the first unclean close. The internal backoff loop deliberately
   * calls {@link openConnection} directly (NOT `connect()`) so a mid-backoff
   * retry does not reset the budget — otherwise the cap could never be reached.
   */
  connect(): void {
    this.resetReconnectAttempts();
    this.openConnection();
  }

  /**
   * Open (or re-open) the underlying socket without touching the reconnect
   * budget. Shared by the public {@link connect} entry point and the internal
   * exponential-backoff reconnect loop.
   */
  private openConnection(): void {
    const token = this.getToken();
    if (!token) {
      this.setConnectionState('error', new Error('No auth token available'));
      return;
    }

    // Only keep the existing socket if it is live (open or still handshaking)
    // AND was opened with the same token. If the token has rotated we must
    // fall through and re-establish the connection with the new token — a bare
    // `readyState === OPEN` early-return would leave a live socket bound to the
    // stale token (the server can then drop it on the next auth check, or worse
    // keep honouring a token that should no longer be valid).
    if (
      this.socket &&
      (this.socket.readyState === WebSocket.OPEN ||
        this.socket.readyState === WebSocket.CONNECTING) &&
      token === this.currentToken
    ) {
      return; // Already connected/connecting with the current token
    }

    // Any pre-existing socket here is either bound to a rotated-out token or
    // already closed/closing. Tear it down cleanly before opening a new one so
    // we never run two sockets in parallel or leak the stale connection.
    this.teardownStaleSocket();

    this.currentToken = token;
    this.shouldReconnect = true;
    this.clearReconnectTimeout();

    // Auth transport: the api-server reads the JWT from the `token` query
    // parameter (see `WsQuery` in `ws_notifications.rs`). Browsers cannot set
    // an `Authorization` header on a WS handshake, so the query param is the
    // contract the server validates. We still offer the `ppt.v1` subprotocol
    // because the server echoes it (Issue #438) to complete the handshake and
    // to leave room for future protocol-version negotiation.
    this.setConnectionState('connecting');

    try {
      this.socket = new WebSocket(buildWebSocketUrl(this.url, token), ['ppt.v1']);
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

    this.currentToken = null;
    this.setConnectionState('disconnected');
  }

  /**
   * Tear down the current socket without touching reconnect intent.
   *
   * Unlike {@link disconnect}, this does NOT flip `shouldReconnect` or emit a
   * `disconnected` state — it is an internal replace step used by
   * {@link connect} when re-establishing the connection (e.g. after a token
   * rotation) or when the previous socket is already closed. The stale socket's
   * handlers are detached first so its imminent `close` event cannot drive a
   * spurious reconnect or state transition on the service that is about to own
   * a fresh socket.
   */
  private teardownStaleSocket(): void {
    if (!this.socket) return;

    this.stopHeartbeat();

    const stale = this.socket;
    stale.onopen = null;
    stale.onclose = null;
    stale.onerror = null;
    stale.onmessage = null;

    try {
      stale.close(1000, 'Reconnecting with a new token');
    } catch {
      // Closing an already-closed/closing socket can throw in some engines;
      // the socket is being discarded regardless, so the error is irrelevant.
    }

    this.socket = null;
  }

  /**
   * Send a message through the WebSocket.
   */
  send(message: WebSocketMessage): boolean {
    if (!this.isConnected()) {
      wsWarn('[WebSocket] Cannot send message: not connected');
      return false;
    }

    try {
      this.socket!.send(JSON.stringify(message));
      return true;
    } catch (error) {
      wsError('[WebSocket] Failed to send message:', error);
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

      // Handle pong response. The client emits `{ type: 'ping' }` and the
      // server may echo `{ type: 'pong' }`; either heartbeat reply is matched
      // here before the canonical-envelope path below.
      if (data && typeof data === 'object' && data.type === 'pong') {
        this.handlePong();
        return;
      }

      // Server speaks the canonical `{ event, payload }` envelope. Normalise
      // it into the internal message shape (event -> type) so subscribers can
      // key off real event names (`notification.created`, `preference.updated`).
      const message = parseServerMessage(data);
      if (!message) {
        // Not a recognised event envelope (e.g. a stray heartbeat frame).
        return;
      }

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
            wsError(`[WebSocket] Handler error for ${message.type}:`, handlerError);
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
            wsError('[WebSocket] Wildcard handler error:', handlerError);
          }
        }
      }
    } catch (error) {
      wsError('[WebSocket] Failed to parse message:', error);
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
        wsError('[WebSocket] Connection state handler error:', handlerError);
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
      wsWarn(
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
      // Continue the backoff loop via openConnection() so this retry does NOT
      // reset the attempt budget — only an explicit connect() does that.
      this.openConnection();
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
          wsError('[WebSocket] max-retries handler error:', handlerError);
        }
      }
    }

    const wildcardHandlers = this.messageHandlers.get('*');
    if (wildcardHandlers) {
      for (const handler of wildcardHandlers) {
        try {
          handler(message);
        } catch (handlerError) {
          wsError('[WebSocket] Wildcard handler error:', handlerError);
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
          wsWarn('[WebSocket] Pong timeout - closing connection');
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
