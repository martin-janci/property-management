/**
 * Button component for consistent button styling across apps.
 */

import type React from 'react';
import { forwardRef } from 'react';
import './Button.css';

export type ButtonVariant = 'primary' | 'secondary' | 'danger' | 'ghost' | 'link';
export type ButtonSize = 'sm' | 'md' | 'lg';

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  /** Visual variant of the button */
  variant?: ButtonVariant;
  /** Size of the button */
  size?: ButtonSize;
  /** Whether the button takes full width */
  fullWidth?: boolean;
  /** Whether the button is in loading state */
  loading?: boolean;
  /** Icon to show before the button text */
  leftIcon?: React.ReactNode;
  /** Icon to show after the button text */
  rightIcon?: React.ReactNode;
}

/**
 * Button component with multiple variants and sizes.
 *
 * @example
 * ```tsx
 * <Button variant="primary" onClick={handleClick}>
 *   Click me
 * </Button>
 *
 * <Button variant="danger" size="sm" loading>
 *   Deleting...
 * </Button>
 * ```
 */
export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  (
    {
      variant = 'primary',
      size = 'md',
      fullWidth = false,
      loading = false,
      disabled,
      leftIcon,
      rightIcon,
      children,
      className = '',
      type = 'button',
      ...props
    },
    ref
  ) => {
    const classes = [
      'ppt-button',
      `ppt-button--${variant}`,
      `ppt-button--${size}`,
      fullWidth && 'ppt-button--full-width',
      loading && 'ppt-button--loading',
      className,
    ]
      .filter(Boolean)
      .join(' ');

    return (
      <button
        ref={ref}
        type={type}
        className={classes}
        disabled={disabled || loading}
        aria-busy={loading}
        {...props}
      >
        {loading && (
          <span className="ppt-button__spinner" aria-hidden="true">
            <svg viewBox="0 0 24 24" fill="none" className="ppt-button__spinner-icon">
              <circle
                cx="12"
                cy="12"
                r="10"
                stroke="currentColor"
                strokeWidth="3"
                strokeLinecap="round"
                strokeDasharray="31.4 31.4"
              />
            </svg>
          </span>
        )}
        {leftIcon && !loading && (
          <span className="ppt-button__icon ppt-button__icon--left">{leftIcon}</span>
        )}
        <span className="ppt-button__content">{children}</span>
        {rightIcon && <span className="ppt-button__icon ppt-button__icon--right">{rightIcon}</span>}
      </button>
    );
  }
);

Button.displayName = 'Button';
