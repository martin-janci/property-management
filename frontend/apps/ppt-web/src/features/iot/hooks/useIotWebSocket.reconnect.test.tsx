/// <reference types="vitest/globals" />
/**
 * Reconnect-policy regression tests for {@link useIotWebSocket}.
 *
 * Bug (code-review-ppt-web-core-iot-ws-reconnect-no-backoff): the IoT sensor
 * WebSocket reconnected on every unclean close at a FIXED 5 s interval with no
 * exponential backoff, no attempt cap, and no give-up. A sensor endpoint that
 * stayed down (redeploy, Redis outage, revoked membership) therefore produced a
 * reconnect storm — one socket every 5 s, forever, from every open IoT tab.
 *
 * Fix: mirror the shared notification stack's policy
 * ({@link WebSocketService} in `lib/websocket.ts`) by reusing its
 * `computeReconnectDelay` helper and `DEFAULT_MAX_RECONNECT_ATTEMPTS` cap:
 *   - exponential backoff with jitter (not a fixed interval),
 *   - stop after the attempt budget is exhausted (give up),
 *   - reset the budget on a successful open.
 *
 * These tests install a controllable fake `WebSocket` and drive the backoff
 * with fake timers. The load-bearing regression assertion is the give-up:
 * before the fix the socket count grows without bound; after it, it stops at
 * `DEFAULT_MAX_RECONNECT_ATTEMPTS + 1`.
 */

import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { DEFAULT_MAX_RECONNECT_ATTEMPTS } from '../../../lib/websocket';
import { useIotWebSocket } from './useIotWebSocket';

vi.mock('@ppt/api-client', () => ({
  getToken: () => 'test-jwt',
}));

class FakeWebSocket {
  static readonly CONNECTING = 0;
  static readonly OPEN = 1;
  static readonly CLOSING = 2;
  static readonly CLOSED = 3;

  static lastInstance: FakeWebSocket | null = null;
  static constructed = 0;

  readyState = FakeWebSocket.CONNECTING;
  onopen: ((ev: unknown) => void) | null = null;
  onclose: ((ev: unknown) => void) | null = null;
  onerror: ((ev: unknown) => void) | null = null;
  onmessage: ((ev: { data: string }) => void) | null = null;

  constructor(public url: string) {
    FakeWebSocket.lastInstance = this;
    FakeWebSocket.constructed += 1;
  }

  send(): void {}

  close(): void {
    this.readyState = FakeWebSocket.CLOSED;
  }

  simulateOpen(): void {
    this.readyState = FakeWebSocket.OPEN;
    this.onopen?.({});
  }

  /** Simulate an unexpected (non-clean) drop that drives the reconnect path. */
  simulateUncleanClose(code = 1006): void {
    this.readyState = FakeWebSocket.CLOSED;
    this.onclose?.({ wasClean: false, code });
  }
}

let originalWebSocket: typeof globalThis.WebSocket;

beforeEach(() => {
  vi.useFakeTimers();
  originalWebSocket = globalThis.WebSocket;
  // biome-ignore lint/suspicious/noExplicitAny: test double for the global WebSocket
  globalThis.WebSocket = FakeWebSocket as any;
  FakeWebSocket.lastInstance = null;
  FakeWebSocket.constructed = 0;
});

afterEach(() => {
  vi.useRealTimers();
  globalThis.WebSocket = originalWebSocket;
  vi.restoreAllMocks();
});

/**
 * Simulate the current socket dropping, then let its backoff timer (if any)
 * fire. Advancing well past the max backoff delay (30 s) guarantees any
 * jittered timeout the hook scheduled runs, regardless of the random factor.
 */
function driveOneOutageCycle(): void {
  const socket = FakeWebSocket.lastInstance;
  if (!socket) throw new Error('expected a live socket to drop');
  act(() => {
    socket.simulateUncleanClose();
    vi.advanceTimersByTime(60_000);
  });
}

