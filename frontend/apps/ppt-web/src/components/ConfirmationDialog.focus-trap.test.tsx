/// <reference types="vitest/globals" />
/**
 * ConfirmationDialog focus-trap regression tests.
 *
 * Guards against a WCAG 2.1.2 (No Keyboard Trap) / 2.4.3 (Focus Order) defect:
 * while the confirm action is in flight BOTH action buttons render
 * `disabled`, so the Tab focus-trap's focusable-element query matched zero
 * elements and silently no-oped — native Tab then moved focus OUT of the open
 * modal to the background page. Compounding, the initial-focus effect was
 * keyed only on `[isOpen]`, so the focused cancel button becoming disabled
 * mid-flight dropped focus to <body> for the whole latency window.
 *
 * These tests assert focus stays trapped inside the dialog while loading.
 */

import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ConfirmationDialog } from './ConfirmationDialog';

describe('ConfirmationDialog focus trap while loading', () => {
  const defaultProps = {
    isOpen: true,
    title: 'Delete item',
    message: 'This cannot be undone.',
    onConfirm: vi.fn(),
    onCancel: vi.fn(),
  };

  it('holds focus inside the dialog when opened in the loading state', async () => {
    const user = userEvent.setup();
    render(<ConfirmationDialog {...defaultProps} isLoading />);

    const dialog = screen.getByRole('alertdialog');

    // Both buttons are disabled, so focus must rest on the dialog container,
    // never on <body>.
    expect(document.activeElement).not.toBe(document.body);
    expect(dialog.contains(document.activeElement)).toBe(true);

    // Forward Tab must not release focus to the background page.
    await user.tab();
    expect(document.activeElement).not.toBe(document.body);
    expect(dialog.contains(document.activeElement)).toBe(true);

    // Reverse Tab (Shift+Tab) must not escape either.
    await user.tab({ shift: true });
    expect(document.activeElement).not.toBe(document.body);
    expect(dialog.contains(document.activeElement)).toBe(true);
  });

  it('pulls focus back into the dialog when the focused button is disabled mid-flight', () => {
    const { rerender } = render(<ConfirmationDialog {...defaultProps} />);

    // Default focus lands on the cancel button.
    expect(screen.getByText('Cancel')).toHaveFocus();

    // Confirm action starts — both buttons become disabled.
    rerender(<ConfirmationDialog {...defaultProps} isLoading />);

    const dialog = screen.getByRole('alertdialog');
    expect(document.activeElement).not.toBe(document.body);
    expect(dialog.contains(document.activeElement)).toBe(true);
  });
});
