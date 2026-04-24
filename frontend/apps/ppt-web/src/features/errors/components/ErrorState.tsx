/**
 * ErrorState — inline error placeholder with an optional retry callback.
 */

import type { ReactNode } from 'react';
import '../styles/errorPages.css';

export interface ErrorStateProps {
  icon?: string;
  title?: string;
  description?: string;
  onRetry?: () => void;
  retryLabel?: string;
  children?: ReactNode;
}

export function ErrorState({
  icon = '⚠️',
  title = 'Something went wrong',
  description = 'Please try again. If the issue persists, contact support.',
  onRetry,
  retryLabel = 'Try again',
  children,
}: ErrorStateProps) {
  return (
    <div className="state-inline" role="alert">
      <span className="state-icon" aria-hidden="true">
        {icon}
      </span>
      <h2 className="state-title">{title}</h2>
      {description && <p className="state-text">{description}</p>}
      {(onRetry || children) && (
        <div className="state-actions">
          {onRetry && (
            <button type="button" className="state-action state-action--primary" onClick={onRetry}>
              {retryLabel}
            </button>
          )}
          {children}
        </div>
      )}
    </div>
  );
}

ErrorState.displayName = 'ErrorState';
