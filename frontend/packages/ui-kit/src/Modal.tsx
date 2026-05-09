/**
 * Modal component for dialogs and overlays.
 */

import type React from 'react';
import { useCallback, useEffect, useRef } from 'react';
import './Modal.css';

export type ModalSize = 'sm' | 'md' | 'lg' | 'xl' | 'full';

export interface ModalProps {
  /** Whether the modal is open */
  open: boolean;
  /** Callback when modal should close */
  onClose: () => void;
  /** Modal title */
  title?: string;
  /** Modal content */
  children: React.ReactNode;
  /** Size of the modal */
  size?: ModalSize;
  /** Whether to close on backdrop click */
  closeOnBackdropClick?: boolean;
  /** Whether to close on Escape key */
  closeOnEscape?: boolean;
  /** Footer content (typically buttons) */
  footer?: React.ReactNode;
  /** Whether to show close button */
  showCloseButton?: boolean;
  /** Accessible label for close button */
  closeButtonLabel?: string;
}

/**
 * Modal component for dialogs.
 *
 * @example
 * ```tsx
 * <Modal
 *   open={isOpen}
 *   onClose={() => setIsOpen(false)}
 *   title="Confirm Action"
 *   footer={
 *     <>
 *       <Button variant="secondary" onClick={onCancel}>Cancel</Button>
 *       <Button variant="primary" onClick={onConfirm}>Confirm</Button>
 *     </>
 *   }
 * >
 *   Are you sure you want to continue?
 * </Modal>
 * ```
 */
export function Modal({
  open,
  onClose,
  title,
  children,
  size = 'md',
  closeOnBackdropClick = true,
  closeOnEscape = true,
  footer,
  showCloseButton = true,
  closeButtonLabel = 'Close',
}: ModalProps) {
  const dialogRef = useRef<HTMLDivElement>(null);
  const previousActiveElement = useRef<HTMLElement | null>(null);

  // Handle escape key
  const handleKeyDown = useCallback(
    (event: KeyboardEvent) => {
      if (closeOnEscape && event.key === 'Escape') {
        onClose();
      }
    },
    [closeOnEscape, onClose]
  );

  // Handle backdrop click
  const handleBackdropClick = useCallback(
    (event: React.MouseEvent<HTMLDivElement>) => {
      if (closeOnBackdropClick && event.target === event.currentTarget) {
        onClose();
      }
    },
    [closeOnBackdropClick, onClose]
  );

  // Focus management
  useEffect(() => {
    if (open) {
      // Store the currently focused element
      previousActiveElement.current = document.activeElement as HTMLElement;

      // Focus the dialog
      dialogRef.current?.focus();

      // Add escape key listener
      document.addEventListener('keydown', handleKeyDown);

      // Prevent body scroll
      document.body.style.overflow = 'hidden';

      return () => {
        document.removeEventListener('keydown', handleKeyDown);
        document.body.style.overflow = '';

        // Restore focus when modal closes
        previousActiveElement.current?.focus();
      };
    }
  }, [open, handleKeyDown]);

  // Trap focus within modal
  const handleFocusTrap = useCallback((event: React.KeyboardEvent<HTMLDivElement>) => {
    if (event.key !== 'Tab') return;

    const dialog = dialogRef.current;
    if (!dialog) return;

    const focusableElements = dialog.querySelectorAll<HTMLElement>(
      'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
    );
    const firstElement = focusableElements[0];
    const lastElement = focusableElements[focusableElements.length - 1];

    if (event.shiftKey) {
      if (document.activeElement === firstElement) {
        lastElement?.focus();
        event.preventDefault();
      }
    } else {
      if (document.activeElement === lastElement) {
        firstElement?.focus();
        event.preventDefault();
      }
    }
  }, []);

  if (!open) {
    return null;
  }

  return (
    <div className="ppt-modal__backdrop" onClick={handleBackdropClick} aria-hidden="true">
      <div
        ref={dialogRef}
        className={`ppt-modal ppt-modal--${size}`}
        role="dialog"
        aria-modal="true"
        aria-labelledby={title ? 'ppt-modal-title' : undefined}
        tabIndex={-1}
        onKeyDown={handleFocusTrap}
      >
        {(title || showCloseButton) && (
          <div className="ppt-modal__header">
            {title && (
              <h2 id="ppt-modal-title" className="ppt-modal__title">
                {title}
              </h2>
            )}
            {showCloseButton && (
              <button
                type="button"
                className="ppt-modal__close"
                onClick={onClose}
                aria-label={closeButtonLabel}
              >
                <svg
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                >
                  <line x1="18" y1="6" x2="6" y2="18" />
                  <line x1="6" y1="6" x2="18" y2="18" />
                </svg>
              </button>
            )}
          </div>
        )}
        <div className="ppt-modal__content">{children}</div>
        {footer && <div className="ppt-modal__footer">{footer}</div>}
      </div>
    </div>
  );
}

Modal.displayName = 'Modal';
