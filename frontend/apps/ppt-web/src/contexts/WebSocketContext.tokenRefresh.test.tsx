/// <reference types="vitest/globals" />
/**
 * End-to-end stale-token-on-refresh regression for {@link WebSocketProvider}.
 *
 * The provider bridges AuthContext to the core WebSocket client: it feeds the
 * current access token in via the `auth.accessToken` prop (App.tsx passes
 * `getAccessToken()`), and re-arms the socket from an effect keyed on that
 * token. When a silent refresh rotates the access token, App re-renders the
 * provider with the new value — and the live socket MUST be re-established with
 * the refreshed token rather than left pinned to the one it opened with.
 *
 * This complements `lib/websocket.tokenRefresh.test.ts` (which pins the service
 * contract directly) by exercising the actual refresh pathway through the React
 * provider, so the wiring that turns a prop change into a reconnect can't
 * silently regress.
 */

import { render } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { WebSocketProvider } from './WebSocketContext';

class FakeWebSocket {
  static readonly CONNECTING = 0;
  static readonly OPEN = 1;
  static readonly CLOSING = 2;
  static readonly CLOSED = 3;

  static instances: FakeWebSocket[] = [];

  readyState = FakeWebSocket.CONNECTING;
  onopen: ((ev: unknown) => void) | null = null;
  onclose: ((ev: unknown) => void) | null = null;
  onerror: ((ev: unknown) => void) | null = null;
  onmessage: ((ev: { data: string }) => void) | null = null;

  closeCalls = 0;

  constructor(
    public url: string,
    public protocols?: string | string[]
  ) {
    FakeWebSocket.instances.push(this);
  }

  send(): void {}

  close(): void {
    this.closeCalls += 1;
    this.readyState = FakeWebSocket.CLOSED;
    this.onclose?.({ code: 1000, reason: '', wasClean: true });
  }

  simulateOpen(): void {
    this.readyState = FakeWebSocket.OPEN;
    this.onopen?.({});
  }

  get bearer(): string | undefined {
    const protocols = Array.isArray(this.protocols) ? this.protocols : [];
    return protocols.find((p) => p.startsWith('bearer.'));
  }
}

let originalWebSocket: typeof globalThis.WebSocket;

beforeEach(() => {
  originalWebSocket = globalThis.WebSocket;
  // biome-ignore lint/suspicious/noExplicitAny: test double for the global WebSocket
  globalThis.WebSocket = FakeWebSocket as any;
  FakeWebSocket.instances = [];
});

afterEach(() => {
  globalThis.WebSocket = originalWebSocket;
  vi.restoreAllMocks();
});

describe('WebSocketProvider — reconnects on token refresh', () => {
  it('re-establishes the socket with the refreshed bearer token when accessToken changes', () => {
    const { rerender } = render(
      <WebSocketProvider
        auth={{ accessToken: 'tok-A', isAuthenticated: true }}
        wsUrl="ws://localhost:8080"
      >
        <div>child</div>
      </WebSocketProvider>
    );

    expect(FakeWebSocket.instances).toHaveLength(1);
    const first = FakeWebSocket.instances[0];
    first.simulateOpen();
    expect(first.bearer).toBe('bearer.tok-A');

    // Silent refresh rotates the access token; App re-renders the provider with
    // the new value.
    rerender(
      <WebSocketProvider
        auth={{ accessToken: 'tok-B', isAuthenticated: true }}
        wsUrl="ws://localhost:8080"
      >
        <div>child</div>
      </WebSocketProvider>
    );

    // A fresh socket must come up offering the refreshed token — the provider
    // must not keep the tok-A socket live.
    const latest = FakeWebSocket.instances[FakeWebSocket.instances.length - 1];
    expect(FakeWebSocket.instances.length).toBeGreaterThanOrEqual(2);
    expect(latest.bearer).toBe('bearer.tok-B');
    expect(first.closeCalls).toBeGreaterThanOrEqual(1);
  });
});
