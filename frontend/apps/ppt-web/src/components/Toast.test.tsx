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
import { act, render, screen, waitFor } from '@testing-library/react';
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
