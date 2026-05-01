/**
 * Page Loading Component
 * Epic 130: Performance Optimization
 *
 * Displays a loading spinner while lazy-loaded pages are being fetched.
 */

import { useTranslation } from 'react-i18next';
import './PageLoading.css';

export function PageLoading() {
  const { t } = useTranslation();

  return (
    <div
      className="page-loading"
      role="status"
      aria-live="polite"
      aria-label={t('common.loading')}
    >
      <div className="page-loading__inner">
        <div className="page-loading__spinner" aria-hidden="true" />
        <span className="page-loading__label">{t('common.loading')}</span>
      </div>
    </div>
  );
}
