/// <reference types="vitest/globals" />
/**
 * ConfirmationDialog Accessibility Tests (Epic 125, Story 125.2)
 *
 * Tests WCAG 2.1 AA compliance for the ConfirmationDialog component including:
 * - Role and ARIA attributes (alertdialog, aria-modal, aria-labelledby, aria-describedby)
 * - Keyboard accessibility (Tab trapping, Escape key)
 * - Focus management
 */

import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { axe } from 'vitest-axe';
import { ConfirmationDialog } from './ConfirmationDialog';

describe('ConfirmationDialog Accessibility', () => {
  const defaultProps = {
    isOpen: true,
    title: 'Confirm Action',
    message: 'Are you sure you want to proceed?',
    onConfirm: vi.fn(),
    onCancel: vi.fn(),
  };

  it('should have no accessibility violations when rendered', async () => {
    const { container } = render(<ConfirmationDialog {...defaultProps} />);

    const results = await axe(container);
    expect(results).toHaveNoViolations();
  });

  it('should have correct ARIA attributes', () => {
    render(<ConfirmationDialog {...defaultProps} />);

    const dialog = screen.getByRole('alertdialog');
    expect(dialog).toHaveAttribute('aria-modal', 'true');
    expect(dialog).toHaveAttribute('aria-labelledby', 'confirmation-dialog-title');
    expect(dialog).toHaveAttribute('aria-describedby', 'confirmation-dialog-description');
  });

  it('should have accessible title and description', () => {
    render(<ConfirmationDialog {...defaultProps} />);

    expect(screen.getByText('Confirm Action')).toHaveAttribute('id', 'confirmation-dialog-title');
    expect(screen.getByText('Are you sure you want to proceed?')).toHaveAttribute(
      'id',
      'confirmation-dialog-description'
    );
  });

  it('should focus the cancel button by default (safer action)', () => {
    render(<ConfirmationDialog {...defaultProps} />);

    expect(screen.getByText('Cancel')).toHaveFocus();
  });

  it('should close on Escape key', async () => {
    const user = userEvent.setup();
    const onCancel = vi.fn();
    render(<ConfirmationDialog {...defaultProps} onCancel={onCancel} />);

    await user.keyboard('{Escape}');
    expect(onCancel).toHaveBeenCalled();
  });

  it('should trap focus within the dialog', async () => {
    const user = userEvent.setup();
    render(<ConfirmationDialog {...defaultProps} />);

    const cancelButton = screen.getByText('Cancel');
    const confirmButton = screen.getByText('Confirm');

    // Focus starts on cancel button
    expect(cancelButton).toHaveFocus();

    // Tab to confirm button
    await user.tab();
    expect(confirmButton).toHaveFocus();

    // Tab should wrap back to cancel button
    await user.tab();
    expect(cancelButton).toHaveFocus();
  });

  it('generates unique, correctly-paired ids for concurrent dialog instances', () => {
    // Two dialogs mounted at once (e.g. nested confirmations) must not share
    // DOM ids, otherwise `aria-labelledby` / `aria-describedby` resolve to the
    // wrong title/description and screen readers announce the wrong content.
    render(
      <>
        <ConfirmationDialog
          {...defaultProps}
          title="First Dialog Title"
          message="First dialog description"
        />
        <ConfirmationDialog
          {...defaultProps}
          title="Second Dialog Title"
          message="Second dialog description"
        />
      </>
    );

    const dialogs = screen.getAllByRole('alertdialog');
    expect(dialogs).toHaveLength(2);

    const [first, second] = dialogs;

    const firstLabelledBy = first.getAttribute('aria-labelledby');
    const firstDescribedBy = first.getAttribute('aria-describedby');
    const secondLabelledBy = second.getAttribute('aria-labelledby');
    const secondDescribedBy = second.getAttribute('aria-describedby');

    expect(firstLabelledBy).toBeTruthy();
    expect(firstDescribedBy).toBeTruthy();

    // Distinct per-instance ids — no duplicate DOM ids across the two dialogs.
    expect(firstLabelledBy).not.toBe(secondLabelledBy);
    expect(firstDescribedBy).not.toBe(secondDescribedBy);

    // Each aria reference must resolve to an element inside its OWN dialog,
    // carrying that dialog's own title/description text.
    // biome-ignore lint/style/noNonNullAssertion: asserted truthy above
    const firstTitle = document.getElementById(firstLabelledBy!);
    // biome-ignore lint/style/noNonNullAssertion: asserted truthy above
    const firstDesc = document.getElementById(firstDescribedBy!);
    expect(first).toContainElement(firstTitle);
    expect(first).toContainElement(firstDesc);
    expect(firstTitle).toHaveTextContent('First Dialog Title');
    expect(firstDesc).toHaveTextContent('First dialog description');

    // biome-ignore lint/style/noNonNullAssertion: resolved below is checked by containment
    const secondTitle = document.getElementById(secondLabelledBy!);
    // biome-ignore lint/style/noNonNullAssertion: resolved below is checked by containment
    const secondDesc = document.getElementById(secondDescribedBy!);
    expect(second).toContainElement(secondTitle);
    expect(second).toContainElement(secondDesc);
    expect(secondTitle).toHaveTextContent('Second Dialog Title');
    expect(secondDesc).toHaveTextContent('Second dialog description');
  });

  it('should have no violations for danger variant', async () => {
    const { container } = render(<ConfirmationDialog {...defaultProps} variant="danger" />);

    const results = await axe(container);
    expect(results).toHaveNoViolations();
  });

  it('should have no violations when loading', async () => {
    const { container } = render(<ConfirmationDialog {...defaultProps} isLoading />);

    const results = await axe(container);
    expect(results).toHaveNoViolations();
  });

  it('should disable buttons when loading', () => {
    render(<ConfirmationDialog {...defaultProps} isLoading />);

    expect(screen.getByText('Cancel')).toBeDisabled();
    expect(screen.getByText('Processing...')).toBeDisabled();
  });
});
