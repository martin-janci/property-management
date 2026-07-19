/**
 * Manager dashboard page with action-first UX pattern.
 * Renders sections through the resolved layout system (spec §4: never gate on
 * layout fetch — falls back to DEFAULT_DASHBOARD_LAYOUT on error or while
 * loading).
 *
 * @module features/dashboard/pages/ManagerDashboardPage
 */

import { useTranslation } from 'react-i18next';
import { useResolvedLayout } from '@ppt/api-client';
import { LayoutRenderer } from '../../layout/LayoutRenderer';
import { dashboardRegistry, DEFAULT_DASHBOARD_LAYOUT } from '../../layout/registry';
import './ManagerDashboardPage.css';

export function ManagerDashboardPage() {
  const { t } = useTranslation();
  const { data: layout } = useResolvedLayout('ppt/dashboard');

  return (
    <div className="dashboard-page">
      <header className="dashboard-page__header">
        <h1 className="dashboard-page__title">{t('dashboard.managerDashboard')}</h1>
        <p className="dashboard-page__subtitle">{t('dashboard.managerWelcome')}</p>
      </header>

      <LayoutRenderer layout={layout ?? DEFAULT_DASHBOARD_LAYOUT} registry={dashboardRegistry} />
    </div>
  );
}
