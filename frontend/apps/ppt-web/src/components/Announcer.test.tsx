/**
 * Announcer tests — stale-message resurrection regression.
 *
 * Bug: the deferred "set" timeout (clear-then-set, +100ms) was not tracked in a
 * ref, so a rapid second announce() left the first announce's deferred set
 * pending. That pending set then fired AFTER the newer message had been shown,
 * resurrecting the stale message in the aria-live region.
 *
 * Fix: track the deferred set timeout per channel and clear the pending one
 * (alongside the clear timeout) before scheduling a new set.
 */
/// <reference types="vitest/globals" />

import { act, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { AnnouncerProvider, useAnnouncer } from './Announcer';

let announce: ReturnType<typeof useAnnouncer>['announce'];

function Capture() {
  announce = useAnnouncer().announce;
  return null;
}

function renderAnnouncer() {
  return render(
    <AnnouncerProvider>
      <Capture />
    </AnnouncerProvider>
  );
}

const polite = () => screen.getByRole('status');

describe('AnnouncerProvider', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.runOnlyPendingTimers();
    vi.useRealTimers();
  });

  it('does not resurrect a stale message when announce() is called rapidly', () => {
    renderAnnouncer();

    // First announce at t=0 schedules its deferred set at t=100.
    act(() => {
      announce('first message');
    });

    // Second announce at t=50 (within the 100ms window) schedules its deferred
    // set at t=150. In the buggy code the first deferred set (t=100) was left
    // untracked, so it still fires at t=100 and flashes the stale message into
    // the live region before the second message lands — a stale announcement.
    act(() => {
      vi.advanceTimersByTime(50);
    });
    act(() => {
      announce('second message');
    });

    // Advance to t=100: only the (cancelled-in-fix) first deferred set is due.
    act(() => {
      vi.advanceTimersByTime(50);
    });

    // With the fix the stale "first message" set was cancelled, so the live
    // region must never have shown it here.
    expect(polite().textContent).not.toBe('first message');

    // Advance to t=150: the second deferred set fires.
    act(() => {
      vi.advanceTimersByTime(50);
    });
    expect(polite().textContent).toBe('second message');
  });

  it('shows the latest message and clears it after the announce window', () => {
    renderAnnouncer();

    act(() => {
      announce('hello');
    });
    act(() => {
      vi.advanceTimersByTime(100);
    });
    expect(polite().textContent).toBe('hello');

    // After the 3000ms clear timeout the region empties.
    act(() => {
      vi.advanceTimersByTime(3000);
    });
    expect(polite().textContent).toBe('');
  });

  it('keeps polite and assertive channels independent', () => {
    renderAnnouncer();

    act(() => {
      announce('be polite', 'polite');
      announce('urgent!', 'assertive');
    });
    act(() => {
      vi.advanceTimersByTime(100);
    });

    expect(screen.getByRole('status').textContent).toBe('be polite');
    expect(screen.getByRole('alert').textContent).toBe('urgent!');
  });
});
