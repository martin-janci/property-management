/**
 * Regression tests for the "max comparison" warning timer in CompareButton
 * (reality-web).
 *
 * Bug (`code-review-reality-web-comparison-timer-leak`): when the comparison
 * set is full, clicking Compare schedules
 *
 *     setTimeout(() => setShowMaxWarning(false), 3000)
 *
 * to auto-hide the warning — but the handle was never retained or cleared. If
 * the button unmounted within the 3s window the timer still fired, calling
 * setState on an unmounted component; and rapid re-clicks stacked independent
 * timers. The fix stores the handle in a ref, clears a pending timer before
 * re-arming, and clears it on unmount.
 */
/// <reference types="vitest/globals" />

import type { ListingSummary } from '@ppt/reality-api-client';
import { act, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

// Isolate the button from the real ComparisonProvider — we only need to drive
// the "comparison is full" branch that arms the warning timer.
const mockAddToComparison = vi.fn();
const mockRemoveFromComparison = vi.fn();
vi.mock('../../lib/comparison-context', () => ({
  useComparison: () => ({
    isInComparison: () => false,
    addToComparison: mockAddToComparison,
    removeFromComparison: mockRemoveFromComparison,
    canAddMore: false, // full → clicking Compare arms the auto-hide timer
  }),
}));

import { CompareButton } from './CompareButton';

const listing = { id: 'l1', title: 'Listing 1' } as unknown as ListingSummary;

describe('CompareButton warning timer', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    mockAddToComparison.mockClear();
    mockRemoveFromComparison.mockClear();
  });

  afterEach(() => {
    vi.runOnlyPendingTimers();
    vi.useRealTimers();
  });

  it('clears the pending auto-hide timer on unmount', () => {
    const clearSpy = vi.spyOn(globalThis, 'clearTimeout');
    const { unmount } = render(<CompareButton listing={listing} />);

    // Arm the warning timer.
    fireEvent.click(screen.getByRole('button'));
    expect(screen.getByRole('alert')).toBeInTheDocument();

    // Unmount before the 3s window elapses — cleanup must clear the timer.
    unmount();
    expect(clearSpy).toHaveBeenCalled();

    // Advancing past the window must not throw / fire a stray setState.
    expect(() => act(() => vi.advanceTimersByTime(3000))).not.toThrow();
    clearSpy.mockRestore();
  });

  it('clears a still-pending timer before re-arming on a second click', () => {
    const clearSpy = vi.spyOn(globalThis, 'clearTimeout');
    render(<CompareButton listing={listing} />);
    const button = screen.getByRole('button');

    fireEvent.click(button); // arms timer #1
    clearSpy.mockClear();
    fireEvent.click(button); // must clear #1 before arming #2

    expect(clearSpy).toHaveBeenCalled();
    clearSpy.mockRestore();
  });

  it('auto-hides the warning after the timeout elapses', () => {
    render(<CompareButton listing={listing} />);

    fireEvent.click(screen.getByRole('button'));
    expect(screen.getByRole('alert')).toBeInTheDocument();

    act(() => {
      vi.advanceTimersByTime(3000);
    });
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });
});
