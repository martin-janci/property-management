/**
 * ForbiddenPage (403).
 *
 * Shown when the user is signed in but lacks the role required for a route.
 */

import { useTranslation } from 'react-i18next';
import { Link, useNavigate } from 'react-router-dom';
import { StateView } from '../components/StateView';

export function ForbiddenPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  return (
    <StateView
      icon="🔒"
      code="403"
      title={t('errors.pages.forbidden.title')}
      description={t('errors.pages.forbidden.description')}
    >
      <button
        type="button"
        className="state-action state-action--primary"
        onClick={() => navigate(-1)}
      >
        {t('errors.pages.goBack')}
      </button>
      <Link to="/" className="state-action state-action--secondary">
        {t('nav.home')}
      </Link>
    </StateView>
  );
}

ForbiddenPage.displayName = 'ForbiddenPage';
