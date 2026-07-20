/**
 * Manager dashboard page with action-first UX pattern.
 * Renders sections through the resolved layout system (spec §4: never gate on
 * layout fetch — falls back to DEFAULT_DASHBOARD_LAYOUT on error or while
 * loading).
 *
 * @module features/dashboard/pages/ManagerDashboardPage
 */

import { type ResolvedScreen, useResolvedLayout } from '@ppt/api-client';
import { useTranslation } from 'react-i18next';
import { Link } from 'react-router-dom';
import { useAuth } from '../../../contexts/AuthContext';
import { LayoutRenderer } from '../../layout/LayoutRenderer';
import { DEFAULT_DASHBOARD_LAYOUT, dashboardRegistry } from '../../layout/registry';
import { usePreviewLayout } from '../../layout/usePreviewLayout';
import './ManagerDashboardPage.css';

const CUSTOMIZE_ROLES = ['org_admin', 'super_admin'] as const;

export function ManagerDashboardPage() {
  const { t } = useTranslation();
  const { data: layout } = useResolvedLayout('ppt/dashboard');
  const { user } = useAuth();
  const { previewLayout, inPreview, sendSectionClick } = usePreviewLayout('ppt/dashboard');

  const canCustomize =
    user?.role != null && (CUSTOMIZE_ROLES as readonly string[]).includes(user.role);

  return (
    <div className="dashboard-page">
      <header className="dashboard-page__header">
        <h1 className="dashboard-page__title">{t('dashboard.managerDashboard')}</h1>
        <p className="dashboard-page__subtitle">{t('dashboard.managerWelcome')}</p>
        {canCustomize && (
          <Link to="/dashboard/customize" className="dashboard-page__customize-link">
            {t('layout.customize.title')}
          </Link>
        )}
      </header>

      <LayoutRenderer
        layout={(previewLayout as ResolvedScreen | null) ?? layout ?? DEFAULT_DASHBOARD_LAYOUT}
        registry={dashboardRegistry}
        onSectionClick={inPreview ? sendSectionClick : undefined}
      />
    </div>
  );
}
