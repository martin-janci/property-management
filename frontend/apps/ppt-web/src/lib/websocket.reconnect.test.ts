/// <reference types="vitest/globals" />
/**
 * Reconnect-budget / resume-after-give-up regression tests.
 *
 * Bug: once `reconnectAttempts` reaches `maxReconnectAttempts`,
 * `scheduleReconnect()` flips `shouldReconnect` off and gives up permanently.
 * Nothing re-armed reconnection — `onclose` only reconnects while
 * `shouldReconnect` is true, and `reconnectAttempts` only reset on a
 * successful `onopen`. The default backoff budget is only ~3 min, so any
 * longer outage (sleep/resume, VPN drop, redeploy) killed the notification
 * socket for the rest of the session.
 *
 * Fix: an explicit `connect()` resets the attempt budget
 * (`resetReconnectAttempts()`) and re-opens, so a resume after give-up gets a
 * full set of attempts again. The internal backoff loop keeps calling the
 * private `openConnection()` (never `connect()`), so a mid-backoff retry does
 * NOT reset the budget — the cap can still be reached on a continuous outage.
 *
 * These tests install a controllable fake `WebSocket` and drive the backoff
 * with fake timers to assert both halves of that contract.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { WebSocketService } from './websocket';

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
  sent: string[] = [];

  constructor(
    public url: string,
    public protocols?: string | string[]
  ) {
    FakeWebSocket.lastInstance = this;
    FakeWebSocket.constructed += 1;
  }

  send(data: string): void {
    this.sent.push(data);
  }

  close(): void {
    this.readyState = FakeWebSocket.CLOSED;
  }

  /** Simulate an unexpected (non-clean) drop that drives the reconnect path. */
  simulateUncleanClose(code = 1006): void {
    this.readyState = FakeWebSocket.CLOSED;
    this.onclose?.({ wasClean: false, code });
  }
}

function makeService(maxReconnectAttempts: number): WebSocketService {
  return new WebSocketService({
    url: 'ws://localhost:8080',
    getToken: () => 'test-jwt',
    minReconnectDelay: 1,
    maxReconnectDelay: 50,
    maxReconnectAttempts,
  });
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

/** Simulate the current socket dropping, then let its backoff timer fire. */
function driveOneOutageCycle(): void {
  const socket = FakeWebSocket.lastInstance;
  if (!socket) throw new Error('expected a live socket to drop');
  socket.simulateUncleanClose();
  // Fire whatever reconnect timeout scheduleReconnect() queued (if any).
  vi.advanceTimersByTime(1000);
}

describe('WebSocketService — reconnect budget & resume after give-up', () => {
  it('gives up after the attempt budget is exhausted on a continuous outage', () => {
    const service = makeService(2);
    const maxRetries = vi.fn();
    service.subscribe('connection:max-retries-exceeded', maxRetries);

    service.connect(); // attempt seed — socket #1

    // Each cycle: unclean close -> scheduleReconnect -> timer fires -> reopen.
    // With max=2: attempts 0->1 (reopen), 1->2 (reopen), then 2>=2 -> give up.
    driveOneOutageCycle(); // socket #2
    driveOneOutageCycle(); // socket #3
    driveOneOutageCycle(); // give up here

    expect(maxRetries).toHaveBeenCalledTimes(1);
    expect(service.getConnectionState()).toBe('error');

    // The give-up is terminal for the backoff loop: a further close does not
    // schedule another reconnect (shouldReconnect was flipped off).
    const constructedAtGiveUp = FakeWebSocket.constructed;
    vi.advanceTimersByTime(5000);
    expect(FakeWebSocket.constructed).toBe(constructedAtGiveUp);
  });

  it('resumes reconnection when connect() is called again after giving up', () => {
    const service = makeService(2);
    const maxRetries = vi.fn();
    service.subscribe('connection:max-retries-exceeded', maxRetries);

    service.connect();
    driveOneOutageCycle();
    driveOneOutageCycle();
    driveOneOutageCycle(); // budget exhausted -> gave up

    expect(maxRetries).toHaveBeenCalledTimes(1);
    const giveUpSocket = FakeWebSocket.lastInstance;
    const constructedAtGiveUp = FakeWebSocket.constructed;

    // Resume: an explicit connect() must reset the budget and open a fresh
    // socket (this is the online-event / manual-reconnect entry point).
    service.connect();
    expect(FakeWebSocket.constructed).toBe(constructedAtGiveUp + 1);
    expect(FakeWebSocket.lastInstance).not.toBe(giveUpSocket);
    expect(service.getConnectionState()).toBe('connecting');

    // And because the budget was reset, the very next unclean close must NOT
    // immediately re-trigger give-up: it schedules another retry instead.
    // (Without the reset, attempts would still be at the cap and this would
    // fire max-retries a second time.)
    driveOneOutageCycle();
    expect(maxRetries).toHaveBeenCalledTimes(1);
    expect(FakeWebSocket.constructed).toBeGreaterThan(constructedAtGiveUp + 1);
  });

  it('resetReconnectAttempts() lets a fresh backoff run reach the cap again', () => {
    const service = makeService(1);
    const maxRetries = vi.fn();
    service.subscribe('connection:max-retries-exceeded', maxRetries);

    service.connect();
    driveOneOutageCycle(); // attempts 0->1 reopen
    driveOneOutageCycle(); // 1>=1 -> give up (1st)
    expect(maxRetries).toHaveBeenCalledTimes(1);

    service.resetReconnectAttempts();
    // Re-arm the loop the way connect() would, then exhaust it again.
    service.connect();
    driveOneOutageCycle(); // attempts 0->1 reopen
    driveOneOutageCycle(); // 1>=1 -> give up (2nd)
    expect(maxRetries).toHaveBeenCalledTimes(2);
  });
});
