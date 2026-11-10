/**
 * NotFoundPage (404).
 *
 * Catch-all route for unrecognised URLs.
 */

import { useTranslation } from 'react-i18next';
import { Link, useNavigate } from 'react-router-dom';
import { StateView } from '../components/StateView';

export function NotFoundPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  return (
    <StateView
      icon="🧭"
      code="404"
      title={t('errors.pages.notFound.title')}
      description={t('errors.pages.notFound.description')}
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

NotFoundPage.displayName = 'NotFoundPage';
