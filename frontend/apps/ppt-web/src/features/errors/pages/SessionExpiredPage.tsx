/**
 * SessionExpiredPage.
 *
 * Shown when the auth context detects an expired session and the refresh
 * token is no longer valid. Saves the current URL so login can return the
 * user to where they were.
 */

import { setReturnUrl } from '@ppt/shared';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { StateView } from '../components/StateView';

export function SessionExpiredPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();

  const handleSignIn = () => {
    const here = `${window.location.pathname}${window.location.search}`;
    if (here && here !== '/login') {
      setReturnUrl(here);
    }
    navigate('/login', { replace: true });
  };

  return (
    <StateView
      icon="⏳"
      title={t('errors.pages.sessionExpired.title')}
      description={t('errors.pages.sessionExpired.description')}
    >
      <button type="button" className="state-action state-action--primary" onClick={handleSignIn}>
        {t('errors.pages.sessionExpired.signIn')}
      </button>
    </StateView>
  );
}

SessionExpiredPage.displayName = 'SessionExpiredPage';
