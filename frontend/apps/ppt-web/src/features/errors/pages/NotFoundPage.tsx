/**
 * NotFoundPage (404).
 *
 * Catch-all route for unrecognised URLs.
 */

import { Link, useNavigate } from 'react-router-dom';
import { StateView } from '../components/StateView';

export function NotFoundPage() {
  const navigate = useNavigate();
  return (
    <StateView
      icon="🧭"
      code="404"
      title="Page not found"
      description="The page you're looking for doesn't exist or has moved."
    >
      <button
        type="button"
        className="state-action state-action--primary"
        onClick={() => navigate(-1)}
      >
        Go back
      </button>
      <Link to="/" className="state-action state-action--secondary">
        Home
      </Link>
    </StateView>
  );
}

NotFoundPage.displayName = 'NotFoundPage';
