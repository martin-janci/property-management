/// <reference types="vitest/globals" />
/**
 * Stale-token-on-refresh regression tests for the core WebSocket client
 * (`lib/websocket.ts`).
 *
 * The notification socket authenticates by offering the access token as the
 * `bearer.<jwt>` handshake subprotocol (see `buildWsSubprotocols`). A long-lived
 * socket therefore pins whatever token it was opened with — so when the access
 * token is rotated mid-session (silent refresh, 401-replay, cold-boot refresh)
 * the client MUST re-establish the connection with the fresh token. If it kept
 * the old socket it would keep authenticating with a token the server has
 * rotated out: either the server drops it on the next validation (realtime goes
 * dark for the session) or it keeps honouring a credential that should no longer
 * be valid.
 *
 * `WebSocketService` guards against this two ways, both exercised here:
 *   1. `openConnection()` records the token each socket was opened with
 *      (`currentToken`) and, when `connect()` is called again with a *different*
 *      token, tears the stale socket down and re-opens with the new one — while
 *      an *unchanged* token on a healthy socket is a no-op (no reconnect churn).
 *   2. The internal backoff reconnect re-reads `getToken()` on every attempt, so
 *      a socket that was dropped (e.g. a server auth-expiry close) comes back up
 *      offering the latest token rather than the one captured at first connect.
 *
 * These paths existed but were previously untested — the reconnect-budget suite
 * pins a static `getToken: () => 'test-jwt'`, so a refactor could silently
 * reintroduce the stale-token bug with no failing test. This file locks the
 * behaviour in.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { WebSocketService } from './websocket';

/**
 * Controllable `WebSocket` double. Records every constructed instance (with the
 * subprotocols it was offered, which carry the bearer token) and every
 * `close()` call so a test can assert both the teardown of a stale socket and
 * the bearer token the replacement socket offers.
 */
class FakeWebSocket {
  static readonly CONNECTING = 0;
  static readonly OPEN = 1;
  static readonly CLOSING = 2;
  static readonly CLOSED = 3;

  static instances: FakeWebSocket[] = [];

  readyState = FakeWebSocket.CONNECTING;
  onopen: ((ev: unknown) => void) | null = null;
  onclose: ((ev: { code: number; reason: string; wasClean: boolean }) => void) | null = null;
  onerror: ((ev: unknown) => void) | null = null;
  onmessage: ((ev: { data: string }) => void) | null = null;

  readonly closeCalls: { code?: number; reason?: string }[] = [];

  constructor(
    public url: string,
    public protocols?: string | string[]
  ) {
    FakeWebSocket.instances.push(this);
  }

  send(): void {}

  close(code?: number, reason?: string): void {
    this.closeCalls.push({ code, reason });
    this.readyState = FakeWebSocket.CLOSED;
    this.onclose?.({ code: code ?? 1000, reason: reason ?? '', wasClean: code === 1000 });
  }

  /** Drive the handshake to open. */
  simulateOpen(): void {
    this.readyState = FakeWebSocket.OPEN;
    this.onopen?.({});
  }

  /** Simulate an unexpected drop (e.g. the server closing on token expiry). */
  simulateUncleanClose(code = 4001): void {
    this.readyState = FakeWebSocket.CLOSED;
    this.onclose?.({ code, reason: 'auth expired', wasClean: false });
  }

  /** The bearer subprotocol this socket was opened with (`bearer.<jwt>`). */
  get bearer(): string | undefined {
    const protocols = Array.isArray(this.protocols) ? this.protocols : [];
    return protocols.find((p) => p.startsWith('bearer.'));
  }
}

let originalWebSocket: typeof globalThis.WebSocket;

beforeEach(() => {
  vi.useFakeTimers();
  originalWebSocket = globalThis.WebSocket;
  // biome-ignore lint/suspicious/noExplicitAny: test double for the global WebSocket
  globalThis.WebSocket = FakeWebSocket as any;
  FakeWebSocket.instances = [];
});

afterEach(() => {
  vi.useRealTimers();
  globalThis.WebSocket = originalWebSocket;
  vi.restoreAllMocks();
});

describe('WebSocketService — stale token on refresh', () => {
  it('reconnects with the refreshed token when connect() is called after a rotation', () => {
    // A mutable token source models the AuthContext getter: the value it returns
    // changes the moment a silent refresh rotates the access token.
    let token = 'tok-A';
    const service = new WebSocketService({ url: 'ws://localhost:8080', getToken: () => token });

    service.connect();
    expect(FakeWebSocket.instances).toHaveLength(1);
    const first = FakeWebSocket.instances[0];
    first.simulateOpen();
    expect(service.getConnectionState()).toBe('connected');
    expect(first.bearer).toBe('bearer.tok-A');

    // Token is rotated (silent refresh), then the provider re-arms the socket by
    // calling connect() again — the exact path WebSocketProvider drives from its
    // `auth.accessToken`-change effect.
    token = 'tok-B';
    service.connect();

    // The stale socket must be torn down (cleanly) and a *new* socket opened
    // offering the refreshed bearer token — never left authenticating with tok-A.
    expect(first.closeCalls).toHaveLength(1);
    expect(FakeWebSocket.instances).toHaveLength(2);
    const second = FakeWebSocket.instances[1];
    expect(second).not.toBe(first);
    expect(second.bearer).toBe('bearer.tok-B');

    service.disconnect();
  });

  it('does not churn a healthy socket when connect() is called with an unchanged token', () => {
    // The flip side of rotation detection: a redundant connect() with the SAME
    // token on an open socket must be a no-op, or every auth re-render would
    // needlessly drop and re-open the live connection.
    const service = new WebSocketService({ url: 'ws://localhost:8080', getToken: () => 'tok-A' });

    service.connect();
    const first = FakeWebSocket.instances[0];
    first.simulateOpen();

    service.connect();

    expect(FakeWebSocket.instances).toHaveLength(1);
    expect(first.closeCalls).toHaveLength(0);
    expect(service.getConnectionState()).toBe('connected');

    service.disconnect();
  });

  it('re-authenticates with the latest token on a background reconnect after a server auth-expiry close', () => {
    // No explicit connect() here: the socket is dropped by the server and the
    // service's own exponential-backoff loop brings it back. That loop re-reads
    // getToken() on each attempt, so the refreshed token must be offered on the
    // reconnect — a captured/stale token would re-authenticate with the expired
    // credential and the server would just close it again.
    let token = 'tok-A';
    const service = new WebSocketService({
      url: 'ws://localhost:8080',
      getToken: () => token,
      minReconnectDelay: 1,
      maxReconnectDelay: 5,
    });

    service.connect();
    const first = FakeWebSocket.instances[0];
    first.simulateOpen();
    expect(first.bearer).toBe('bearer.tok-A');

    // The access token is refreshed in the background (the getter now returns the
    // new value), then the server closes the socket because the old token lapsed.
    token = 'tok-B';
    first.simulateUncleanClose();

    // Let the backoff timer fire and re-open the socket.
    vi.advanceTimersByTime(10);

    expect(FakeWebSocket.instances.length).toBeGreaterThanOrEqual(2);
    const latest = FakeWebSocket.instances[FakeWebSocket.instances.length - 1];
    expect(latest.bearer).toBe('bearer.tok-B');

    service.disconnect();
  });
});