describe('useIotWebSocket — reconnect policy', () => {
  it('gives up after the attempt budget is exhausted on a continuous outage', () => {
    renderHook(() => useIotWebSocket('sensor-1', 'org-1'));

    // Seed socket is #1. Each cycle reopens until the cap is hit.
    // scheduleReconnect gives up once attempts >= DEFAULT_MAX_RECONNECT_ATTEMPTS,
    // so the loop reopens exactly DEFAULT_MAX_RECONNECT_ATTEMPTS times.
    for (let i = 0; i < DEFAULT_MAX_RECONNECT_ATTEMPTS + 2; i++) {
      driveOneOutageCycle();
    }

    // Fixed-interval-with-no-cap (the bug) would have built one socket per
    // close and kept going; the cap bounds it to seed + budget.
    expect(FakeWebSocket.constructed).toBe(DEFAULT_MAX_RECONNECT_ATTEMPTS + 1);

    // Give-up is terminal: further closes schedule nothing.
    const atGiveUp = FakeWebSocket.constructed;
    driveOneOutageCycle();
    driveOneOutageCycle();
    expect(FakeWebSocket.constructed).toBe(atGiveUp);
  });

  it('does not reconnect at the old fixed 5 s interval (uses sub-5 s backoff first)', () => {
    renderHook(() => useIotWebSocket('sensor-1', 'org-1'));

    const socket = FakeWebSocket.lastInstance;
    if (!socket) throw new Error('expected a live socket');
    act(() => {
      socket.simulateUncleanClose();
    });

    // First-attempt exponential backoff is min(1000 * 2^0, 30000) * [0.5, 1) =
    // < 1000 ms — strictly less than the old fixed 5 s. Advancing 1 s must
    // already have reopened; the old fixed-5s path would not have fired yet.
    const before = FakeWebSocket.constructed;
    act(() => {
      vi.advanceTimersByTime(1_000);
    });
    expect(FakeWebSocket.constructed).toBe(before + 1);
  });

  it('resets the attempt budget after a successful reopen', () => {
    renderHook(() => useIotWebSocket('sensor-1', 'org-1'));

    // Burn most of the budget, then succeed — the reset means a fresh outage
    // gets a full budget again rather than immediately giving up.
    for (let i = 0; i < DEFAULT_MAX_RECONNECT_ATTEMPTS - 1; i++) {
      driveOneOutageCycle();
    }
    act(() => {
      FakeWebSocket.lastInstance?.simulateOpen();
    });

    const afterOpen = FakeWebSocket.constructed;
    // A full fresh budget worth of reconnects must follow before give-up.
    for (let i = 0; i < DEFAULT_MAX_RECONNECT_ATTEMPTS; i++) {
      driveOneOutageCycle();
    }
    expect(FakeWebSocket.constructed).toBe(afterOpen + DEFAULT_MAX_RECONNECT_ATTEMPTS);
  });

  it('exposes hasGivenUp once the reconnect budget is exhausted', () => {
    const { result } = renderHook(() => useIotWebSocket('sensor-1', 'org-1'));

    // The give-up is the UI-facing "offline" contract: the hook returns
    // `hasGivenUp` so a view can stop assuming a silent-but-live socket.
    // Counting constructed sockets (the other tests) can't prove the hook
    // actually told the UI it stopped — a regression that bounds reconnects
    // but forgets `setHasGivenUp(true)` would pass those and fail here.
    expect(result.current.hasGivenUp).toBe(false);

    for (let i = 0; i < DEFAULT_MAX_RECONNECT_ATTEMPTS + 2; i++) {
      driveOneOutageCycle();
    }

    expect(result.current.hasGivenUp).toBe(true);
  });

  it('tracks isConnected across a successful open and a subsequent unclean drop', () => {
    const { result } = renderHook(() => useIotWebSocket('sensor-1', 'org-1'));

    // Constructed but not yet acknowledged by the server — still offline.
    expect(result.current.isConnected).toBe(false);

    act(() => {
      FakeWebSocket.lastInstance?.simulateOpen();
    });
    expect(result.current.isConnected).toBe(true);

    // An unclean drop must flip the flag back to false (before it schedules
    // the backoff reconnect) so the chart can't keep rendering as "live".
    act(() => {
      FakeWebSocket.lastInstance?.simulateUncleanClose();
    });
    expect(result.current.isConnected).toBe(false);
  });

  it('cancels a pending reconnect when the hook unmounts (no closed-tab storm)', () => {
    const { unmount } = renderHook(() => useIotWebSocket('sensor-1', 'org-1'));

    // An unclean close schedules a backoff reconnect timer...
    act(() => {
      FakeWebSocket.lastInstance?.simulateUncleanClose();
    });
    const beforeUnmount = FakeWebSocket.constructed;

    // ...but unmounting (tab closed / route left) must cancel it so the timer
    // never reopens a socket. The pre-fix reconnect storm came precisely from
    // reconnect timers that outlived the component that scheduled them.
    unmount();
    act(() => {
      vi.advanceTimersByTime(60_000);
    });
    expect(FakeWebSocket.constructed).toBe(beforeUnmount);
  });
});
