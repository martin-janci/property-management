/**
 * ForbiddenPage (403).
 *
 * Shown when the user is signed in but lacks the role required for a route.
 */

import { Link, useNavigate } from 'react-router-dom';
import { StateView } from '../components/StateView';

export function ForbiddenPage() {
  const navigate = useNavigate();
  return (
    <StateView
      icon="🔒"
      code="403"
      title="Access denied"
      description="You don't have permission to view this page. Contact your manager if you think this is a mistake."
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

ForbiddenPage.displayName = 'ForbiddenPage';
