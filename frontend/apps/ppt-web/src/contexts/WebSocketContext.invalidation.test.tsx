/// <reference types="vitest/globals" />
/**
 * Realtime cache-invalidation mapping tests (regression).
 *
 * The api-server pushes canonical `domain.action` event envelopes on the
 * per-user `notifications:{user_id}` WebSocket channel — `notification.created`
 * (with a `payload.category`), `message.created`, `preference.updated`. The
 * client's `eventToQueryKeys` used to key on legacy `entity:*` names that the
 * server never emits, so every realtime frame missed the lookup and no
 * TanStack Query cache was ever invalidated (100% dead sync).
 *
 * These tests drive a real {@link WebSocketProvider} over a controllable fake
 * `WebSocket` and assert that the actual server event names now resolve to the
 * correct query roots via `onEntityEvent`.
 */

import { render } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { categoryToQueryKeys, eventToQueryKeys, WebSocketProvider } from './WebSocketContext';

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

  constructor(
    public url: string,
    public protocols?: string | string[]
  ) {
    FakeWebSocket.lastInstance = this;
  }

  send(): void {}

  close(): void {
    this.readyState = FakeWebSocket.CLOSED;
  }

  simulateOpen(): void {
    this.readyState = FakeWebSocket.OPEN;
    this.onopen?.({});
  }

  simulateServerEvent(event: string, payload: unknown): void {
    this.onmessage?.({ data: JSON.stringify({ event, payload }) });
  }
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

function mountProvider(onEntityEvent: (eventType: string, queryKeys: string[]) => void) {
  render(
    <WebSocketProvider
      auth={{ accessToken: 'test-jwt', isAuthenticated: true }}
      wsUrl="ws://localhost:8080"
      onEntityEvent={onEntityEvent}
    >
      <div>child</div>
    </WebSocketProvider>
  );
  const socket = FakeWebSocket.lastInstance;
  if (!socket) throw new Error('expected a WebSocket to be constructed');
  socket.simulateOpen();
  return socket;
}

describe('WebSocketContext — realtime event → query-key mapping', () => {
  it('does not key on any legacy entity:* event names', () => {
    for (const key of Object.keys(eventToQueryKeys)) {
      expect(key.startsWith('entity:')).toBe(false);
    }
  });

  it('invalidates the messages root on a message.created frame', () => {
    const onEntityEvent = vi.fn();
    const socket = mountProvider(onEntityEvent);

    socket.simulateServerEvent('message.created', { thread_id: 't-1', message_id: 'm-1' });

    expect(onEntityEvent).toHaveBeenCalledTimes(1);
    const [eventType, queryKeys] = onEntityEvent.mock.calls[0];
    expect(eventType).toBe('message.created');
    expect(queryKeys).toEqual(['messages']);
  });

  it('invalidates the notifications root on a preference.updated frame', () => {
    const onEntityEvent = vi.fn();
    const socket = mountProvider(onEntityEvent);

    socket.simulateServerEvent('preference.updated', { channel: 'push', enabled: false });

    expect(onEntityEvent).toHaveBeenCalledTimes(1);
    expect(onEntityEvent.mock.calls[0][1]).toEqual(['notifications']);
  });

  it('routes notification.created by payload.category to the notifications + entity roots', () => {
    const onEntityEvent = vi.fn();
    const socket = mountProvider(onEntityEvent);

    socket.simulateServerEvent('notification.created', {
      notification_id: 'n-1',
      category: 'faults',
      title: 'New fault',
    });

    expect(onEntityEvent).toHaveBeenCalledTimes(1);
    const [eventType, queryKeys] = onEntityEvent.mock.calls[0];
    expect(eventType).toBe('notification.created');
    expect(queryKeys).toEqual(['notifications', ...categoryToQueryKeys.faults]);
  });

  it('falls back to the notifications root for an unknown notification category', () => {
    const onEntityEvent = vi.fn();
    const socket = mountProvider(onEntityEvent);

    socket.simulateServerEvent('notification.created', { category: 'system' });

    expect(onEntityEvent).toHaveBeenCalledTimes(1);
    expect(onEntityEvent.mock.calls[0][1]).toEqual(['notifications']);
  });

  it('does not fire for a legacy entity:updated frame (server never emits it)', () => {
    const onEntityEvent = vi.fn();
    const socket = mountProvider(onEntityEvent);

    socket.simulateServerEvent('entity:updated', { entityType: 'fault' });

    expect(onEntityEvent).not.toHaveBeenCalled();
  });
});
