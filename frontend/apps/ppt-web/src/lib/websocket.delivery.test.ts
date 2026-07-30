/// <reference types="vitest/globals" />
/**
 * Realtime delivery tests for the messaging WebSocket transport
 * (Coverage 6-5 — Epic 6, Story 6.5).
 *
 * `websocket.test.ts` already pins the two pure contract helpers
 * (`buildWebSocketUrl`, `parseServerMessage`). The untested half is the live
 * dispatch path inside `WebSocketService`: an inbound `{ event, payload }`
 * frame off the socket must be parsed and fanned out to the right subscribers.
 * Direct messaging realtime sync rides this exact path — the api-server pushes
 * a `message:new` envelope and the messaging UI subscribes to it to invalidate
 * its caches. If dispatch drops `message:new`, the inbox never updates until a
 * manual refresh.
 *
 * These tests install a controllable fake `WebSocket` so we can drive `onopen`
 * and `onmessage` deterministically and assert:
 *  - a `message:new` frame reaches a typed subscriber with the payload intact;
 *  - wildcard (`*`) subscribers also see it;
 *  - subscribers for other event types are NOT invoked (no cross-talk);
 *  - `unsubscribe()` stops further delivery (no leak after a thread unmounts).
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { WebSocketService } from './websocket';

/**
 * Minimal controllable WebSocket stand-in. Captures the handlers the service
 * attaches and exposes helpers to simulate the connection opening and the
 * server pushing a frame.
 */
class FakeWebSocket {
  static readonly CONNECTING = 0;
  static readonly OPEN = 1;
  static readonly CLOSING = 2;
  static readonly CLOSED = 3;

  static lastInstance: FakeWebSocket | null = null;

  readyState = FakeWebSocket.CONNECTING;
  onopen: ((ev: unknown) => void) | null = null;
  onclose: ((ev: unknown) => void) | null = null;
  onerror: ((ev: unknown) => void) | null = null;
  onmessage: ((ev: { data: string }) => void) | null = null;
  sent: string[] = [];

  constructor(
    public url: string,
    public protocols?: string | string[]
  ) {
    FakeWebSocket.lastInstance = this;
  }

  send(data: string): void {
    this.sent.push(data);
  }

  close(): void {
    this.readyState = FakeWebSocket.CLOSED;
  }

  /** Simulate the socket transitioning to OPEN. */
  simulateOpen(): void {
    this.readyState = FakeWebSocket.OPEN;
    this.onopen?.({});
  }

  /** Simulate the server pushing a canonical `{ event, payload }` frame. */
  simulateServerEvent(event: string, payload: unknown): void {
    this.onmessage?.({ data: JSON.stringify({ event, payload }) });
  }
}

function connectedService(): { service: WebSocketService; socket: FakeWebSocket } {
  const service = new WebSocketService({
    url: 'ws://localhost:8080',
    getToken: () => 'test-jwt',
  });
  service.connect();
  const socket = FakeWebSocket.lastInstance;
  if (!socket) throw new Error('expected a WebSocket to be constructed');
  socket.simulateOpen();
  return { service, socket };
}

let originalWebSocket: typeof globalThis.WebSocket;

beforeEach(() => {
  originalWebSocket = globalThis.WebSocket;
  // biome-ignore lint/suspicious/noExplicitAny: test double for the global WebSocket
  globalThis.WebSocket = FakeWebSocket as any;
  FakeWebSocket.lastInstance = null;
});

afterEach(() => {
  globalThis.WebSocket = originalWebSocket;
  vi.restoreAllMocks();
});

