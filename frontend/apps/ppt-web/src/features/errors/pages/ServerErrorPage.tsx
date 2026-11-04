/**
 * ServerErrorPage (500).
 *
 * Generic fallback for unexpected runtime errors. Pair with the existing
 * ErrorBoundary so it can render this page on uncaught failures.
 */

import { useTranslation } from 'react-i18next';
import { Link } from 'react-router-dom';
import { StateView } from '../components/StateView';

export interface ServerErrorPageProps {
  /** Optional callback to retry the failing operation. */
  onRetry?: () => void;
  /** Optional details visible to the user (use sparingly). */
  details?: string;
}

export function ServerErrorPage({ onRetry, details }: ServerErrorPageProps) {
  const { t } = useTranslation();
  return (
    <StateView
      icon="🛠️"
      code="500"
      title={t('errors.somethingWentWrong')}
      description={details ?? t('errors.pages.serverError.description')}
    >
      {onRetry && (
        <button type="button" className="state-action state-action--primary" onClick={onRetry}>
          {t('errors.pages.tryAgain')}
        </button>
      )}
      <Link to="/" className="state-action state-action--secondary">
        {t('errors.pages.backToDashboard')}
      </Link>
    </StateView>
  );
}

ServerErrorPage.displayName = 'ServerErrorPage';
