/// <reference types="vitest/globals" />
/**
 * useDraftStorage tests (Epic 80, Story 80.2 — AC-4 draft auto-save).
 *
 * Covers: debounced persist, synchronous restore on mount, savedAt timestamp,
 * clear(), and the private-mode (localStorage unavailable) no-op path.
 */

import { act, renderHook } from '@testing-library/react';
import { useDraftStorage } from './useDraftStorage';

const KEY = 'test-draft-key';

describe('useDraftStorage', () => {
  beforeEach(() => {
    localStorage.clear();
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.runOnlyPendingTimers();
    vi.useRealTimers();
  });

  it('persists values to localStorage after the debounce window', () => {
    const { result } = renderHook(() => useDraftStorage<{ subject: string }>(KEY));

    expect(result.current.savedAt).toBeNull();
    act(() => result.current.save({ subject: 'hello' }));

    // Not yet flushed (debounce pending).
    expect(localStorage.getItem(KEY)).toBeNull();

    act(() => vi.advanceTimersByTime(800));

    const raw = localStorage.getItem(KEY);
    expect(raw).not.toBeNull();
    const parsed = JSON.parse(raw as string);
    expect(parsed.values).toEqual({ subject: 'hello' });
    expect(typeof parsed.savedAt).toBe('number');
    expect(result.current.savedAt).toBe(parsed.savedAt);
  });

  it('debounces rapid saves into a single flush of the latest value', () => {
    const { result } = renderHook(() => useDraftStorage<{ subject: string }>(KEY));

    act(() => {
      result.current.save({ subject: 'a' });
      result.current.save({ subject: 'ab' });
      result.current.save({ subject: 'abc' });
    });
    act(() => vi.advanceTimersByTime(800));

    const parsed = JSON.parse(localStorage.getItem(KEY) as string);
    expect(parsed.values).toEqual({ subject: 'abc' });
  });

  it('restores a previously-stored draft synchronously on mount', () => {
    localStorage.setItem(KEY, JSON.stringify({ values: { subject: 'restored' }, savedAt: 123456 }));

    const { result } = renderHook(() => useDraftStorage<{ subject: string }>(KEY));

    expect(result.current.restored).toEqual({ subject: 'restored' });
    expect(result.current.savedAt).toBe(123456);
  });

  it('returns null restored value when no draft exists', () => {
    const { result } = renderHook(() => useDraftStorage<{ subject: string }>(KEY));
    expect(result.current.restored).toBeNull();
    expect(result.current.savedAt).toBeNull();
  });

  it('clear() removes the stored draft and resets savedAt', () => {
    const { result } = renderHook(() => useDraftStorage<{ subject: string }>(KEY));

    act(() => result.current.save({ subject: 'x' }));
    act(() => vi.advanceTimersByTime(800));
    expect(localStorage.getItem(KEY)).not.toBeNull();

    act(() => result.current.clear());
    expect(localStorage.getItem(KEY)).toBeNull();
    expect(result.current.savedAt).toBeNull();
  });

  it('tolerates corrupt stored JSON without throwing', () => {
    localStorage.setItem(KEY, '{ not json');
    const { result } = renderHook(() => useDraftStorage<{ subject: string }>(KEY));
    expect(result.current.restored).toBeNull();
  });

  // ── #1364: unmount must flush a still-pending debounced save ──
  // The hook advertises protection against "accidental tab close / navigation".
  // For the in-app (SPA) navigation case that means: if the user types and the
  // component unmounts before the 800ms debounce fires, the last keystrokes
  // must still be persisted via the synchronous unmount flush — not silently
  // dropped (the bug originally shipped in PR #1359).
  it('flushes a pending debounced save synchronously on unmount', () => {
    const { result, unmount } = renderHook(() => useDraftStorage<{ subject: string }>(KEY));

    act(() => result.current.save({ subject: 'typed-then-navigated' }));

    // Debounce has NOT fired yet — nothing persisted.
    expect(localStorage.getItem(KEY)).toBeNull();

    // Simulate SPA navigation away from the form before the timer fires.
    act(() => unmount());

    const raw = localStorage.getItem(KEY);
    expect(raw, 'pending draft was dropped on unmount').not.toBeNull();
    const parsed = JSON.parse(raw as string);
    expect(parsed.values).toEqual({ subject: 'typed-then-navigated' });
    expect(typeof parsed.savedAt).toBe('number');
  });

  it('does not write on unmount when nothing is pending', () => {
    const { unmount } = renderHook(() => useDraftStorage<{ subject: string }>(KEY));
    act(() => unmount());
    expect(localStorage.getItem(KEY)).toBeNull();
  });

  it('does not re-flush an already-persisted draft on unmount', () => {
    const { result, unmount } = renderHook(() => useDraftStorage<{ subject: string }>(KEY));

    act(() => result.current.save({ subject: 'flushed' }));
    act(() => vi.advanceTimersByTime(800));
    const afterFlush = localStorage.getItem(KEY);
    expect(afterFlush).not.toBeNull();

    // Nothing pending now — unmount must be a no-op (pendingRef was cleared by
    // the flush), so the stored value is byte-for-byte unchanged.
    act(() => unmount());
    expect(localStorage.getItem(KEY)).toBe(afterFlush);
  });
});
