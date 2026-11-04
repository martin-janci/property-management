/// <reference types="vitest/globals" />
/**
 * Unit tests for the realtime WebSocket contract helpers (#775, #819).
 *
 * These cover the two ends of the client/server contract that were broken:
 *
 * 1. URL builder — the client must connect to the canonical mounted path
 *    `/api/v1/users/me/notifications/ws` with the JWT as a `?token=` query
 *    param (the api-server reads `WsQuery.token`; browsers cannot set a WS
 *    `Authorization` header). A bare `/ws` 404s on upgrade.
 * 2. Message parser — the server emits the canonical `{ event, payload }`
 *    envelope (`WsEvent` in `ws_notifications.rs`). The client must map
 *    `event` -> internal `type` so subscribers fire on the real event names
 *    (`notification.created`, `preference.updated`).
 *
 * A drift in either direction means realtime cannot connect or silently drops
 * every event, so the round-trip is asserted explicitly.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  buildWebSocketUrl,
  buildWsSubprotocols,
  parseServerMessage,
  WebSocketService,
  WS_NOTIFICATIONS_PATH,
} from './websocket';

describe('buildWebSocketUrl', () => {
  it('appends the canonical handler path to a bare origin', () => {
    const url = buildWebSocketUrl('ws://localhost:8080');
    expect(url).toBe(`ws://localhost:8080${WS_NOTIFICATIONS_PATH}`);
  });

  it('never puts a token in the URL (JWT travels in the handshake subprotocol)', () => {
    const url = new URL(buildWebSocketUrl('wss://app.example.com'));
    expect(url.searchParams.get('token')).toBeNull();
    expect(url.search).toBe('');
    expect(url.pathname).toBe(WS_NOTIFICATIONS_PATH);
  });

  it('corrects a legacy /ws path on the base to the canonical path', () => {
    const url = new URL(buildWebSocketUrl('ws://localhost:8080/ws'));
    expect(url.pathname).toBe(WS_NOTIFICATIONS_PATH);
  });

  it('preserves the wss scheme for production hosts', () => {
    const url = buildWebSocketUrl('wss://app.example.com');
    expect(url.startsWith('wss://app.example.com')).toBe(true);
  });
});

describe('buildWsSubprotocols', () => {
  it('carries the JWT as a bearer subprotocol alongside the version protocol', () => {
    expect(buildWsSubprotocols('jwt-abc')).toEqual(['bearer.jwt-abc', 'ppt.v1']);
  });
});

describe('parseServerMessage', () => {
  it('maps the canonical {event, payload} envelope onto the internal type', () => {
    const msg = parseServerMessage({
      event: 'notification.created',
      payload: { notification_id: '00000000-0000-0000-0000-000000000001', title: 'Hi' },
    });
    expect(msg).not.toBeNull();
    expect(msg?.type).toBe('notification.created');
    expect(msg?.payload).toEqual({
      notification_id: '00000000-0000-0000-0000-000000000001',
      title: 'Hi',
    });
    expect(typeof msg?.timestamp).toBe('string');
  });

  it('handles the preference.updated event', () => {
    const msg = parseServerMessage({ event: 'preference.updated', payload: { channel: 'email' } });
    expect(msg?.type).toBe('preference.updated');
    expect(msg?.payload).toEqual({ channel: 'email' });
  });

  it('defaults a missing payload to null', () => {
    const msg = parseServerMessage({ event: 'preference.updated' });
    expect(msg?.type).toBe('preference.updated');
    expect(msg?.payload).toBeNull();
  });

  it('returns null for a frame without an event field', () => {
    expect(parseServerMessage({ type: 'pong' })).toBeNull();
    expect(parseServerMessage({ foo: 'bar' })).toBeNull();
  });

  it('returns null for non-object input', () => {
    expect(parseServerMessage(null)).toBeNull();
    expect(parseServerMessage('string')).toBeNull();
    expect(parseServerMessage(42)).toBeNull();
  });
});

/**
 * Minimal `WebSocket` double so the service lifecycle can be driven under fake
 * timers without a real network. Records every instance and every `close()`
 * call so a test can assert the client never force-closes a healthy socket.
 */
