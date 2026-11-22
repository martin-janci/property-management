/**
 * Accessible Prompt Dialog Component.
 *
 * The text-input sibling of {@link ConfirmationDialog}: an in-app, styled,
 * focus-trapped, translatable replacement for `window.prompt()`. Provides
 * proper focus management, keyboard navigation (Escape cancels, Enter submits
 * single-line input) and screen-reader support.
 */

import type React from 'react';
import { useCallback, useEffect, useId, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import './ConfirmationDialog.css';
import './PromptDialog.css';

export interface PromptDialogProps {
  /** Whether the dialog is open */
  isOpen: boolean;
  /** Dialog title */
  title: string;
  /** Dialog message/description shown above the input */
  message: string;
  /** Label for the submit button */
  confirmLabel?: string;
  /** Label for the cancel button */
  cancelLabel?: string;
  /** Variant affects styling of the confirm button */
  variant?: 'default' | 'danger';
  /** Placeholder text for the input */
  placeholder?: string;
  /** Initial input value */
  defaultValue?: string;
  /** Render a multi-line textarea instead of a single-line input */
  multiline?: boolean;
  /** When true the submit button is disabled while the input is empty/whitespace */
  requireValue?: boolean;
  /** Accessible label for the input (defaults to the dialog title) */
  inputLabel?: string;
  /** Called with the entered value when the user submits */
  onConfirm: (value: string) => void;
  /** Called when the user cancels or presses Escape */
  onCancel: () => void;
}

export const PromptDialog: React.FC<PromptDialogProps> = ({
  isOpen,
  title,
  message,
  confirmLabel = 'Confirm',
  cancelLabel = 'Cancel',
  variant = 'default',
  placeholder,
  defaultValue = '',
  multiline = false,
  requireValue = true,
  inputLabel,
  onConfirm,
  onCancel,
}) => {
  const { t } = useTranslation();
  const dialogRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement | HTMLTextAreaElement>(null);
  const previousActiveElement = useRef<Element | null>(null);
  const [value, setValue] = useState(defaultValue);
  const titleId = useId();
  const descId = useId();
  const inputId = useId();

  // Reset the field to its default each time the dialog opens so a reused
  // instance never leaks the previous prompt's text.
  useEffect(() => {
    if (isOpen) {
      setValue(defaultValue);
    }
  }, [isOpen, defaultValue]);

  // Store the opener and move focus into the dialog (onto the input).
  useEffect(() => {
    if (isOpen) {
      if (previousActiveElement.current === null) {
        previousActiveElement.current = document.activeElement;
      }
      inputRef.current?.focus();
    } else {
      if (previousActiveElement.current instanceof HTMLElement) {
        previousActiveElement.current.focus();
      }
      previousActiveElement.current = null;
    }
  }, [isOpen]);

  const submitDisabled = requireValue && value.trim().length === 0;

  const submit = useCallback(() => {
    if (requireValue && value.trim().length === 0) {
      return;
    }
    onConfirm(value);
  }, [onConfirm, requireValue, value]);

  const handleKeyDown = useCallback(
    (event: React.KeyboardEvent) => {
      if (event.key === 'Escape') {
        onCancel();
        return;
      }

      // Enter submits from a single-line input (but not from the textarea,
      // where Enter must insert a newline).
      if (event.key === 'Enter' && !multiline && event.target === inputRef.current) {
        event.preventDefault();
        submit();
        return;
      }

      // Trap focus within the dialog.
      if (event.key === 'Tab' && dialogRef.current) {
        const focusableElements = dialogRef.current.querySelectorAll<HTMLElement>(
          'button:not([disabled]), input, textarea, [tabindex]:not([tabindex="-1"])'
        );
        if (focusableElements.length === 0) {
          event.preventDefault();
          dialogRef.current.focus();
          return;
        }
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
    [multiline, onCancel, submit]
  );

  const handleBackdropClick = useCallback(
    (event: React.MouseEvent) => {
      if (event.target === event.currentTarget) {
        onCancel();
      }
    },
    [onCancel]
  );

  if (!isOpen) {
    return null;
  }

  const inputClassName = 'prompt-dialog-input';
  const ariaLabel = inputLabel ?? title;

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
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        aria-describedby={descId}
        tabIndex={-1}
      >
        <h2 id={titleId} className="confirmation-dialog-title">
          {title}
        </h2>
        <p id={descId} className="confirmation-dialog-message prompt-dialog-message">
          {message}
        </p>
        <div className="prompt-dialog-field">
          {multiline ? (
            <textarea
              id={inputId}
              ref={inputRef as React.RefObject<HTMLTextAreaElement>}
              className={inputClassName}
              value={value}
              placeholder={placeholder}
              aria-label={ariaLabel}
              onChange={(e) => setValue(e.target.value)}
            />
          ) : (
            <input
              id={inputId}
              ref={inputRef as React.RefObject<HTMLInputElement>}
              type="text"
              className={inputClassName}
              value={value}
              placeholder={placeholder}
              aria-label={ariaLabel}
              onChange={(e) => setValue(e.target.value)}
            />
          )}
        </div>
        <div className="confirmation-dialog-actions">
          <button
            type="button"
            className="confirmation-dialog-button confirmation-dialog-button--cancel"
            onClick={onCancel}
          >
            {cancelLabel || t('common.cancel')}
          </button>
          <button
            type="button"
            className={`confirmation-dialog-button confirmation-dialog-button--confirm confirmation-dialog-button--${variant}`}
            onClick={submit}
            disabled={submitDisabled}
          >
            {confirmLabel || t('common.confirm')}
          </button>
        </div>
      </div>
    </div>
  );
};

PromptDialog.displayName = 'PromptDialog';
