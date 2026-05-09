'use client';

/**
 * StateView — shared layout for full-page error / empty surfaces.
 */

import type { ReactNode } from 'react';

export interface StateViewProps {
  icon?: string;
  code?: string;
  title: string;
  description?: string;
  children?: ReactNode;
}

export function StateView({ icon, code, title, description, children }: StateViewProps) {
  return (
    <main className="state-page" role="alert" aria-live="polite">
      <div className="state-card">
        {icon && (
          <span className="state-icon" aria-hidden="true">
            {icon}
          </span>
        )}
        {code && <p className="state-code">{code}</p>}
        <h1 className="state-title">{title}</h1>
        {description && <p className="state-text">{description}</p>}
        {children && <div className="state-actions">{children}</div>}
      </div>
      <style jsx>{`
        .state-page {
          min-height: 100vh;
          display: flex;
          align-items: center;
          justify-content: center;
          padding: 2rem 1rem;
          background: var(--ppt-bg-app);
        }
        .state-card {
          width: 100%;
          max-width: 480px;
          background: var(--ppt-bg-surface);
          border-radius: 12px;
          box-shadow: var(--ppt-shadow-md);
          padding: 2.5rem 2rem;
          text-align: center;
        }
        .state-icon {
          font-size: 3rem;
          margin-bottom: 1rem;
          display: block;
        }
        .state-code {
          font-size: 0.875rem;
          font-weight: 600;
          color: var(--ppt-fg-muted);
          text-transform: uppercase;
          letter-spacing: 0.1em;
          margin: 0 0 0.5rem;
        }
        .state-title {
          font-size: 1.5rem;
          font-weight: 600;
          color: var(--ppt-fg-primary);
          margin: 0 0 0.75rem;
        }
        .state-text {
          font-size: 0.9375rem;
          color: var(--ppt-neutral-600);
          margin: 0 0 1.5rem;
          line-height: 1.5;
        }
        .state-actions {
          display: flex;
          flex-direction: column;
          align-items: stretch;
          gap: 0.5rem;
        }
      `}</style>
    </main>
  );
}
