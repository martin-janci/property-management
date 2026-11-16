/**
 * Tests for useNetworkStatus — browser-API guarding.
 *
 * Focus: the hook and its `isOnline()` utility must not crash when the
 * browser globals they probe (`navigator`, `window`) are absent — SSR,
 * web-workers, or any non-DOM/unsupported environment. The client snapshot
 * fed to `useSyncExternalStore` (`getOnlineSnapshot`) and the event
 * subscription (`subscribeToNetworkStatus`) are the risky paths: they read
 * `navigator.onLine` / `window.addEventListener` directly. When the global
 * is missing the hook must fall back to the optimistic "online" default
 * rather than throw.
 */

import { renderHook } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { isOnline, useNetworkStatus } from './useNetworkStatus';

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('useNetworkStatus — browser-API guarding', () => {
  it('does not throw and reports online when navigator is absent', () => {
    // Simulate a non-browser / unsupported env where `navigator` is missing.
    vi.stubGlobal('navigator', undefined);

    const { result } = renderHook(() => useNetworkStatus());

    // Optimistic fallback: with no navigator we assume connectivity.
    expect(result.current.isOnline).toBe(true);
  });

  it('exposes the real navigator.onLine value when available', () => {
    vi.stubGlobal('navigator', { onLine: false });

    const { result } = renderHook(() => useNetworkStatus());

    expect(result.current.isOnline).toBe(false);
  });
});

describe('isOnline() utility — browser-API guarding', () => {
  it('falls back to true when navigator is absent', () => {
    vi.stubGlobal('navigator', undefined);

    expect(isOnline()).toBe(true);
  });

  it('reflects navigator.onLine when available', () => {
    vi.stubGlobal('navigator', { onLine: false });

    expect(isOnline()).toBe(false);
  });
});
