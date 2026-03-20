/**
 * Badge component for status indicators and labels.
 */

import type React from 'react';
import { forwardRef } from 'react';
import './Badge.css';

export type BadgeVariant = 'default' | 'primary' | 'secondary' | 'success' | 'warning' | 'danger' | 'info';
export type BadgeSize = 'sm' | 'md' | 'lg';

export interface BadgeProps extends React.HTMLAttributes<HTMLSpanElement> {
  /** Visual variant of the badge */
  variant?: BadgeVariant;
  /** Size of the badge */
  size?: BadgeSize;
  /** Whether to show as a dot (no text) */
  dot?: boolean;
  /** Whether the badge is rounded (pill shape) */
  rounded?: boolean;
  /** Icon to show before the badge text */
  icon?: React.ReactNode;
}

/**
 * Badge component for status indicators and labels.
 *
 * @example
 * ```tsx
 * <Badge variant="success">Active</Badge>
 * <Badge variant="danger" dot />
 * <Badge variant="primary" size="sm" rounded>New</Badge>
 * ```
 */
export const Badge = forwardRef<HTMLSpanElement, BadgeProps>(
  (
    {
      variant = 'default',
      size = 'md',
      dot = false,
      rounded = false,
      icon,
      className = '',
      children,
      ...props
    },
    ref
  ) => {
    const classes = [
      'ppt-badge',
      `ppt-badge--${variant}`,
      `ppt-badge--${size}`,
      dot && 'ppt-badge--dot',
      rounded && 'ppt-badge--rounded',
      className,
    ]
      .filter(Boolean)
      .join(' ');

    if (dot) {
      return (
        <span
          ref={ref}
          className={classes}
          role="status"
          aria-label={typeof children === 'string' ? children : undefined}
          {...props}
        />
      );
    }

    return (
      <span ref={ref} className={classes} {...props}>
        {icon && <span className="ppt-badge__icon">{icon}</span>}
        {children}
      </span>
    );
  }
);

Badge.displayName = 'Badge';