describe('WebSocketService — message:new delivery', () => {
  it('delivers a message:new frame to a typed subscriber with payload intact', () => {
    const { service, socket } = connectedService();
    const handler = vi.fn();
    service.subscribe('message:new', handler);

    const payload = { threadId: 'thread-1', messageId: 'm-9', senderId: 'user-2' };
    socket.simulateServerEvent('message:new', payload);

    expect(handler).toHaveBeenCalledTimes(1);
    const received = handler.mock.calls[0][0];
    expect(received.type).toBe('message:new');
    expect(received.payload).toEqual(payload);
    expect(typeof received.timestamp).toBe('string');
  });

  it('also fans the frame out to wildcard (*) subscribers', () => {
    const { service, socket } = connectedService();
    const typed = vi.fn();
    const wildcard = vi.fn();
    service.subscribe('message:new', typed);
    service.subscribe('*', wildcard);

    socket.simulateServerEvent('message:new', { threadId: 'thread-1' });

    expect(typed).toHaveBeenCalledTimes(1);
    expect(wildcard).toHaveBeenCalledTimes(1);
    expect(wildcard.mock.calls[0][0].type).toBe('message:new');
  });

  it('does not invoke subscribers for unrelated event types', () => {
    const { service, socket } = connectedService();
    const messageHandler = vi.fn();
    const faultHandler = vi.fn();
    service.subscribe('message:new', messageHandler);
    service.subscribe('notification:fault', faultHandler);

    socket.simulateServerEvent('message:new', { threadId: 'thread-1' });

    expect(messageHandler).toHaveBeenCalledTimes(1);
    expect(faultHandler).not.toHaveBeenCalled();
  });

  it('stops delivery after unsubscribe (no leak when a thread view unmounts)', () => {
    const { service, socket } = connectedService();
    const handler = vi.fn();
    const unsubscribe = service.subscribe('message:new', handler);

    socket.simulateServerEvent('message:new', { threadId: 'thread-1' });
    expect(handler).toHaveBeenCalledTimes(1);

    unsubscribe();
    socket.simulateServerEvent('message:new', { threadId: 'thread-1' });
    // still 1 — the unsubscribed handler must not fire again
    expect(handler).toHaveBeenCalledTimes(1);
  });

  it('tracks the last event timestamp for gap detection on delivery', () => {
    const { service, socket } = connectedService();
    expect(service.getLastEventTimestamp()).toBeNull();

    service.subscribe('message:new', vi.fn());
    socket.simulateServerEvent('message:new', { threadId: 'thread-1' });

    expect(service.getLastEventTimestamp()).not.toBeNull();
  });
});

/**
 * Token-rotation re-auth (regression).
 *
 * The auth layer rotates the access token periodically. `WebSocketContext`
 * reacts to the new token by calling `service.connect()` again. Before the fix,
 * `connect()` early-returned whenever a socket was already OPEN — regardless of
 * the token — leaving a live connection bound to the *stale* token. The server
 * can drop that socket on its next auth check (silent realtime outage) or keep
 * honouring a rotated-out token.
 *
 * The contract asserted here: a `connect()` whose token differs from the live
 * socket's token tears the stale socket down and opens a fresh one carrying the
 * new token — while a `connect()` with the unchanged token still no-ops.
 */
describe('WebSocketService — token rotation re-auth', () => {
  function tokenParam(socket: FakeWebSocket): string | null {
    return new URL(socket.url).searchParams.get('token');
  }

  it('tears down the stale socket and reconnects with the rotated token', () => {
    let token = 'token-old';
    const service = new WebSocketService({
      url: 'ws://localhost:8080',
      getToken: () => token,
    });

    service.connect();
    const first = FakeWebSocket.lastInstance;
    if (!first) throw new Error('expected the first WebSocket to be constructed');
    first.simulateOpen();
    expect(tokenParam(first)).toBe('token-old');

    // Auth layer rotates the token, then the context re-invokes connect().
    token = 'token-new';
    service.connect();

    const second = FakeWebSocket.lastInstance;
    if (!second) throw new Error('expected a second WebSocket to be constructed');

    // A brand-new socket carrying the rotated token must have been opened...
    expect(second).not.toBe(first);
    expect(tokenParam(second)).toBe('token-new');
    // ...and the stale socket bound to the old token must have been closed.
    expect(first.readyState).toBe(FakeWebSocket.CLOSED);
  });

  it('no-ops when connect() is called again with the unchanged token', () => {
    const service = new WebSocketService({
      url: 'ws://localhost:8080',
      getToken: () => 'token-stable',
    });

    service.connect();
    const first = FakeWebSocket.lastInstance;
    if (!first) throw new Error('expected the first WebSocket to be constructed');
    first.simulateOpen();

    // Same token → the existing live socket is reused, no new socket created.
    service.connect();
    expect(FakeWebSocket.lastInstance).toBe(first);
    expect(first.readyState).toBe(FakeWebSocket.OPEN);
  });

  it('detaches stale-socket handlers so its close does not trigger a reconnect', () => {
    let token = 'rot-a';
    const service = new WebSocketService({
      url: 'ws://localhost:8080',
      getToken: () => token,
      minReconnectDelay: 1,
    });

    service.connect();
    const first = FakeWebSocket.lastInstance;
    if (!first) throw new Error('expected the first WebSocket to be constructed');
    first.simulateOpen();
    const firstOnClose = first.onclose;

    token = 'rot-b';
    service.connect();

    // The stale socket's handlers were detached during teardown, so a late
    // close event on it is inert and cannot drive the service's reconnect path.
    expect(first.onclose).toBeNull();
    expect(firstOnClose).not.toBeNull();
  });
});
