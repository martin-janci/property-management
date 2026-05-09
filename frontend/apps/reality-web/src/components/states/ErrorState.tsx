'use client';

/**
 * ErrorState — inline error placeholder with optional retry callback.
 */

import type { ReactNode } from 'react';

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
            <button type="button" className="state-action" onClick={onRetry}>
              {retryLabel}
            </button>
          )}
          {children}
        </div>
      )}
      <style jsx>{`
        .state-inline {
          padding: 2.5rem 1rem;
          text-align: center;
          color: var(--ppt-fg-muted);
          display: flex;
          flex-direction: column;
          align-items: center;
          gap: 0.75rem;
        }
        .state-icon { font-size: 2rem; }
        .state-title { font-size: 1.125rem; font-weight: 600; color: var(--ppt-fg-primary); margin: 0; }
        .state-text { font-size: 0.875rem; max-width: 360px; margin: 0; }
        .state-actions { display: flex; gap: 8px; }
        .state-action {
          padding: 10px 20px; border-radius: 8px; border: none; cursor: pointer;
          background: var(--ppt-color-primary); color: var(--ppt-fg-on-accent); font-size: 14px; font-weight: 500;
        }
        .state-action:hover { background: var(--ppt-color-primary-hover); }
        .state-action:focus-visible { outline: none; box-shadow: var(--ppt-focus-ring-shadow); }
      `}</style>
    </div>
  );
}
