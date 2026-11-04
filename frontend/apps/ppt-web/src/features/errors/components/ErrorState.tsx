/**
 * ErrorState — inline error placeholder with an optional retry callback.
 */

import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
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
  title,
  description,
  onRetry,
  retryLabel,
  children,
}: ErrorStateProps) {
  const { t } = useTranslation();
  const resolvedDescription = description ?? t('errors.pages.inlineError.description');
  return (
    <div className="state-inline" role="alert">
      <span className="state-icon" aria-hidden="true">
        {icon}
      </span>
      <h2 className="state-title">{title ?? t('errors.somethingWentWrong')}</h2>
      {resolvedDescription && <p className="state-text">{resolvedDescription}</p>}
      {(onRetry || children) && (
        <div className="state-actions">
          {onRetry && (
            <button type="button" className="state-action state-action--primary" onClick={onRetry}>
              {retryLabel ?? t('errors.pages.tryAgain')}
            </button>
          )}
          {children}
        </div>
      )}
    </div>
  );
}

ErrorState.displayName = 'ErrorState';
