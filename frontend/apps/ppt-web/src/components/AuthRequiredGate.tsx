/**
 * Inline auth-required gate.
 *
 * Used by routes that require an authenticated user with an active
 * `organizationId`. Centralises a JSX block that was repeated inline
 * across DisputesPageRoute, OutagesPageRoute, FaultsPageRoute and
 * AnnouncementsPageRoute (4 copies of the same eight lines, all using
 * the same translation keys).
 *
 * For routes that need full SPA-style auth + redirect, use
 * `<ProtectedRoute>` instead.
 */

import { useTranslation } from 'react-i18next';
import { Link } from 'react-router-dom';

export function AuthRequiredGate() {
  const { t } = useTranslation();
  return (
    <div className="error-page" role="alert">
      <h1>{t('errors.authenticationRequired')}</h1>
      <p>{t('errors.missingOrgContext')}</p>
      <Link to="/login">{t('auth.signIn')}</Link>
    </div>
  );
}
