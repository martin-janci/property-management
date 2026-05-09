/**
 * EmptyState — inline empty/no-results placeholder for use inside content
 * sections. For full-page error layouts use `StateView` instead.
 */

import type { ReactNode } from 'react';
import '../styles/errorPages.css';

export interface EmptyStateProps {
  icon?: string;
  title: string;
  description?: string;
  children?: ReactNode;
}

export function EmptyState({ icon = '📭', title, description, children }: EmptyStateProps) {
  return (
    <div className="state-inline" role="status">
      <span className="state-icon" aria-hidden="true">
        {icon}
      </span>
      <h2 className="state-title">{title}</h2>
      {description && <p className="state-text">{description}</p>}
      {children}
    </div>
  );
}

EmptyState.displayName = 'EmptyState';
