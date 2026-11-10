/**
 * Tests for usePerformanceMetrics.
 *
 * Focus: event-listener lifecycle. The hook registers `visibilitychange`
 * (for unload reporting) and `load` (for navigation timing) listeners; both
 * must be removed on cleanup, and the effect must NOT re-subscribe when
 * `onReport` is passed inline (a fresh function reference every render).
 */

import { renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { usePerformanceMetrics } from './usePerformanceMetrics';

// Minimal PerformanceObserver stub so the effect body runs under jsdom
// (the real effect early-returns when PerformanceObserver is undefined).
class FakePerformanceObserver {
  observe() {}
  disconnect() {}
}

function countListeners(spy: ReturnType<typeof vi.spyOn>, event: string): number {
  return spy.mock.calls.filter((call) => call[0] === event).length;
}

describe('usePerformanceMetrics', () => {
  let addSpy: ReturnType<typeof vi.spyOn>;
  let removeSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    vi.stubGlobal('PerformanceObserver', FakePerformanceObserver);
    addSpy = vi.spyOn(window, 'addEventListener');
    removeSpy = vi.spyOn(window, 'removeEventListener');
  });

  afterEach(() => {
    addSpy.mockRestore();
    removeSpy.mockRestore();
    vi.unstubAllGlobals();
  });

  it('removes the visibilitychange listener on unmount', () => {
    const { unmount } = renderHook(() => usePerformanceMetrics(() => {}));

    expect(countListeners(addSpy, 'visibilitychange')).toBe(1);
    expect(countListeners(removeSpy, 'visibilitychange')).toBe(0);

    unmount();

    expect(countListeners(removeSpy, 'visibilitychange')).toBe(1);
  });

  it('does not register visibilitychange when reportOnUnload is false', () => {
    const { unmount } = renderHook(() =>
      usePerformanceMetrics(() => {}, { reportOnUnload: false })
    );

    expect(countListeners(addSpy, 'visibilitychange')).toBe(0);

    unmount();

    expect(countListeners(removeSpy, 'visibilitychange')).toBe(0);
  });

  it('adds and later removes the load listener when the document is still loading', () => {
    const readyStateSpy = vi.spyOn(document, 'readyState', 'get').mockReturnValue('loading');

    const { unmount } = renderHook(() => usePerformanceMetrics(() => {}));

    expect(countListeners(addSpy, 'load')).toBe(1);
    expect(countListeners(removeSpy, 'load')).toBe(0);

    unmount();

    expect(countListeners(removeSpy, 'load')).toBe(1);
    readyStateSpy.mockRestore();
  });

  it('does not re-subscribe listeners when onReport is a new inline function each render', () => {
    // Simulate the common misuse: an inline callback recreated every render.
    const { rerender, unmount } = renderHook(({ n }) => usePerformanceMetrics(() => n), {
      initialProps: { n: 0 },
    });

    rerender({ n: 1 });
    rerender({ n: 2 });
    rerender({ n: 3 });

    // Despite four renders, the visibilitychange listener is registered once
    // (the effect depends only on reportOnUnload, and onReport lives in a ref).
    expect(countListeners(addSpy, 'visibilitychange')).toBe(1);
    expect(countListeners(removeSpy, 'visibilitychange')).toBe(0);

    unmount();
    expect(countListeners(removeSpy, 'visibilitychange')).toBe(1);
  });

  it('invokes the latest onReport (not a stale one) when the page is hidden', () => {
    const first = vi.fn();
    const second = vi.fn();

    const { result, rerender } = renderHook(({ cb }) => usePerformanceMetrics(cb), {
      initialProps: { cb: first },
    });

    // Seed a metric so reportMetrics has something to send (it no-ops on an
    // empty metrics object). The hook returns its live metrics ref object.
    result.current.fcp = 123;

    // Re-render with a different callback; the effect must not re-subscribe,
    // but the ref must now point at the newest callback.
    rerender({ cb: second });

    vi.spyOn(document, 'visibilityState', 'get').mockReturnValue('hidden');
    window.dispatchEvent(new Event('visibilitychange'));

    expect(first).not.toHaveBeenCalled();
    expect(second).toHaveBeenCalledTimes(1);
    expect(second).toHaveBeenCalledWith(expect.objectContaining({ fcp: 123 }));
  });
});