class MockWebSocket {
  static readonly CONNECTING = 0;
  static readonly OPEN = 1;
  static readonly CLOSING = 2;
  static readonly CLOSED = 3;

  static instances: MockWebSocket[] = [];

  readyState = MockWebSocket.CONNECTING;
  onopen: (() => void) | null = null;
  onclose: ((event: { code: number; reason: string; wasClean: boolean }) => void) | null = null;
  onerror: (() => void) | null = null;
  onmessage: ((event: { data: string }) => void) | null = null;

  readonly url: string;
  readonly protocols?: string | string[];
  readonly sent: string[] = [];
  readonly closeCalls: { code?: number; reason?: string }[] = [];

  constructor(url: string, protocols?: string | string[]) {
    this.url = url;
    this.protocols = protocols;
    MockWebSocket.instances.push(this);
  }

  send(data: string): void {
    this.sent.push(data);
  }

  close(code?: number, reason?: string): void {
    this.closeCalls.push({ code, reason });
    this.readyState = MockWebSocket.CLOSED;
    this.onclose?.({ code: code ?? 1000, reason: reason ?? '', wasClean: code === 1000 });
  }

  /** Test helper: drive the handshake to open. */
  simulateOpen(): void {
    this.readyState = MockWebSocket.OPEN;
    this.onopen?.();
  }
}

describe('WebSocketService heartbeat (regression: #ws-pong-timeout-drop)', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    MockWebSocket.instances = [];
    vi.stubGlobal('WebSocket', MockWebSocket);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.useRealTimers();
  });

  it('does not force-close the socket after 60s with no app-level pong reply', () => {
    // Regression: the client used to send an application-level {type:'ping'}
    // every 30s and force-close with code 4000 "Pong timeout" 10s later because
    // the api-server never emits an app-level {type:'pong'}. Liveness now lives
    // purely at the protocol layer (server Ping + browser auto-Pong), so a
    // healthy socket that receives no app-level pong must stay open.
    const service = new WebSocketService({ getToken: () => 'jwt-token' });

    service.connect();
    expect(MockWebSocket.instances).toHaveLength(1);
    const socket = MockWebSocket.instances[0];
    socket.simulateOpen();
    expect(service.getConnectionState()).toBe('connected');

    // Well past the old 30s heartbeat + 10s pong-timeout window.
    vi.advanceTimersByTime(60_000);

    // The client must not have closed the socket…
    expect(socket.closeCalls).toHaveLength(0);
    // …must never send an app-level heartbeat frame…
    expect(socket.sent).toHaveLength(0);
    // …must not have scheduled a reconnect (no second socket)…
    expect(MockWebSocket.instances).toHaveLength(1);
    // …and must still report itself connected.
    expect(service.getConnectionState()).toBe('connected');

    service.disconnect();
  });
});

describe('WebSocketService auth transport (security: JWT must not leak into URL)', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    MockWebSocket.instances = [];
    vi.stubGlobal('WebSocket', MockWebSocket);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.useRealTimers();
  });

  it('never puts the JWT in the connection URL and carries it as a subprotocol', () => {
    // Regression: the client used to append `?token=<jwt>` to the WS URL, which
    // leaks the bearer token into api-server access logs / proxy logs (a single
    // log line is enough for session takeover). The token must now travel in the
    // `Sec-WebSocket-Protocol` handshake header as `bearer.<jwt>` instead.
    const jwt = 'secret-jwt-token-value';
    const service = new WebSocketService({ getToken: () => jwt });

    service.connect();
    expect(MockWebSocket.instances).toHaveLength(1);
    const socket = MockWebSocket.instances[0];

    // The token must NOT appear anywhere in the connection URL.
    expect(socket.url).not.toContain(jwt);
    expect(socket.url).not.toContain('token=');

    // It must instead be offered as the `bearer.<jwt>` subprotocol, alongside
    // the `ppt.v1` protocol the server echoes to complete the handshake.
    expect(socket.protocols).toEqual([`bearer.${jwt}`, 'ppt.v1']);

    service.disconnect();
  });
});
