/**
 * Spinner component for loading states.
 */

import type React from 'react';
import { forwardRef } from 'react';
import './Spinner.css';

export type SpinnerSize = 'xs' | 'sm' | 'md' | 'lg' | 'xl';

export interface SpinnerProps extends React.HTMLAttributes<HTMLDivElement> {
  /** Size of the spinner */
  size?: SpinnerSize;
  /** Color of the spinner (CSS color value) */
  color?: string;
  /** Label for screen readers */
  label?: string;
}

/**
 * Spinner component for indicating loading states.
 *
 * @example
 * ```tsx
 * <Spinner />
 * <Spinner size="lg" label="Loading data..." />
 * <Spinner size="sm" color="var(--ppt-color-primary)" />
 * ```
 */
export const Spinner = forwardRef<HTMLDivElement, SpinnerProps>(
  (
    {
      size = 'md',
      color,
      label = 'Loading',
      className = '',
      style,
      ...props
    },
    ref
  ) => {
    const classes = ['ppt-spinner', `ppt-spinner--${size}`, className]
      .filter(Boolean)
      .join(' ');

    const spinnerStyle: React.CSSProperties = {
      ...style,
      ...(color && { '--ppt-spinner-color': color } as React.CSSProperties),
    };

    return (
      <div
        ref={ref}
        className={classes}
        role="status"
        aria-label={label}
        style={spinnerStyle}
        {...props}
      >
        <svg viewBox="0 0 24 24" fill="none" className="ppt-spinner__icon">
          <circle
            cx="12"
            cy="12"
            r="10"
            stroke="currentColor"
            strokeWidth="3"
            strokeLinecap="round"
            strokeDasharray="31.4 31.4"
            className="ppt-spinner__circle"
          />
        </svg>
        <span className="ppt-spinner__label">{label}</span>
      </div>
    );
  }
);

Spinner.displayName = 'Spinner';

/**
 * SpinnerOverlay component for full-page loading states.
 */
export interface SpinnerOverlayProps extends SpinnerProps {
  /** Whether to show the overlay */
  visible?: boolean;
  /** Whether the overlay is transparent */
  transparent?: boolean;
}

export const SpinnerOverlay = forwardRef<HTMLDivElement, SpinnerOverlayProps>(
  ({ visible = true, transparent = false, className = '', ...props }, ref) => {
    if (!visible) return null;

    const classes = [
      'ppt-spinner-overlay',
      transparent && 'ppt-spinner-overlay--transparent',
      className,
    ]
      .filter(Boolean)
      .join(' ');

    return (
      <div ref={ref} className={classes}>
        <Spinner {...props} />
      </div>
    );
  }
);

SpinnerOverlay.displayName = 'SpinnerOverlay';
