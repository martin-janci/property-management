/**
 * Toast Component Tests (Epic 80, Story 80.3)
 *
 * Tests for the Toast notification system including:
 * - Rendering different toast types
 * - Toast provider functionality
 * - Auto-dismiss behavior
 * - User interactions
 */

/// <reference types="vitest/globals" />
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ToastProvider, useToast } from './Toast';

// Test component that uses the toast hook
function TestToastTrigger({
  type,
  title,
  message,
}: {
  type: 'success' | 'error' | 'info' | 'warning';
  title: string;
  message?: string;
}) {
  const { showToast } = useToast();
  return (
    <button type="button" onClick={() => showToast({ type, title, message })}>
      Show Toast
    </button>
  );
}

describe('Toast Component', () => {
  it('renders success toast correctly', async () => {
    const user = userEvent.setup();

    render(
      <ToastProvider>
        <TestToastTrigger type="success" title="Success!" message="Operation completed" />
      </ToastProvider>
    );

    await user.click(screen.getByText('Show Toast'));

    expect(screen.getByText('Success!')).toBeInTheDocument();
    expect(screen.getByText('Operation completed')).toBeInTheDocument();
  });

  it('renders error toast correctly', async () => {
    const user = userEvent.setup();

    render(
      <ToastProvider>
        <TestToastTrigger type="error" title="Error!" message="Something went wrong" />
      </ToastProvider>
    );

    await user.click(screen.getByText('Show Toast'));

    expect(screen.getByText('Error!')).toBeInTheDocument();
    expect(screen.getByText('Something went wrong')).toBeInTheDocument();
  });

  it('renders warning toast correctly', async () => {
    const user = userEvent.setup();

    render(
      <ToastProvider>
        <TestToastTrigger type="warning" title="Warning!" />
      </ToastProvider>
    );

    await user.click(screen.getByText('Show Toast'));

    expect(screen.getByText('Warning!')).toBeInTheDocument();
  });

  it('renders info toast correctly', async () => {
    const user = userEvent.setup();

    render(
      <ToastProvider>
        <TestToastTrigger type="info" title="Info" />
      </ToastProvider>
    );

    await user.click(screen.getByText('Show Toast'));

    expect(screen.getByText('Info')).toBeInTheDocument();
  });

  it('dismisses toast when close button is clicked', async () => {
    const user = userEvent.setup();

    render(
      <ToastProvider>
        <TestToastTrigger type="info" title="Dismissable Toast" />
      </ToastProvider>
    );

    await user.click(screen.getByText('Show Toast'));
    expect(screen.getByText('Dismissable Toast')).toBeInTheDocument();

    await user.click(screen.getByLabelText('Dismiss notification'));

    await waitFor(() => {
      expect(screen.queryByText('Dismissable Toast')).not.toBeInTheDocument();
    });
  });

  // Helper that exposes the showToast function so the auto-dismiss tests can
  // trigger a toast without going through userEvent — userEvent's internal
  // awaits stall when combined with fake timers in this setup.
  function ToastEmitter({
    onReady,
  }: {
    onReady: (showToast: ReturnType<typeof useToast>['showToast']) => void;
  }) {
    const { showToast } = useToast();
    onReady(showToast);
    return null;
  }

  it('auto-dismisses success toast after duration', async () => {
    // Success toasts auto-dismiss after DEFAULT_SUCCESS_DURATION (5000ms).
    vi.useFakeTimers();
    try {
      let trigger: ReturnType<typeof useToast>['showToast'] | null = null;

      render(
        <ToastProvider>
          <ToastEmitter
            onReady={(showToast) => {
              trigger = showToast;
            }}
          />
        </ToastProvider>
      );

      await act(async () => {
        trigger?.({ type: 'success', title: 'Auto-dismiss me' });
      });
      expect(screen.getByText('Auto-dismiss me')).toBeInTheDocument();

      // Advance past the 5000ms success duration; flush React state updates.
      await act(async () => {
        vi.advanceTimersByTime(5001);
      });

      expect(screen.queryByText('Auto-dismiss me')).not.toBeInTheDocument();
    } finally {
      vi.useRealTimers();
    }
  });

  it('clears the "copied" timer when the toast unmounts (no setState after unmount)', async () => {
    // Regression: ToastItem scheduled setTimeout(() => setCopied(false), 2000)
    // on copy but never cleared it. Dismissing the toast within that 2s window
    // left a dangling timer that fired setState on an unmounted component.
    // After the fix, unmounting must leave no pending copy timer behind.
    vi.useFakeTimers();
    const writeText = vi.fn().mockResolvedValue(undefined);
    const originalClipboard = navigator.clipboard;
    Object.defineProperty(navigator, 'clipboard', {
      value: { writeText },
      configurable: true,
    });
    try {
      let trigger: ReturnType<typeof useToast>['showToast'] | null = null;

      render(
        <ToastProvider>
          <ToastEmitter
            onReady={(showToast) => {
              trigger = showToast;
            }}
          />
        </ToastProvider>
      );

      // Error toast => duration 0 => no provider auto-dismiss timer interferes.
      await act(async () => {
        trigger?.({ type: 'error', title: 'Copy me' });
      });

      const before = vi.getTimerCount();

      // Click copy; flush the async clipboard write + setCopied(true).
      await act(async () => {
        fireEvent.click(screen.getByLabelText('Copy error message'));
      });
      expect(writeText).toHaveBeenCalled();

      // The copy handler scheduled exactly one additional 2s timer.
      expect(vi.getTimerCount()).toBe(before + 1);

      // Dismiss the toast *before* the 2s timer elapses -> ToastItem unmounts.
      await act(async () => {
        fireEvent.click(screen.getByLabelText('Dismiss notification'));
      });
      expect(screen.queryByText('Copy me')).not.toBeInTheDocument();

      // Unmount must clear the copy timer. Without the fix the count stays at
      // `before + 1` and that timer later fires setCopied on an unmounted node.
      expect(vi.getTimerCount()).toBe(before);
    } finally {
      Object.defineProperty(navigator, 'clipboard', {
        value: originalClipboard,
        configurable: true,
      });
      vi.useRealTimers();
    }
  });

  it('bounds the persistent-toast queue — evicted toasts do not reappear when visible ones are dismissed', async () => {
    // Regression (code-review: persistent-toast overflow leak).
    //
    // Error toasts are persistent (duration = 0): they have no auto-dismiss
    // timer and only the 3 most-recent toasts render a close button. Before
    // the fix, showToast retained every toast in state and only *sliced* the
    // render to the last 3. Flooding >3 persistent errors therefore left the
    // overflow toasts stuck in state forever (a memory/DOM-state leak) — and,
    // observably, dismissing the visible 3 "revealed" the previously-hidden
    // older toasts instead of clearing the stack. After the fix the queue is
    // bounded to MAX_VISIBLE_TOASTS, so dismissing the visible toasts empties
    // the region.
    let trigger: ReturnType<typeof useToast>['showToast'] | null = null;

    const { container } = render(
      <ToastProvider>
        <ToastEmitter
          onReady={(showToast) => {
            trigger = showToast;
          }}
        />
      </ToastProvider>
    );

    // Flood six persistent error toasts (> MAX_VISIBLE_TOASTS = 3).
    await act(async () => {
      for (let i = 1; i <= 6; i++) {
        trigger?.({ type: 'error', title: `Flood error ${i}` });
      }
    });

    // Only the three most-recent are rendered; the three oldest were evicted.
    expect(screen.getAllByRole('alert')).toHaveLength(3);
    expect(screen.getByText('Flood error 6')).toBeInTheDocument();
    expect(screen.queryByText('Flood error 1')).not.toBeInTheDocument();

    // Dismiss every currently-visible toast.
    const dismissButtons = screen.getAllByLabelText('Dismiss notification');
    expect(dismissButtons).toHaveLength(3);
    for (const button of dismissButtons) {
      await act(async () => {
        fireEvent.click(button);
      });
    }

    // Nothing should remain. Before the fix the three evicted-but-retained
    // toasts would resurface here and keep the region non-empty.
    expect(screen.queryAllByRole('alert')).toHaveLength(0);
    expect(container.querySelector('.toast')).toBeNull();
  });

  it('does not leak auto-dismiss timers for toasts evicted past the visible window', async () => {
    // Companion to the persistent-toast bound: timed toasts (duration > 0)
    // register a setTimeout each. Before the fix, flooding >3 timed toasts
    // kept all of them — and all their pending timers — alive in the timeout
    // map even though only 3 rendered. After the fix, evicting the overflow
    // toasts also clears their timers, so no more than MAX_VISIBLE_TOASTS
    // auto-dismiss timers are ever outstanding.
    vi.useFakeTimers();
    try {
      let trigger: ReturnType<typeof useToast>['showToast'] | null = null;

      render(
        <ToastProvider>
          <ToastEmitter
            onReady={(showToast) => {
              trigger = showToast;
            }}
          />
        </ToastProvider>
      );

      const before = vi.getTimerCount();

      // Ten success toasts, each with a 5000ms auto-dismiss timer.
      await act(async () => {
        for (let i = 1; i <= 10; i++) {
          trigger?.({ type: 'success', title: `Timed ${i}` });
        }
      });

      // At most MAX_VISIBLE_TOASTS (3) pending auto-dismiss timers remain —
      // not one per emitted toast. Pre-fix this was `before + 10`.
      expect(vi.getTimerCount()).toBeLessThanOrEqual(before + 3);
    } finally {
      vi.useRealTimers();
    }
  });

  it('error toast persists (no auto-dismiss)', async () => {
    // Error toasts have duration = 0, meaning no auto-dismiss timer is set.
    vi.useFakeTimers();
    try {
      let trigger: ReturnType<typeof useToast>['showToast'] | null = null;

      render(
        <ToastProvider>
          <ToastEmitter
            onReady={(showToast) => {
              trigger = showToast;
            }}
          />
        </ToastProvider>
      );

      await act(async () => {
        trigger?.({ type: 'error', title: 'Sticky error' });
      });
      expect(screen.getByText('Sticky error')).toBeInTheDocument();

      // Advance well past any plausible auto-dismiss window.
      await act(async () => {
        vi.advanceTimersByTime(30_000);
      });

      // Error toast should still be present.
      expect(screen.getByText('Sticky error')).toBeInTheDocument();
    } finally {
      vi.useRealTimers();
    }
  });
});

describe('useToast Hook', () => {
  it('throws error when used outside ToastProvider', () => {
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});

    const ThrowingComponent = () => {
      useToast();
      return null;
    };

    expect(() => render(<ThrowingComponent />)).toThrow(
      'useToast must be used within a ToastProvider'
    );

    consoleError.mockRestore();
  });
});
