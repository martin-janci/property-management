/// <reference types="vitest/globals" />
/**
 * Toast Component Accessibility Tests (Epic 125, Story 125.1)
 *
 * Tests WCAG 2.1 AA compliance for the Toast notification system including:
 * - Role and ARIA attributes
 * - Keyboard accessibility
 * - Color contrast (via axe-core)
 */

import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { axe } from 'vitest-axe';
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

describe('Toast Accessibility', () => {
  it('should have no accessibility violations when rendered', async () => {
    const { container } = render(
      <ToastProvider>
        <TestToastTrigger type="success" title="Success!" message="Operation completed" />
      </ToastProvider>
    );

    const results = await axe(container);
    expect(results).toHaveNoViolations();
  });

  it('should have no accessibility violations when toast is visible', async () => {
    const user = userEvent.setup();

    const { container } = render(
      <ToastProvider>
        <TestToastTrigger type="success" title="Success!" message="Operation completed" />
      </ToastProvider>
    );

    await user.click(screen.getByText('Show Toast'));

    // Wait for toast to appear
    expect(screen.getByText('Success!')).toBeInTheDocument();

    const results = await axe(container);
    expect(results).toHaveNoViolations();
  });

  it('should have no violations for error toast', async () => {
    const user = userEvent.setup();

    const { container } = render(
      <ToastProvider>
        <TestToastTrigger type="error" title="Error!" message="Something went wrong" />
      </ToastProvider>
    );

    await user.click(screen.getByText('Show Toast'));
    expect(screen.getByText('Error!')).toBeInTheDocument();

    const results = await axe(container);
    expect(results).toHaveNoViolations();
  });

  it('should have no violations for warning toast', async () => {
    const user = userEvent.setup();

    const { container } = render(
      <ToastProvider>
        <TestToastTrigger type="warning" title="Warning!" />
      </ToastProvider>
    );

    await user.click(screen.getByText('Show Toast'));
    expect(screen.getByText('Warning!')).toBeInTheDocument();

    const results = await axe(container);
    expect(results).toHaveNoViolations();
  });

  it('should have no violations for info toast', async () => {
    const user = userEvent.setup();

    const { container } = render(
      <ToastProvider>
        <TestToastTrigger type="info" title="Info" />
      </ToastProvider>
    );

    await user.click(screen.getByText('Show Toast'));
    expect(screen.getByText('Info')).toBeInTheDocument();

    const results = await axe(container);
    expect(results).toHaveNoViolations();
  });

  it('uses role="status" (polite) for success and info toasts', async () => {
    // Success/info are non-interrupting feedback. They must map to the polite
    // live region (role="status") so they do not interrupt whatever the
    // screen reader is currently announcing. role="alert" implies
    // aria-live="assertive", which would barge in — wrong for these variants.
    for (const type of ['success', 'info'] as const) {
      const user = userEvent.setup();
      const { unmount } = render(
        <ToastProvider>
          <TestToastTrigger type={type} title={`${type} toast`} />
        </ToastProvider>
      );

      await user.click(screen.getByText('Show Toast'));
      expect(screen.getByText(`${type} toast`)).toBeInTheDocument();

      const live = screen.getByRole('status');
      expect(live).toHaveAttribute('aria-live', 'polite');
      // A polite toast must not also carry the assertive alert role.
      expect(screen.queryByRole('alert')).not.toBeInTheDocument();

      unmount();
    }
  });

  it('uses role="alert" (assertive) for error and warning toasts', async () => {
    // Errors/warnings are time-sensitive and should interrupt. They map to the
    // assertive live region (role="alert", aria-live="assertive").
    for (const type of ['error', 'warning'] as const) {
      const user = userEvent.setup();
      const { unmount } = render(
        <ToastProvider>
          <TestToastTrigger type={type} title={`${type} toast`} />
        </ToastProvider>
      );

      await user.click(screen.getByText('Show Toast'));
      expect(screen.getByText(`${type} toast`)).toBeInTheDocument();

      const live = screen.getByRole('alert');
      expect(live).toHaveAttribute('aria-live', 'assertive');
      // An assertive toast must not be exposed as a polite status region.
      expect(screen.queryByRole('status')).not.toBeInTheDocument();

      unmount();
    }
  });

  it('dismiss button should be keyboard accessible', async () => {
    const user = userEvent.setup();

    render(
      <ToastProvider>
        <TestToastTrigger type="info" title="Dismissable Toast" />
      </ToastProvider>
    );

    await user.click(screen.getByText('Show Toast'));
    expect(screen.getByText('Dismissable Toast')).toBeInTheDocument();

    // Verify dismiss button exists and has accessible label
    const dismissButton = screen.getByLabelText('Dismiss notification');
    expect(dismissButton).toBeInTheDocument();
    expect(dismissButton).toHaveAttribute('type', 'button');
  });
});
