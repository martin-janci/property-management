/**
 * Card component for content containers.
 */

import type React from 'react';
import { forwardRef } from 'react';
import './Card.css';

export interface CardProps extends React.HTMLAttributes<HTMLDivElement> {
  /** Card variant */
  variant?: 'elevated' | 'outlined' | 'flat';
  /** Whether the card is interactive (clickable) */
  interactive?: boolean;
  /** Padding size */
  padding?: 'none' | 'sm' | 'md' | 'lg';
}

export interface CardHeaderProps extends React.HTMLAttributes<HTMLDivElement> {
  /** Card title */
  title?: string;
  /** Card subtitle */
  subtitle?: string;
  /** Action element (e.g., button, menu) */
  action?: React.ReactNode;
}

export interface CardContentProps extends React.HTMLAttributes<HTMLDivElement> {}

export interface CardFooterProps extends React.HTMLAttributes<HTMLDivElement> {}

/**
 * Card component for grouping related content.
 *
 * @example
 * ```tsx
 * <Card variant="elevated">
 *   <Card.Header title="Card Title" subtitle="Subtitle" />
 *   <Card.Content>
 *     <p>Card content goes here.</p>
 *   </Card.Content>
 *   <Card.Footer>
 *     <Button>Action</Button>
 *   </Card.Footer>
 * </Card>
 * ```
 */
export const Card = forwardRef<HTMLDivElement, CardProps>(
  (
    {
      variant = 'elevated',
      interactive = false,
      padding = 'md',
      className = '',
      children,
      ...props
    },
    ref
  ) => {
    const classes = [
      'ppt-card',
      `ppt-card--${variant}`,
      `ppt-card--padding-${padding}`,
      interactive && 'ppt-card--interactive',
      className,
    ]
      .filter(Boolean)
      .join(' ');

    return (
      <div ref={ref} className={classes} {...props}>
        {children}
      </div>
    );
  }
) as React.ForwardRefExoticComponent<CardProps & React.RefAttributes<HTMLDivElement>> & {
  Header: typeof CardHeader;
  Content: typeof CardContent;
  Footer: typeof CardFooter;
};

Card.displayName = 'Card';

/**
 * Card header with title, subtitle, and optional action.
 */
const CardHeader = forwardRef<HTMLDivElement, CardHeaderProps>(
  ({ title, subtitle, action, className = '', children, ...props }, ref) => {
    const classes = ['ppt-card__header', className].filter(Boolean).join(' ');

    return (
      <div ref={ref} className={classes} {...props}>
        {(title || subtitle) && (
          <div className="ppt-card__header-content">
            {title && <h3 className="ppt-card__title">{title}</h3>}
            {subtitle && <p className="ppt-card__subtitle">{subtitle}</p>}
          </div>
        )}
        {children}
        {action && <div className="ppt-card__header-action">{action}</div>}
      </div>
    );
  }
);

CardHeader.displayName = 'Card.Header';

/**
 * Card content area.
 */
const CardContent = forwardRef<HTMLDivElement, CardContentProps>(
  ({ className = '', children, ...props }, ref) => {
    const classes = ['ppt-card__content', className].filter(Boolean).join(' ');

    return (
      <div ref={ref} className={classes} {...props}>
        {children}
      </div>
    );
  }
);

CardContent.displayName = 'Card.Content';

/**
 * Card footer for actions.
 */
const CardFooter = forwardRef<HTMLDivElement, CardFooterProps>(
  ({ className = '', children, ...props }, ref) => {
    const classes = ['ppt-card__footer', className].filter(Boolean).join(' ');

    return (
      <div ref={ref} className={classes} {...props}>
        {children}
      </div>
    );
  }
);

CardFooter.displayName = 'Card.Footer';

// Attach sub-components
Card.Header = CardHeader;
Card.Content = CardContent;
Card.Footer = CardFooter;
