/**
 * Accessible Confirmation Dialog Component.
 *
 * Provides an accessible modal dialog for confirmation actions,
 * with proper focus management, keyboard navigation, and screen reader support.
 */

import type React from 'react';
import { useCallback, useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import './ConfirmationDialog.css';

export interface ConfirmationDialogProps {
  /** Whether the dialog is open */
  isOpen: boolean;
  /** Dialog title */
  title: string;
  /** Dialog message/description */
  message: string;
  /** Text for the confirm button */
  confirmLabel?: string;
  /** Text for the cancel button */
  cancelLabel?: string;
  /** Variant affects styling of confirm button */
  variant?: 'default' | 'danger';
  /** Called when user confirms */
  onConfirm: () => void;
  /** Called when user cancels or presses Escape */
  onCancel: () => void;
  /** Whether the confirm action is in progress */
  isLoading?: boolean;
  /**
   * Text shown on the confirm button while `isLoading` is true.
   * Defaults to the shared `common.processing` translation.
   */
  loadingLabel?: string;
}

export const ConfirmationDialog: React.FC<ConfirmationDialogProps> = ({
  isOpen,
  title,
  message,
  confirmLabel = 'Confirm',
  cancelLabel = 'Cancel',
  variant = 'default',
  onConfirm,
  onCancel,
  isLoading = false,
  loadingLabel,
}) => {
  const { t } = useTranslation();
  const dialogRef = useRef<HTMLDivElement>(null);
  const cancelButtonRef = useRef<HTMLButtonElement>(null);
  const previousActiveElement = useRef<Element | null>(null);

  // Store the previously focused element and focus the dialog when opened
  useEffect(() => {
    if (isOpen) {
      previousActiveElement.current = document.activeElement;
      // Focus the cancel button by default (safer action)
      cancelButtonRef.current?.focus();
    } else if (previousActiveElement.current instanceof HTMLElement) {
      // Restore focus when closed
      previousActiveElement.current.focus();
    }
  }, [isOpen]);

  // Handle Escape key
  const handleKeyDown = useCallback(
    (event: React.KeyboardEvent) => {
      if (event.key === 'Escape' && !isLoading) {
        onCancel();
      }

      // Trap focus within the dialog
      if (event.key === 'Tab' && dialogRef.current) {
        const focusableElements = dialogRef.current.querySelectorAll<HTMLElement>(
          'button:not([disabled]), [tabindex]:not([tabindex="-1"])'
        );
        const firstElement = focusableElements[0];
        const lastElement = focusableElements[focusableElements.length - 1];

        if (event.shiftKey && document.activeElement === firstElement) {
          event.preventDefault();
          lastElement?.focus();
        } else if (!event.shiftKey && document.activeElement === lastElement) {
          event.preventDefault();
          firstElement?.focus();
        }
      }
    },
    [isLoading, onCancel]
  );

  // Handle backdrop click
  const handleBackdropClick = useCallback(
    (event: React.MouseEvent) => {
      if (event.target === event.currentTarget && !isLoading) {
        onCancel();
      }
    },
    [isLoading, onCancel]
  );

  if (!isOpen) {
    return null;
  }

  return (
    <div
      className="confirmation-dialog-overlay"
      onClick={handleBackdropClick}
      onKeyDown={handleKeyDown}
      role="presentation"
    >
      <div
        ref={dialogRef}
        className="confirmation-dialog"
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="confirmation-dialog-title"
        aria-describedby="confirmation-dialog-description"
      >
        <h2 id="confirmation-dialog-title" className="confirmation-dialog-title">
          {title}
        </h2>
        <p id="confirmation-dialog-description" className="confirmation-dialog-message">
          {message}
        </p>
        <div className="confirmation-dialog-actions">
          <button
            ref={cancelButtonRef}
            type="button"
            className="confirmation-dialog-button confirmation-dialog-button--cancel"
            onClick={onCancel}
            disabled={isLoading}
          >
            {cancelLabel}
          </button>
          <button
            type="button"
            className={`confirmation-dialog-button confirmation-dialog-button--confirm confirmation-dialog-button--${variant}`}
            onClick={onConfirm}
            disabled={isLoading}
          >
            {isLoading ? (loadingLabel ?? t('common.processing')) : confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
};

ConfirmationDialog.displayName = 'ConfirmationDialog';
