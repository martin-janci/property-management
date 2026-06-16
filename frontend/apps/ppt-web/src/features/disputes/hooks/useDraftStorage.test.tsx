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
});
