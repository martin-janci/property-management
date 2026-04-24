'use client';

/**
 * EmptyState — inline empty placeholder for content areas.
 */

import type { ReactNode } from 'react';

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
      <style jsx>{`
        .state-inline {
          padding: 2.5rem 1rem;
          text-align: center;
          color: #6b7280;
          display: flex;
          flex-direction: column;
          align-items: center;
          gap: 0.75rem;
        }
        .state-icon { font-size: 2rem; }
        .state-title { font-size: 1.125rem; font-weight: 600; color: #111827; margin: 0; }
        .state-text { font-size: 0.875rem; max-width: 360px; margin: 0; }
      `}</style>
    </div>
  );
}
