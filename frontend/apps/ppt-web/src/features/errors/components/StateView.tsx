/**
 * StateView — shared layout for full-page error / empty states.
 *
 * Used by NotFound, Forbidden, ServerError and SessionExpired screens.
 * Children render any custom action buttons or supporting content.
 */

import type { ReactNode } from 'react';
import '../styles/errorPages.css';

export interface StateViewProps {
  /** Optional emoji or single-character icon shown above the title. */
  icon?: string;
  /** Status code label (e.g. "404"). Optional. */
  code?: string;
  /** Headline. */
  title: string;
  /** Optional supporting copy below the title. */
  description?: string;
  /** Action buttons / links (rendered in a vertical stack). */
  children?: ReactNode;
}

export function StateView({ icon, code, title, description, children }: StateViewProps) {
  return (
    <div className="state-page" role="alert" aria-live="polite">
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
    </div>
  );
}

StateView.displayName = 'StateView';
