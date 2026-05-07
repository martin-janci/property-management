/**
 * Alert component for messages and notifications.
 */

import type React from 'react';
import { forwardRef } from 'react';
import './Alert.css';

export type AlertVariant = 'info' | 'success' | 'warning' | 'danger';

export interface AlertProps extends React.HTMLAttributes<HTMLDivElement> {
  /** Visual variant of the alert */
  variant?: AlertVariant;
  /** Alert title */
  title?: string;
  /** Icon to display (if not provided, default icon for variant is used) */
  icon?: React.ReactNode;
  /** Whether to show the default icon */
  showIcon?: boolean;
  /** Whether the alert can be dismissed */
  dismissible?: boolean;
  /** Callback when dismiss button is clicked */
  onDismiss?: () => void;
  /** Accessible label for dismiss button */
  dismissLabel?: string;
}

/**
 * Alert component for displaying important messages.
 *
 * @example
 * ```tsx
 * <Alert variant="success" title="Success!">
 *   Your changes have been saved.
 * </Alert>
 *
 * <Alert variant="danger" dismissible onDismiss={handleDismiss}>
 *   An error occurred while saving.
 * </Alert>
 * ```
 */
export const Alert = forwardRef<HTMLDivElement, AlertProps>(
  (
    {
      variant = 'info',
      title,
      icon,
      showIcon = true,
      dismissible = false,
      onDismiss,
      dismissLabel = 'Dismiss',
      className = '',
      children,
      ...props
    },
    ref
  ) => {
    const classes = ['ppt-alert', `ppt-alert--${variant}`, className].filter(Boolean).join(' ');

    const defaultIcon = icon || getDefaultIcon(variant);

    return (
      <div ref={ref} className={classes} role="alert" {...props}>
        {showIcon && <span className="ppt-alert__icon">{defaultIcon}</span>}
        <div className="ppt-alert__content">
          {title && <div className="ppt-alert__title">{title}</div>}
          {children && <div className="ppt-alert__message">{children}</div>}
        </div>
        {dismissible && (
          <button
            type="button"
            className="ppt-alert__dismiss"
            onClick={onDismiss}
            aria-label={dismissLabel}
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
    );
  }
);

Alert.displayName = 'Alert';

function getDefaultIcon(variant: AlertVariant): React.ReactNode {
  switch (variant) {
    case 'success':
      return (
        <svg
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <circle cx="12" cy="12" r="10" />
          <polyline points="9 12 12 15 16 10" />
        </svg>
      );
    case 'warning':
      return (
        <svg
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z" />
          <line x1="12" y1="9" x2="12" y2="13" />
          <line x1="12" y1="17" x2="12.01" y2="17" />
        </svg>
      );
    case 'danger':
      return (
        <svg
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <circle cx="12" cy="12" r="10" />
          <line x1="15" y1="9" x2="9" y2="15" />
          <line x1="9" y1="9" x2="15" y2="15" />
        </svg>
      );
    default:
      return (
        <svg
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <circle cx="12" cy="12" r="10" />
          <line x1="12" y1="16" x2="12" y2="12" />
          <line x1="12" y1="8" x2="12.01" y2="8" />
        </svg>
      );
  }
}
