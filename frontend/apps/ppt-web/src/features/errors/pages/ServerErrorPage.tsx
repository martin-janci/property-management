/**
 * ServerErrorPage (500).
 *
 * Generic fallback for unexpected runtime errors. Pair with the existing
 * ErrorBoundary so it can render this page on uncaught failures.
 */

import { Link } from 'react-router-dom';
import { StateView } from '../components/StateView';

export interface ServerErrorPageProps {
  /** Optional callback to retry the failing operation. */
  onRetry?: () => void;
  /** Optional details visible to the user (use sparingly). */
  details?: string;
}

export function ServerErrorPage({ onRetry, details }: ServerErrorPageProps) {
  return (
    <StateView
      icon="🛠️"
      code="500"
      title="Something went wrong"
      description={details ?? 'An unexpected error occurred. Please try again in a moment.'}
    >
      {onRetry && (
        <button type="button" className="state-action state-action--primary" onClick={onRetry}>
          Try again
        </button>
      )}
      <Link to="/" className="state-action state-action--secondary">
        Back to dashboard
      </Link>
    </StateView>
  );
}

ServerErrorPage.displayName = 'ServerErrorPage';
