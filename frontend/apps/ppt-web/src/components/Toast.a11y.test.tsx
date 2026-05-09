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
