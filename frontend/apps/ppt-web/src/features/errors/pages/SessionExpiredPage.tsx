/**
 * SessionExpiredPage.
 *
 * Shown when the auth context detects an expired session and the refresh
 * token is no longer valid. Saves the current URL so login can return the
 * user to where they were.
 */

import { useNavigate } from 'react-router-dom';
import { StateView } from '../components/StateView';

const RETURN_URL_KEY = 'ppt_return_url';

export function SessionExpiredPage() {
  const navigate = useNavigate();

  const handleSignIn = () => {
    try {
      const here = `${window.location.pathname}${window.location.search}`;
      if (here && here !== '/login') {
        sessionStorage.setItem(RETURN_URL_KEY, here);
      }
    } catch {
      // sessionStorage unavailable; proceed without a return URL.
    }
    navigate('/login', { replace: true });
  };

  return (
    <StateView
      icon="⏳"
      title="Your session has expired"
      description="Sign in again to continue. We'll bring you back to where you were."
    >
      <button
        type="button"
        className="state-action state-action--primary"
        onClick={handleSignIn}
      >
        Sign in
      </button>
    </StateView>
  );
}

SessionExpiredPage.displayName = 'SessionExpiredPage';
