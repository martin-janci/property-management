/**
 * ModalDrawer — unified modal dialog and right-slide drawer.
 *
 * mode="modal" renders a centred overlay dialog.
 * mode="drawer" renders a right-side slide-in panel.
 *
 * Both use a backdrop, focus trap via <dialog>, and Escape-to-close.
 * WCAG: uses native <dialog> element for correct role and focus management.
 *
 * Usage:
 *   <ModalDrawer mode="modal" open={isOpen} onClose={close} title="Edit listing">
 *     <p>Content here</p>
 *   </ModalDrawer>
 */

import React, { useEffect, useRef } from 'react';
import styles from './ModalDrawer.module.css';

export type ModalDrawerMode = 'modal' | 'drawer';

export interface ModalDrawerProps {
  mode: ModalDrawerMode;
  open: boolean;
  onClose: () => void;
  title: string;
  children: React.ReactNode;
  /** Custom class applied to the panel. */
  className?: string;
}

const XIcon = () => (
  <svg
    width="16"
    height="16"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2.5"
    strokeLinecap="round"
    strokeLinejoin="round"
    aria-hidden="true"
  >
    <line x1="18" y1="6" x2="6" y2="18" />
    <line x1="6" y1="6" x2="18" y2="18" />
  </svg>
);

/** ModalDrawer: centred modal or right-slide drawer with backdrop. */
export const ModalDrawer: React.FC<ModalDrawerProps> = ({
  mode,
  open,
  onClose,
  title,
  children,
  className,
}) => {
  const dialogRef = useRef<HTMLDialogElement>(null);

  // Open / close native dialog
  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;

    if (open) {
      if (!dialog.open) {
        dialog.showModal();
      }
    } else {
      if (dialog.open) {
        dialog.close();
      }
    }
  }, [open]);

  // Close on Escape (native dialog already handles this, but we sync state)
  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;

    const handler = () => onClose();
    dialog.addEventListener('cancel', handler);
    return () => dialog.removeEventListener('cancel', handler);
  }, [onClose]);

  if (!open) return null;

  const isDrawer = mode === 'drawer';

  return (
    <dialog
      ref={dialogRef}
      className={[
        styles.backdrop,
        isDrawer ? styles.backdropDrawer : '',
      ].filter(Boolean).join(' ')}
      onClick={(e) => {
        // Close when clicking the backdrop (outside the panel)
        if (e.target === e.currentTarget) onClose();
      }}
      aria-labelledby="modal-drawer-title"
      // Remove default dialog styling
      style={{ padding: 0, border: 'none', background: 'transparent', maxWidth: '100vw', maxHeight: '100vh' }}
    >
      <div
        className={[
          styles.panel,
          isDrawer ? styles.panelDrawer : '',
          className,
        ].filter(Boolean).join(' ')}
        role="document"
      >
        {/* Header */}
        <div className={styles.header}>
          <h2 id="modal-drawer-title" className={styles.title}>{title}</h2>
          <button
            className={styles.closeBtn}
            onClick={onClose}
            type="button"
            aria-label="Close"
          >
            <XIcon />
          </button>
        </div>

        {/* Content */}
        <div className={styles.content}>
          {children}
        </div>
      </div>
    </dialog>
  );
};

export default ModalDrawer;
