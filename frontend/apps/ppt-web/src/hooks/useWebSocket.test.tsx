/// <reference types="vitest/globals" />
/**
 * Regression tests for the {@link useWebSocket} subscription gate.
 *
 * The subscription effect intentionally stores the handler in a ref so that
 * swapping one handler function for another does NOT tear down and re-create
 * the server subscription (no resubscribe churn). But the original effect
 * gated on `handlerRef.current` while depending only on `[eventType, subscribe]`.
 * That meant a handler supplied *after* mount (e.g. a handler that is `undefined`
 * on the first render and only becomes defined once some async state resolves)
 * never caused `subscribe()` to run — the effect had already early-returned and
 * nothing in its dep array changed. These tests pin the corrected behaviour:
 *
 *  1. a late-arriving handler triggers `subscribe()`,
 *  2. clearing the handler unsubscribes,
 *  3. swapping handler identity while a handler stays present does NOT resubscribe.
 */

import { renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { MessageHandler } from '../lib/websocket';
import { useWebSocket } from './useWebSocket';

const unsubscribe = vi.fn();
const subscribe = vi.fn<(eventType: string, handler: MessageHandler) => () => void>(
  () => unsubscribe
);

vi.mock('../contexts/WebSocketContext', () => ({
  useWebSocketContext: () => ({
    isConnected: true,
    isConnecting: false,
    error: null,
    subscribe,
    send: vi.fn(() => true),
    reconnect: vi.fn(),
  }),
}));

// Clear at the start of each test (not after). @testing-library's auto-cleanup
// unmounts the previous test's hook during afterEach, which can call
// `unsubscribe` *after* an afterEach-based mockClear and leak into the next
// test's counts; clearing in beforeEach sidesteps that ordering entirely.
beforeEach(() => {
  subscribe.mockClear();
  unsubscribe.mockClear();
});

describe('useWebSocket subscription gate', () => {
  it('subscribes when a handler is supplied after mount', () => {
    const { rerender } = renderHook(
      ({ handler }: { handler?: MessageHandler }) => useWebSocket('message:new', handler),
      { initialProps: { handler: undefined as MessageHandler | undefined } }
    );

    // No handler at mount → no subscription yet.
    expect(subscribe).not.toHaveBeenCalled();

    // Handler arrives after mount → the effect must now subscribe.
    const handler: MessageHandler = vi.fn();
    rerender({ handler });

    expect(subscribe).toHaveBeenCalledTimes(1);
    expect(subscribe).toHaveBeenCalledWith('message:new', expect.any(Function));
  });

  it('unsubscribes when the handler is cleared after mount', () => {
    const handler: MessageHandler = vi.fn();
    const { rerender } = renderHook(
      ({ h }: { h?: MessageHandler }) => useWebSocket('message:new', h),
      { initialProps: { h: handler as MessageHandler | undefined } }
    );

    expect(subscribe).toHaveBeenCalledTimes(1);

    // Handler goes away → the effect cleanup must run.
    rerender({ h: undefined });

    expect(unsubscribe).toHaveBeenCalledTimes(1);
  });

  it('does not resubscribe when the handler identity changes but stays present', () => {
    const first: MessageHandler = vi.fn();
    const { rerender } = renderHook(
      ({ h }: { h: MessageHandler }) => useWebSocket('message:new', h),
      { initialProps: { h: first } }
    );

    expect(subscribe).toHaveBeenCalledTimes(1);

    // Swap the handler function — presence unchanged, so no churn.
    const second: MessageHandler = vi.fn();
    rerender({ h: second });

    expect(subscribe).toHaveBeenCalledTimes(1);
    expect(unsubscribe).not.toHaveBeenCalled();
  });

  it('routes delivered messages to the latest handler after an identity swap', () => {
    const first = vi.fn();
    const second = vi.fn();
    const { rerender } = renderHook(
      ({ h }: { h: MessageHandler }) => useWebSocket('message:new', h),
      { initialProps: { h: first as MessageHandler } }
    );

    // The wrapped handler captured by subscribe() should delegate to the ref.
    const wrapped = subscribe.mock.calls[0][1];
    rerender({ h: second });

    const message = { type: 'message:new', payload: {}, timestamp: 'now' };
    wrapped(message);

    expect(first).not.toHaveBeenCalled();
    expect(second).toHaveBeenCalledWith(message);
  });
});
